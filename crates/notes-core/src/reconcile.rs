//! Reconciliation and identity correlation — `ARCHITECTURE.md` §8 and §9.
//!
//! **`stat`, then hash. Everything else is a hint.** A watcher event, a window
//! regaining focus, a tab switch and a poll timer all arrive here as the same
//! thing: *these paths may have moved, go and look*. Nothing in this module
//! believes an event; it looks at the disk and compares against the registry,
//! and size-and-mtime alone never conclude anything (scope §12).
//!
//! Two rules run through it:
//!
//! - **A dirty buffer is never overwritten and never silently replaced.** An
//!   external change to a note the user is editing suspends autosave and raises
//!   a conflict; the core cannot write the draft itself, because the frontend
//!   holds the buffer (`docs/DECISIONS-0.1a.md` D-11), so the event says so and
//!   the frontend calls `draft_write`.
//! - **Ambiguity means a new identity.** §9: two candidates on either side of a
//!   rename, and the file gets a new `NoteId`. Re-identifying a note is cheap;
//!   attaching one to the wrong history is not.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use notes_fs::FileSystem;
use notes_model::{ContentHash, CoreError, EntryKind, NoteId, RelPath};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// At most this many files are hashed per reconciliation tick; the rest are
/// queued for the next one (`ARCHITECTURE.md` §8). **The interface never waits
/// on reconciliation** to open or edit a note, so falling behind is allowed and
/// blocking is not.
const HASH_BUDGET: usize = 50;

/// How long a self-write expectation stays armed.
///
/// `ARCHITECTURE.md` §8: consumed on its first match and expired after two
/// seconds, "so an external write that lands right after ours is seen, not
/// swallowed".
const EXPECTATION_TTL: Duration = Duration::from_secs(2);

/// What happened to a path, as the frontend needs to hear it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "event", rename_all = "snake_case")]
#[ts(export)]
pub enum CoreEvent {
    /// The disk changed and no open buffer is at risk. The frontend reloads and
    /// keeps the cursor.
    FsChanged {
        path: RelPath,
        kind: ChangeKind,
        note_id: Option<NoteId>,
    },
    /// The disk changed under a **dirty** buffer. Autosave is suspended for
    /// this note; the frontend writes the draft and offers the comparison.
    NoteConflict {
        note_id: NoteId,
        kind: ConflictKind,
    },
    /// A note was renamed or moved **outside** the application and identity
    /// correlation reconnected it (§9). The tab follows; it does not reset.
    NoteMoved {
        note_id: NoteId,
        from: RelPath,
        to: RelPath,
    },
    /// The root is not reachable. Never inferred from a missing file — only
    /// from the root itself.
    WorkspaceUnavailable {
        root: String,
        reason: notes_model::UnavailableReason,
    },
    WorkspaceAvailable,
    /// The watch could not be established or was lost. The UI says so, and says
    /// what to do about it when there is something.
    WatchDegraded {
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ChangeKind {
    Created,
    Modified,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ConflictKind {
    Modified,
    Removed,
}

/// One tick's worth of reconciliation.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Reconciled {
    pub events: Vec<CoreEvent>,
    /// Paths deferred by the hash budget. A non-zero value means another tick
    /// is owed, and the caller should schedule one rather than wait.
    #[ts(type = "number")]
    pub queued: usize,
}

/// A write this application is about to make, so its own event can be dropped.
#[derive(Debug, Clone)]
pub(crate) struct Expectation {
    pub path: RelPath,
    pub hash: ContentHash,
    pub armed_at: Instant,
}

/// The mutable state reconciliation keeps between ticks.
///
/// Behind a `Mutex` so an expectation can be armed **before** the write it
/// describes, from a `&self` path deep inside the write protocol. Arming after
/// the fact would leave a window in which our own event is news, and the window
/// is exactly as long as the write.
#[derive(Default)]
pub(crate) struct Recon {
    pub expectations: Vec<Expectation>,
    pub queued: BTreeSet<RelPath>,
}

impl Recon {
    pub fn arm(&mut self, path: &RelPath, hash: ContentHash) {
        let now = Instant::now();
        self.expectations
            .retain(|e| now.duration_since(e.armed_at) < EXPECTATION_TTL);
        self.expectations.push(Expectation {
            path: path.clone(),
            hash,
            armed_at: now,
        });
    }

    /// True when this exact `(path, hash)` was one of ours. **Consumed on its
    /// first match**, so a second write of identical bytes by someone else is
    /// still seen.
    pub(crate) fn consume(&mut self, path: &RelPath, hash: &ContentHash) -> bool {
        let now = Instant::now();
        self.expectations
            .retain(|e| now.duration_since(e.armed_at) < EXPECTATION_TTL);
        match self
            .expectations
            .iter()
            .position(|e| &e.path == path && &e.hash == hash)
        {
            Some(i) => {
                self.expectations.remove(i);
                true
            }
            None => false,
        }
    }
}

impl super::WorkspaceService {
    /// Look at the disk and say what changed.
    ///
    /// `hints` are paths worth looking at — from the watcher, or empty for a
    /// full scan on focus. `dirty` is which notes have unsaved edits, because
    /// **the core does not hold buffers** and cannot know.
    pub fn reconcile(
        &mut self,
        hints: &BTreeSet<RelPath>,
        dirty: &[NoteId],
    ) -> super::Result<Reconciled> {
        let full = hints.is_empty() || hints.iter().any(|p| p.is_root());
        let dirty: BTreeSet<NoteId> = dirty.iter().copied().collect();
        let mut events = Vec::new();

        // The root first: a workspace that has gone is not a thousand removed
        // notes, and §12 forbids inferring a deletion from an unreachable root.
        {
            let open = self.open()?;
            if !open.fs.root().is_dir() {
                return Ok(Reconciled {
                    events: vec![CoreEvent::WorkspaceUnavailable {
                        root: open.registry.root.clone(),
                        reason: notes_model::UnavailableReason::RootMissing,
                    }],
                    queued: 0,
                });
            }
        }

        // What to look at this tick: anything queued by the last one, plus the
        // hints, or the whole tree.
        let mut candidates: BTreeSet<RelPath> =
            std::mem::take(&mut *self.open_mut()?.recon.lock().expect("recon")).queued;
        if full {
            candidates.extend(self.walk()?);
            candidates.extend(self.open()?.registry.notes.values().map(|r| r.path.clone()));
        } else {
            candidates.extend(hints.iter().cloned());
        }

        let mut budget = HASH_BUDGET;
        let mut vanished: Vec<(NoteId, RelPath)> = Vec::new();

        for path in candidates {
            if path.is_root() {
                continue;
            }
            let known = self
                .open()?
                .registry
                .find_by_path(&path)
                .map(|(id, r)| (id, r.base_rev(), r.native_id.clone()));

            let stat = self.open()?.fs.stat(&path);
            match (known, stat) {
                // Known, and still there.
                (Some((id, base, _)), Ok(stat)) => {
                    if stat.kind != EntryKind::File {
                        continue;
                    }
                    if base.cheap_match(&stat) {
                        continue; // size and mtime agree: nothing to hash
                    }
                    if budget == 0 {
                        self.open()?.queue(path);
                        continue;
                    }
                    budget -= 1;
                    let bytes = match self.open()?.fs.read(&path) {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    let hash = notes_fs::hash(&bytes);

                    // Our own write, landing back as an event.
                    if self.open()?.consume_self_write(&path, &hash) {
                        self.touch(id, &path)?;
                        continue;
                    }
                    if hash == base.hash {
                        // Touch-only: mtime moved, content did not. The registry
                        // learns the new mtime so the next tick is cheap again.
                        self.touch(id, &path)?;
                        continue;
                    }

                    if dirty.contains(&id) {
                        self.open_mut()?.suspended.insert(id);
                        events.push(CoreEvent::NoteConflict {
                            note_id: id,
                            kind: ConflictKind::Modified,
                        });
                    } else {
                        self.touch(id, &path)?;
                        events.push(CoreEvent::FsChanged {
                            path: path.clone(),
                            kind: ChangeKind::Modified,
                            note_id: Some(id),
                        });
                    }
                }

                // Known, and gone.
                (Some((id, _, _)), Err(_)) => vanished.push((id, path)),

                // Not known, and there.
                //
                // **Only on a hinted tick.** The registry is populated when a
                // note is *opened* (`docs/DECISIONS-0.1a.md` D-09), so on a full
                // scan every note the user has never opened is "not known" —
                // reporting them would announce the whole workspace as created
                // on every window focus, which is worse than saying nothing. A
                // hint means *something happened here, just now*, and that is
                // the only circumstance in which "created" is information.
                //
                // Nothing is lost on a full scan: a scan re-lists the tree
                // anyway, and a file that appeared as half of a rename is found
                // by correlation below, which walks for it.
                (None, Ok(stat)) => {
                    if !full && stat.kind == EntryKind::File && path.is_note() {
                        events.push(CoreEvent::FsChanged {
                            path,
                            kind: ChangeKind::Created,
                            note_id: None,
                        });
                    }
                }

                // Not known and not there: a file that was created and removed
                // between two ticks. Nothing to say.
                (None, Err(_)) => {}
            }
        }

        if !vanished.is_empty() {
            events.extend(self.correlate(vanished, &dirty, &mut budget)?);
        }

        let dir = self.open()?.dir.clone();
        let registry = self.open()?.registry.clone();
        if !self.open()?.read_only {
            super::store_registry(&dir, &registry)?;
        }
        let queued = self.open()?.recon.lock().expect("recon").queued.len();
        Ok(Reconciled { events, queued })
    }

    /// Update a record's `stat` without treating it as a change.
    fn touch(&mut self, id: NoteId, path: &RelPath) -> super::Result<()> {
        let stat = self.open()?.fs.stat(path)?;
        let bytes = self.open()?.fs.read(path).unwrap_or_default();
        let hash = notes_fs::hash(&bytes);
        if let Some(rec) = self.open_mut()?.registry.notes.get_mut(&id) {
            rec.size = stat.size;
            rec.mtime_ns = stat.mtime_ns;
            rec.hash = hash;
            rec.native_id = stat.native_id;
            rec.last_seen = super::now();
        }
        Ok(())
    }

    /// `ARCHITECTURE.md` §9, driven from the notes that vanished.
    ///
    /// The document phrases it as *"appeared := disk paths not in registry"*,
    /// which in this application is nearly every file — the registry is
    /// populated lazily, when a note is opened. Driving it from the vanished
    /// side computes the same answer and costs **nothing at all** when nothing
    /// has vanished, which is every tick but one.
    fn correlate(
        &mut self,
        vanished: Vec<(NoteId, RelPath)>,
        dirty: &BTreeSet<NoteId>,
        budget: &mut usize,
    ) -> super::Result<Vec<CoreEvent>> {
        let mut events = Vec::new();

        // Every path on disk that no record claims. This is the "appeared" set,
        // and walking for it is why correlation is gated on a vanishing.
        let present = self.walk()?;
        let claimed: BTreeSet<RelPath> = self
            .open()?
            .registry
            .notes
            .values()
            .map(|r| r.path.clone())
            .collect();
        let appeared: Vec<RelPath> = present
            .into_iter()
            .filter(|p| !claimed.contains(p) && p.is_note())
            .collect();

        // Rule 1: a unique native id on both sides. Strongest signal there is,
        // and the only one that survives a note being edited as it moves.
        let mut by_native: BTreeMap<String, Vec<RelPath>> = BTreeMap::new();
        let mut sizes: BTreeMap<RelPath, u64> = BTreeMap::new();
        if self.open()?.fs.caps().native_id {
            for p in &appeared {
                if let Ok(s) = self.open()?.fs.stat(p) {
                    sizes.insert(p.clone(), s.size);
                    if let Some(n) = s.native_id {
                        by_native
                            .entry(format!("{n:?}"))
                            .or_default()
                            .push(p.clone());
                    }
                }
            }
        } else {
            for p in &appeared {
                if let Ok(s) = self.open()?.fs.stat(p) {
                    sizes.insert(p.clone(), s.size);
                }
            }
        }

        let mut taken: BTreeSet<RelPath> = BTreeSet::new();
        for (id, from) in vanished {
            let rec = match self.open()?.registry.record(id) {
                Some(r) => (r.native_id.clone(), r.hash.clone(), r.size),
                None => continue,
            };

            let mut hit: Option<RelPath> = None;

            if let Some(native) = &rec.0 {
                if let Some(cands) = by_native.get(&format!("{native:?}")) {
                    let free: Vec<&RelPath> =
                        cands.iter().filter(|p| !taken.contains(*p)).collect();
                    if free.len() == 1 {
                        hit = Some(free[0].clone());
                    }
                }
            }

            // Rule 2: a unique, non-empty content hash. **Zero-byte files are
            // never correlated** — every empty file has the same hash, so the
            // signal is no signal at all.
            if hit.is_none() && rec.2 > 0 {
                let same_size: Vec<RelPath> = appeared
                    .iter()
                    .filter(|p| !taken.contains(*p) && sizes.get(*p) == Some(&rec.2))
                    .cloned()
                    .collect();
                let mut matches = Vec::new();
                for p in same_size {
                    if *budget == 0 {
                        break;
                    }
                    *budget -= 1;
                    if let Ok(b) = self.open()?.fs.read(&p) {
                        if notes_fs::hash(&b) == rec.1 {
                            matches.push(p);
                        }
                    }
                }
                if matches.len() == 1 {
                    hit = Some(matches.remove(0));
                }
            }

            match hit {
                // Rule 3, by omission: no unique match is a new identity. The
                // old record is dropped; the file will get an id of its own
                // when it is opened.
                None => {
                    if dirty.contains(&id) {
                        // "Removed, note open, buffer dirty → keep buffer;
                        // never recreate the path" (§8). The record stays so the
                        // buffer still has an identity to be resolved against.
                        self.open_mut()?.suspended.insert(id);
                        events.push(CoreEvent::NoteConflict {
                            note_id: id,
                            kind: ConflictKind::Removed,
                        });
                    } else {
                        self.open_mut()?.registry.notes.remove(&id);
                        events.push(CoreEvent::FsChanged {
                            path: from,
                            kind: ChangeKind::Removed,
                            note_id: Some(id),
                        });
                    }
                }
                Some(to) => {
                    taken.insert(to.clone());
                    if let Some(rec) = self.open_mut()?.registry.notes.get_mut(&id) {
                        rec.path = to.clone();
                        rec.last_seen = super::now();
                    }
                    self.touch(id, &to)?;
                    events.push(CoreEvent::NoteMoved {
                        note_id: id,
                        from,
                        to,
                    });
                }
            }
        }
        Ok(events)
    }

    /// Every visible file in the workspace, one directory at a time.
    ///
    /// Uses the same ignore rules as the sidebar, so a change inside `.git/`
    /// never reaches reconciliation.
    fn walk(&self) -> super::Result<BTreeSet<RelPath>> {
        let mut out = BTreeSet::new();
        let mut stack = vec![RelPath::root()];
        // A workspace is a folder the user chose, and a pathological tree is
        // their business — but an unbounded walk inside a reconciliation tick
        // is not, so the depth is capped and the cap is generous.
        let mut visited = 0usize;
        while let Some(dir) = stack.pop() {
            visited += 1;
            if visited > 20_000 {
                break;
            }
            let Ok(entries) = self.list_dir(&dir) else {
                continue;
            };
            for e in entries {
                match e.kind {
                    EntryKind::Dir => stack.push(e.path),
                    EntryKind::File => {
                        out.insert(e.path);
                    }
                    // Symlinks are not traversed anywhere in this application
                    // (`ARCHITECTURE.md` §18.7).
                    _ => {}
                }
            }
        }
        Ok(out)
    }

    /// One watcher-driven tick: take what the watcher has seen and reconcile
    /// exactly those paths.
    ///
    /// Returns an empty result immediately when nothing has been seen and
    /// nothing is queued, so a poll loop costs a channel read.
    pub fn tick(&mut self, dirty: &[NoteId]) -> super::Result<Reconciled> {
        // A file that appeared or vanished outside the app changes the tree
        // as surely as one we renamed ourselves.
        self.invalidate_paths();
        let paths = self.watched_paths();
        let queued = self
            .open()?
            .recon
            .lock()
            .map(|r| !r.queued.is_empty())
            .unwrap_or(false);
        if paths.is_empty() && !queued {
            return Ok(Reconciled {
                events: Vec::new(),
                queued: 0,
            });
        }
        // A non-empty set is reconciled as hints; a queue with no new hints
        // still has to drain, and `reconcile` picks it up either way.
        let hints = if paths.is_empty() {
            // An empty hint set means "full scan", which is not what a drained
            // queue asks for — name the queued paths explicitly.
            self.open()?
                .recon
                .lock()
                .map(|r| r.queued.clone())
                .unwrap_or_default()
        } else {
            paths
        };
        self.reconcile(&hints, dirty)
    }

    /// A full scan — what a window regaining focus, a tab switch or a manual
    /// refresh triggers (`ARCHITECTURE.md` §8).
    pub fn reconcile_all(&mut self, dirty: &[NoteId]) -> super::Result<Reconciled> {
        // A file that appeared or vanished outside the app changes the tree
        // as surely as one we renamed ourselves.
        self.invalidate_paths();
        self.reconcile(&BTreeSet::new(), dirty)
    }

    /// Start (or restart) watching this workspace. Returns the reason when the
    /// platform cannot, so the caller can poll and the UI can say why.
    pub fn start_watch(&mut self) -> super::Result<Option<String>> {
        let watch = self.open()?.fs.watch();
        let reason = watch.degraded.as_ref().map(|d| match d {
            notes_fs::Degraded::Unsupported(m) | notes_fs::Degraded::WatchLimit(m) => m.clone(),
        });
        self.open_mut()?.watch = Some(watch);
        Ok(reason)
    }

    /// Everything the watcher has seen since the last call. Empty when there is
    /// no watch, which is exactly when the caller should be polling instead.
    pub fn watched_paths(&self) -> BTreeSet<RelPath> {
        self.open
            .as_ref()
            .and_then(|o| o.watch.as_ref())
            .map(|w| w.drain())
            .unwrap_or_default()
    }

    /// Block until the watcher reports something, for a caller with nothing
    /// else to do — a test, and the shell's watch thread.
    pub fn wait_for_change(&self, timeout: Duration) -> BTreeSet<RelPath> {
        self.open
            .as_ref()
            .and_then(|o| o.watch.as_ref())
            .map(|w| w.wait(timeout))
            .unwrap_or_default()
    }
}

impl From<CoreError> for CoreEvent {
    fn from(e: CoreError) -> Self {
        CoreEvent::WatchDegraded {
            reason: e.to_string(),
        }
    }
}
