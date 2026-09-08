//! `WorkspaceService` — the only public API of this application.
//!
//! `src-tauri` and, from 0.3, `notes-mcp` are clients of it and hold no policy
//! of their own (ADR-003). That is what lets the MCP server work with the window
//! closed, and what lets every rule below be tested with `cargo test` and no
//! Tauri (`ARCHITECTURE.md` §14).

pub mod conflicts;
pub mod drafts;
pub mod ignore;
pub mod index;
mod lock;
pub mod paths;
pub mod preview;
pub mod reconcile;
pub mod registry;
pub mod search;
pub mod settings;
mod state;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use notes_fs::{FileSystem, LocalFs, WriteOutcome};
use notes_model::{
    BaseRev, Caps, CoreError, Entry, NoteId, ReadOnlyReason, RelPath, TextProfile,
    UnavailableReason, WorkspaceId,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use conflicts::{ConflictChoice, ConflictSnapshot, Conflicts, Side};
pub use drafts::{DraftInfo, DraftReason};
pub use notes_fs::DeleteOutcome;
pub use preview::{Asset, Document, Rendered};
pub use reconcile::{ChangeKind, ConflictKind, CoreEvent, Reconciled};
pub use registry::{Registry, WorkspaceEntry, WorkspacesIndex};
pub use settings::{Session, Settings, Tab};

use state::Loaded;

type Result<T> = std::result::Result<T, CoreError>;

/// What the watcher is doing, for the status bar.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WatchStatus {
    pub watching: bool,
    /// True while the walk that installs the watches is still running.
    pub walking: bool,
    #[ts(type = "number")]
    pub dirs: usize,
    #[ts(type = "number")]
    pub unreadable: usize,
    #[ts(type = "number")]
    pub over_limit: usize,
    /// Set only when **nothing** is watched. A directory that could not be
    /// watched is counted above, not here.
    pub degraded: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkspaceInfo {
    pub id: WorkspaceId,
    pub root: String,
    pub display_name: String,
    pub caps: Caps,
    pub case_insensitive: bool,
    /// True when this workspace's state was written by a newer build. Nothing
    /// is overwritten in that case; the user is told rather than losing it.
    pub read_only: bool,
    /// The schema that newer build wrote, when there is one. Carried so the
    /// message can say *how far* ahead the state is instead of only that it is.
    pub state_schema_ahead: Option<u32>,
    pub restored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OpenedNote {
    pub note_id: NoteId,
    pub path: RelPath,
    pub text: String,
    pub profile: TextProfile,
    pub base_rev: BaseRev,
    pub read_only: Option<ReadOnlyReason>,
    pub draft: Option<DraftInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "result", rename_all = "snake_case")]
#[ts(export)]
pub enum SaveResult {
    Saved {
        base_rev: BaseRev,
        #[ts(type = "number")]
        buffer_version: u64,
        unchanged: bool,
    },
    /// The disk moved under the buffer. Autosave is suspended for this note, a
    /// draft holds the buffer, and nothing was written.
    Conflict {
        disk_rev: BaseRev,
        #[ts(type = "number")]
        buffer_version: u64,
    },
    WriteFailed {
        kind: notes_model::IoKind,
        #[ts(type = "number")]
        buffer_version: u64,
    },
}

/// Whether a delete can be undone from the operating system's bin.
///
/// Two different events, and scope §7.7 requires the application to say which:
/// a silent fallback would tell a user their file is in the bin when it is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DeleteKind {
    Trashed,
    Permanent,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Deleted {
    pub path: RelPath,
    pub outcome: DeleteKind,
    /// The notes that went with it, so their tabs can be closed. Their drafts
    /// are **not** removed: a note deleted with unsaved edits is precisely the
    /// case where the draft is the only copy.
    pub note_ids: Vec<NoteId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DraftChoice {
    Restore,
    Discard,
}

impl Open {
    /// Arm a self-write expectation from a `&self` path — the write protocol
    /// holds an immutable borrow while it works, and the expectation has to be
    /// armed before the bytes land.
    fn arm(&self, path: &RelPath, hash: notes_model::ContentHash) {
        if let Ok(mut r) = self.recon.lock() {
            r.arm(path, hash);
        }
    }
    fn consume_self_write(&self, path: &RelPath, hash: &notes_model::ContentHash) -> bool {
        self.recon
            .lock()
            .map(|mut r| r.consume(path, hash))
            .unwrap_or(false)
    }
    fn queue(&self, path: RelPath) {
        if let Ok(mut r) = self.recon.lock() {
            r.queued.insert(path);
        }
    }
}

/// The quick-open index and whether it still describes the tree.
#[derive(Default)]
struct PathState {
    index: Option<index::PathIndex>,
    stale: bool,
}

struct Open {
    id: WorkspaceId,
    fs: LocalFs,
    dir: PathBuf,
    registry: Registry,
    read_only: bool,
    extra_ignore: Vec<String>,
    suspended: BTreeSet<NoteId>,
    /// What reconciliation remembers between ticks: the self-write
    /// expectations, and the paths the hash budget deferred.
    recon: std::sync::Mutex<reconcile::Recon>,
    /// The running watch, when the backend has one.
    watch: Option<notes_fs::Watch>,
    /// The quick-open path list, built on a thread of its own.
    ///
    /// `stale` is set by anything that changes the shape of the tree; the
    /// rebuild happens on the next `quick_open` rather than immediately, so a
    /// reconciliation tick cannot put the walk on a treadmill.
    paths: std::sync::Mutex<PathState>,
}

pub struct WorkspaceService {
    data_dir: PathBuf,
    open: Option<Open>,
    settings: Settings,
    /// One search at a time. Starting another cancels the first, and dropping a
    /// `Search` stops its walk — otherwise every keystroke in the search box
    /// would leave a thread scanning the corpus for nobody.
    search: Option<search::Search>,
}

impl WorkspaceService {
    pub fn new() -> Result<Self> {
        Self::with_data_dir(paths::data_dir()?)
    }

    pub fn with_data_dir(data_dir: impl Into<PathBuf>) -> Result<Self> {
        let data_dir = data_dir.into();
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| CoreError::io("mkdir", data_dir.display(), &e))?;
        let settings = match state::load::<Settings>(&paths::global_settings(&data_dir))? {
            Loaded::Ok(s) => s,
            // A settings file from the future is not fatal: defaults are safe,
            // and `webkit_dmabuf_workaround` degrading to `auto` is the point.
            Loaded::Fresh | Loaded::TooNew { .. } => Settings::default(),
        };
        Ok(Self {
            data_dir,
            open: None,
            settings,
            search: None,
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// The open workspace's id, for callers that need to address its app-data
    /// directory — `notes-mcp` from 0.3, and the tests today.
    pub fn workspace_id(&self) -> Option<WorkspaceId> {
        self.open.as_ref().map(|o| o.id)
    }

    // ---- workspace ------------------------------------------------------

    /// Adopt a folder. **Creates nothing inside it** (scope §2.3).
    pub fn open_workspace(&mut self, root: &Path) -> Result<WorkspaceInfo> {
        self.adopt(root, false)
    }

    /// Create `parent/name` and adopt it. The only path on which this
    /// application makes a directory the user did not already have.
    pub fn create_workspace(&mut self, parent: &Path, name: &str) -> Result<WorkspaceInfo> {
        if name.is_empty() || name.contains('/') || name.contains('\\') {
            return Err(CoreError::InvalidPath {
                path: name.to_string(),
                reason: "workspace name must be a single path segment".into(),
            });
        }
        let dir = parent.join(name);
        match std::fs::create_dir(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(CoreError::AlreadyExists {
                    path: dir.display().to_string(),
                })
            }
            Err(e) => return Err(CoreError::io("create_dir", dir.display(), &e)),
        }
        self.adopt(&dir, false)
    }

    /// Re-open the workspace this application last had open.
    ///
    /// `Ok(None)` means there has never been one. A folder that has moved or
    /// become unreadable is an **error**, not `None`: "no workspace" and "your
    /// notes are not where they were" are the two answers a user most needs told
    /// apart.
    pub fn restore_last_workspace(&mut self) -> Result<Option<WorkspaceInfo>> {
        let index = self.index()?;
        let Some(id) = index.last_workspace else {
            return Ok(None);
        };
        let Some(entry) = index.workspaces.iter().find(|w| w.id == id) else {
            return Ok(None);
        };
        let root = PathBuf::from(&entry.root);
        if !root.is_dir() {
            return Err(CoreError::Unavailable {
                root: entry.root.clone(),
                reason: UnavailableReason::RootMissing,
            });
        }
        self.adopt(&root, true).map(Some)
    }

    pub fn recent_workspaces(&self) -> Result<Vec<WorkspaceEntry>> {
        let mut v = self.index()?.workspaces;
        v.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
        Ok(v)
    }

    /// Close, refusing while buffers are dirty.
    ///
    /// The core does not hold buffers — the frontend does (`ARCHITECTURE.md`
    /// §5) — so the caller states which notes are dirty. `DirtyBuffers` names
    /// them so the UI can offer to flush rather than just say no.
    pub fn close_workspace(&mut self, dirty: &[NoteId]) -> Result<()> {
        if !dirty.is_empty() {
            return Err(CoreError::DirtyBuffers {
                note_ids: dirty.to_vec(),
                count: dirty.len(),
            });
        }
        if let Some(open) = self.open.take() {
            state::store(&paths::registry_file(&open.dir), &open.registry)?;
        }
        Ok(())
    }

    fn adopt(&mut self, root: &Path, restored: bool) -> Result<WorkspaceInfo> {
        let fs = LocalFs::open(root)?;
        let canonical = fs.root().display().to_string();
        let display_name = fs
            .root()
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| canonical.clone());

        let mut index = self.index()?;
        let id = index
            .by_root(&canonical)
            .map(|w| w.id)
            .unwrap_or_else(WorkspaceId::new);
        let dir = paths::workspace_dir(&self.data_dir, id);

        // Read-only probe, before anything else touches the tree: writing a
        // temporary file here would break scope §2.3 (D-01).
        let entries = fs.list(&RelPath::root())?;
        let case_insensitive =
            notes_fs::probe_case_insensitive(fs.root(), &entries).unwrap_or(true);

        let (registry, ahead) = match state::load::<Registry>(&paths::registry_file(&dir))? {
            Loaded::Ok(mut r) => {
                // The probe is authoritative and corrects in both directions.
                r.case_insensitive = case_insensitive;
                r.root = canonical.clone();
                (r, None)
            }
            Loaded::Fresh => (
                Registry::new(
                    id,
                    &canonical,
                    case_insensitive,
                    fs.stat(&RelPath::root()).ok().and_then(|s| s.native_id),
                ),
                None,
            ),
            Loaded::TooNew { found } => (
                Registry::new(id, &canonical, case_insensitive, None),
                Some(found),
            ),
        };
        let read_only = ahead.is_some();

        index.touch(id, &canonical, &display_name);
        state::store(&paths::workspaces_index(&self.data_dir), &index)?;
        if !read_only {
            state::store(&paths::registry_file(&dir), &registry)?;
        }

        // Prune resolved conflict snapshots on the way in, once per open. It
        // reads directory entries and their timestamps and nothing else, so it
        // costs nothing on a workspace that has never had a conflict.
        if !read_only {
            conflicts::prune(&dir, self.settings.files.conflict_retention_days);
        }

        let extra_ignore = read_portable_ignore(fs.root());
        let caps = fs.caps();
        self.open = Some(Open {
            id,
            fs,
            dir,
            registry,
            read_only,
            extra_ignore,
            suspended: BTreeSet::new(),
            recon: std::sync::Mutex::new(reconcile::Recon::default()),
            watch: None,
            paths: std::sync::Mutex::new(PathState::default()),
        });

        Ok(WorkspaceInfo {
            id,
            root: canonical,
            display_name,
            caps,
            case_insensitive,
            read_only,
            state_schema_ahead: ahead,
            restored,
        })
    }

    fn index(&self) -> Result<WorkspacesIndex> {
        Ok(
            match state::load::<WorkspacesIndex>(&paths::workspaces_index(&self.data_dir))? {
                Loaded::Ok(i) => i,
                Loaded::Fresh | Loaded::TooNew { .. } => WorkspacesIndex::default(),
            },
        )
    }

    fn open(&self) -> Result<&Open> {
        self.open.as_ref().ok_or(CoreError::NoWorkspace)
    }
    fn open_mut(&mut self) -> Result<&mut Open> {
        self.open.as_mut().ok_or(CoreError::NoWorkspace)
    }

    // ---- tree -----------------------------------------------------------

    /// One level, filtered. Never reads a file's contents and never assigns a
    /// `NoteId` — both would break the 0.1a listing criterion (D-09).
    pub fn list_dir(&self, dir: &RelPath) -> Result<Vec<Entry>> {
        let open = self.open()?;
        let show_hidden = open
            .registry
            .settings
            .show_hidden
            .unwrap_or(self.settings.files.show_hidden);
        Ok(open
            .fs
            .list(dir)?
            .into_iter()
            .filter(|e| !ignore::is_hidden(e, show_hidden, &open.extra_ignore))
            .collect())
    }

    // ---- notes ----------------------------------------------------------

    pub fn open_note(&mut self, path: &RelPath) -> Result<OpenedNote> {
        let (stat, bytes) = {
            let open = self.open()?;
            (open.fs.stat(path)?, open.fs.read(path)?)
        };
        let hash = notes_fs::hash(&bytes);
        let (profile, text) = TextProfile::detect(&bytes);
        let read_only = profile.read_only_reason();

        let dir = self.open()?.dir.clone();
        let persist = !self.open()?.read_only;
        let open = self.open_mut()?;
        let note_id = open.registry.observe(path, &stat, hash.clone());
        if persist {
            state::store(&paths::registry_file(&dir), &open.registry)?;
        }

        let draft = drafts::read(&paths::drafts_dir(&dir), note_id)?.map(|d| d.info);
        Ok(OpenedNote {
            note_id,
            path: path.clone(),
            text: text.unwrap_or_default(),
            profile,
            base_rev: BaseRev {
                size: stat.size,
                mtime_ns: stat.mtime_ns,
                hash,
            },
            read_only,
            draft,
        })
    }

    /// The write protocol of `ARCHITECTURE.md` §5, with `base_rev` explicit on
    /// the wire so the app and `notes-mcp` share one contract (scope §9).
    pub fn save_note(
        &mut self,
        note_id: NoteId,
        text: &str,
        buffer_version: u64,
        base_rev: &BaseRev,
    ) -> Result<SaveResult> {
        let (dir, path, profile_bytes) = {
            let open = self.open()?;
            if open.read_only {
                return Err(CoreError::ReadOnly {
                    note_id,
                    reason: ReadOnlyReason::Workspace,
                });
            }
            let rec = open
                .registry
                .record(note_id)
                .ok_or_else(|| CoreError::NotFound {
                    path: note_id.to_string(),
                })?;
            let path = rec.path.clone();
            // Re-detect the profile from disk rather than trusting the caller:
            // the bytes decide what the file's shape is, and the caller only
            // holds text.
            let current = open.fs.read(&path)?;
            let (profile, _) = TextProfile::detect(&current);
            if let Some(reason) = profile.read_only_reason() {
                return Err(CoreError::ReadOnly { note_id, reason });
            }
            (open.dir.clone(), path, profile.encode(text))
        };

        let mut guard = lock::acquire(&paths::lock_file(&dir))?;
        let outcome = guard.with(|| -> Result<SaveResult> {
            let new_hash = notes_fs::hash(&profile_bytes);
            let open = self.open.as_ref().expect("checked above");

            // Step 4: nothing to do. No write, no mtime bump, no `git status`.
            if let Some(rec) = open.registry.record(note_id) {
                if rec.hash == new_hash {
                    if let Ok(disk) = open.fs.stat(&path) {
                        if rec.base_rev().cheap_match(&disk) {
                            return Ok(SaveResult::Saved {
                                base_rev: rec.base_rev(),
                                buffer_version,
                                unchanged: true,
                            });
                        }
                    }
                }
            }

            // Step 5: compare against what the caller read.
            let disk = open.fs.stat(&path)?;
            if !base_rev.cheap_match(&disk) {
                let current = open.fs.read(&path)?;
                let disk_hash = notes_fs::hash(&current);
                if disk_hash == new_hash {
                    // Convergence: the disk already holds exactly this buffer.
                    let rev = BaseRev {
                        size: disk.size,
                        mtime_ns: disk.mtime_ns,
                        hash: disk_hash,
                    };
                    return Ok(SaveResult::Saved {
                        base_rev: rev,
                        buffer_version,
                        unchanged: true,
                    });
                }
                if disk_hash != base_rev.hash {
                    let disk_rev = BaseRev {
                        size: disk.size,
                        mtime_ns: disk.mtime_ns,
                        hash: disk_hash,
                    };
                    return Ok(SaveResult::Conflict {
                        disk_rev,
                        buffer_version,
                    });
                }
                // Touch-only change: mtime moved, content did not.
            }

            // A failed write is a *result*, not an error: the acceptance
            // criterion is "disco cheio / permissão negada → erro visível,
            // buffer recuperável ao reabrir", and propagating `Err` here would
            // skip the draft that makes the buffer recoverable.
            // Armed before the write: the watcher event this causes must not
            // come back as news (`ARCHITECTURE.md` §8).
            open.arm(&path, new_hash.clone());
            let written = match open.fs.write_atomic(&path, &profile_bytes, Some(base_rev)) {
                Ok(w) => w,
                Err(CoreError::Io { kind, .. }) => {
                    return Ok(SaveResult::WriteFailed {
                        kind,
                        buffer_version,
                    })
                }
                Err(e) => return Err(e),
            };
            match written {
                WriteOutcome::Written(stat) => Ok(SaveResult::Saved {
                    base_rev: BaseRev {
                        size: stat.size,
                        mtime_ns: stat.mtime_ns,
                        hash: new_hash,
                    },
                    buffer_version,
                    unchanged: false,
                }),
                WriteOutcome::Diverged(stat) => {
                    let current = open.fs.read(&path).unwrap_or_default();
                    Ok(SaveResult::Conflict {
                        disk_rev: BaseRev {
                            size: stat.size,
                            mtime_ns: stat.mtime_ns,
                            hash: notes_fs::hash(&current),
                        },
                        buffer_version,
                    })
                }
            }
        })??;

        self.settle(
            note_id,
            &path,
            text,
            buffer_version,
            base_rev,
            outcome,
            &dir,
        )
    }

    /// Apply the consequences of a save: registry, draft, suspension.
    #[allow(clippy::too_many_arguments)]
    fn settle(
        &mut self,
        note_id: NoteId,
        path: &RelPath,
        text: &str,
        buffer_version: u64,
        base_rev: &BaseRev,
        outcome: SaveResult,
        dir: &Path,
    ) -> Result<SaveResult> {
        match &outcome {
            SaveResult::Saved {
                base_rev: new_base,
                unchanged,
                ..
            } => {
                let open = self.open_mut()?;
                open.suspended.remove(&note_id);
                if let Some(rec) = open.registry.notes.get_mut(&note_id) {
                    if !*unchanged {
                        rec.rev += 1;
                    }
                    rec.size = new_base.size;
                    rec.mtime_ns = new_base.mtime_ns;
                    rec.hash = new_base.hash.clone();
                    rec.last_seen = now();
                }
                let registry = open.registry.clone();
                state::store(&paths::registry_file(dir), &registry)?;
                // Only now: the buffer is on disk, so the draft is redundant.
                drafts::discard(&paths::drafts_dir(dir), note_id)?;
            }
            SaveResult::Conflict { .. } => {
                self.write_draft_inner(
                    note_id,
                    path,
                    text,
                    buffer_version,
                    base_rev,
                    DraftReason::Conflict,
                    dir,
                )?;
                self.open_mut()?.suspended.insert(note_id);
            }
            SaveResult::WriteFailed { .. } => {
                self.write_draft_inner(
                    note_id,
                    path,
                    text,
                    buffer_version,
                    base_rev,
                    DraftReason::WriteFailed,
                    dir,
                )?;
            }
        }
        Ok(outcome)
    }

    /// Persist a buffer **without writing the note**.
    ///
    /// `ARCHITECTURE.md` §4.2 requires drafts on `stale` and `exit`, and §5 says
    /// that while autosave is suspended the edits keep going to the draft — none
    /// of which the core can do on its own, because the frontend owns the
    /// buffer. This is the command that was missing (`docs/DECISIONS-0.1a.md`
    /// D-11).
    pub fn write_draft(
        &mut self,
        note_id: NoteId,
        text: &str,
        buffer_version: u64,
        base_rev: &BaseRev,
        reason: DraftReason,
    ) -> Result<DraftInfo> {
        let (dir, path) = {
            let open = self.open()?;
            let rec = open
                .registry
                .record(note_id)
                .ok_or_else(|| CoreError::NotFound {
                    path: note_id.to_string(),
                })?;
            (open.dir.clone(), rec.path.clone())
        };
        self.write_draft_inner(note_id, &path, text, buffer_version, base_rev, reason, &dir)
    }

    #[allow(clippy::too_many_arguments)]
    fn write_draft_inner(
        &mut self,
        note_id: NoteId,
        path: &RelPath,
        text: &str,
        buffer_version: u64,
        base_rev: &BaseRev,
        reason: DraftReason,
        dir: &Path,
    ) -> Result<DraftInfo> {
        let info = DraftInfo {
            schema: 1,
            note_id,
            path: path.clone(),
            buffer_version,
            base_rev: base_rev.clone(),
            reason,
            written_at: now(),
        };
        drafts::write(
            &paths::drafts_dir(dir),
            &drafts::Draft {
                info: info.clone(),
                bytes: text.as_bytes().to_vec(),
            },
        )?;
        Ok(info)
    }

    pub fn list_drafts(&self) -> Result<Vec<DraftInfo>> {
        Ok(drafts::list(&paths::drafts_dir(&self.open()?.dir)))
    }

    /// Restore or discard a draft. Discarding is the only way one goes away
    /// other than a confirmed write.
    pub fn resolve_draft(&mut self, note_id: NoteId, choice: DraftChoice) -> Result<OpenedNote> {
        let dir = self.open()?.dir.clone();
        let draft = drafts::read(&paths::drafts_dir(&dir), note_id)?;
        let path = self
            .open()?
            .registry
            .record(note_id)
            .ok_or_else(|| CoreError::NotFound {
                path: note_id.to_string(),
            })?
            .path
            .clone();
        let mut opened = self.open_note(&path)?;
        match choice {
            DraftChoice::Discard => {
                drafts::discard(&paths::drafts_dir(&dir), note_id)?;
                opened.draft = None;
            }
            DraftChoice::Restore => {
                if let Some(d) = draft {
                    opened.text = String::from_utf8_lossy(&d.bytes).into_owned();
                }
            }
        }
        self.open_mut()?.suspended.remove(&note_id);
        Ok(opened)
    }

    pub fn is_suspended(&self, note_id: NoteId) -> bool {
        self.open
            .as_ref()
            .is_some_and(|o| o.suspended.contains(&note_id))
    }

    // ---- entries: rename, move, duplicate, delete ------------------------

    /// Rename in place. **The `NoteId` does not change** — scope §17's 0.1b
    /// criterion is *"rename via app não reseta aba/cursor/id"*, and
    /// `ARCHITECTURE.md` §9 is explicit that a rename the application performs
    /// never enters identity correlation. Renaming a folder carries every note
    /// beneath it, for the same reason.
    pub fn rename_entry(&mut self, path: &RelPath, new_name: &str) -> Result<Entry> {
        // The tree is about to change shape; quick open must not offer a
        // path that is no longer there.
        self.invalidate_paths();
        let parent = path.parent().ok_or_else(|| CoreError::InvalidPath {
            path: path.to_string(),
            reason: "the workspace root cannot be renamed from inside it".into(),
        })?;
        let to = parent.join(new_name)?;
        self.relocate(path, &to, new_name)
    }

    /// Move into another directory, keeping the name and the `NoteId`.
    ///
    /// One root means one volume, so the cross-volume `Unsupported` of
    /// `ARCHITECTURE.md` §7.1 cannot arise here; it is left in the contract for
    /// the mobile adapters of 0.4, where it can.
    pub fn move_entry(&mut self, path: &RelPath, to_dir: &RelPath) -> Result<Entry> {
        // The tree is about to change shape; quick open must not offer a
        // path that is no longer there.
        self.invalidate_paths();
        let name = path.file_name().to_string();
        let to = to_dir.join(&name)?;
        if to == *path {
            return Err(CoreError::AlreadyExists {
                path: to.to_string(),
            });
        }
        // Moving a folder into itself would leave the tree unreachable, and the
        // error a filesystem gives for it says nothing a user can act on.
        if to.as_str().starts_with(&format!("{}/", path.as_str())) {
            return Err(CoreError::InvalidPath {
                path: to.to_string(),
                reason: "a folder cannot be moved inside itself".into(),
            });
        }
        self.relocate(path, &to, &name)
    }

    fn relocate(&mut self, from: &RelPath, to: &RelPath, name: &str) -> Result<Entry> {
        {
            let open = self.open()?;
            if open.read_only {
                return Err(CoreError::Unavailable {
                    root: open.registry.root.clone(),
                    reason: UnavailableReason::PermissionRevoked,
                });
            }
        }
        // The name has to be legal on every platform the workspace might be
        // carried to, and free by the root's own case and normalisation rules —
        // the same two checks `note_create` makes (scope §7.6). A rename that
        // only changes case is not a collision with itself.
        notes_model::portable_name(name).map_err(|rule| CoreError::InvalidPath {
            path: name.to_string(),
            reason: rule.to_string(),
        })?;
        if to != from {
            self.check_collision(to)?;
        }

        let dir = self.open()?.dir.clone();
        let kind = self.open()?.fs.stat(from)?.kind;
        self.open()?.fs.rename(from, to)?;

        let open = self.open_mut()?;
        open.registry.repath(from, to);
        let registry = open.registry.clone();
        store_registry(&dir, &registry)?;

        let stat = self.open()?.fs.stat(to).ok();
        Ok(Entry {
            name: name.to_string(),
            is_note: kind == notes_model::EntryKind::File && to.is_note(),
            size: (kind == notes_model::EntryKind::File)
                .then(|| stat.map(|s| s.size))
                .flatten(),
            kind,
            path: to.clone(),
        })
    }

    /// Copy a note or a folder beside itself, as `nome (copy).md`.
    ///
    /// **A new file is a new note**, so the copy gets its own `NoteId` — it has
    /// none of the original's history, and pretending otherwise would attach two
    /// files to one identity. `create_new` throughout: scope §17's criterion is
    /// *"criar/duplicar nunca sobrescreve destino existente"*, and the numbered
    /// fallback is what makes that keepable rather than a refusal.
    pub fn duplicate_entry(&mut self, path: &RelPath) -> Result<Entry> {
        // The tree is about to change shape; quick open must not offer a
        // path that is no longer there.
        self.invalidate_paths();
        if self.open()?.read_only {
            return Err(CoreError::Unavailable {
                root: self.open()?.registry.root.clone(),
                reason: UnavailableReason::PermissionRevoked,
            });
        }
        let to = self.free_name(path, "copy")?;
        let kind = self.open()?.fs.stat(path)?.kind;
        match kind {
            notes_model::EntryKind::File => {
                let bytes = self.open()?.fs.read(path)?;
                self.open()?.arm(&to, notes_fs::hash(&bytes));
                self.open()?.fs.create_new(&to, &bytes)?;
            }
            notes_model::EntryKind::Dir => self.copy_tree(path, &to)?,
            // A symlink is not followed anywhere else in this application, and
            // copying one would mean deciding whether to copy the link or its
            // target — a decision nothing has asked for.
            other => {
                return Err(CoreError::Unsupported {
                    cap: format!("duplicating a {other:?} entry"),
                })
            }
        }
        let stat = self.open()?.fs.stat(&to).ok();
        Ok(Entry {
            name: to.file_name().to_string(),
            is_note: kind == notes_model::EntryKind::File && to.is_note(),
            size: (kind == notes_model::EntryKind::File)
                .then(|| stat.map(|s| s.size))
                .flatten(),
            kind,
            path: to,
        })
    }

    fn copy_tree(&self, from: &RelPath, to: &RelPath) -> Result<()> {
        let open = self.open()?;
        open.fs.create_dir(to)?;
        for e in open.fs.list(from)? {
            let target = to.join(&e.name)?;
            match e.kind {
                notes_model::EntryKind::Dir => self.copy_tree(&e.path, &target)?,
                notes_model::EntryKind::File => {
                    let bytes = open.fs.read(&e.path)?;
                    open.arm(&target, notes_fs::hash(&bytes));
                    open.fs.create_new(&target, &bytes)?;
                }
                // Skipped rather than refused: one link inside a folder is not a
                // reason to fail a copy the user asked for, and following it
                // would copy something from outside the workspace into it.
                _ => continue,
            }
        }
        Ok(())
    }

    /// `nome (copy).md`, or `nome (copy 2).md` when that is taken.
    ///
    /// **The word is ASCII and the same in every locale**, deliberately: a
    /// filename that depended on the interface language would give the same
    /// folder different names on two machines, and the scope's own
    /// `nome (local).md` set the precedent (`docs/DECISIONS-0.1b.md` D-10).
    fn free_name(&self, path: &RelPath, word: &str) -> Result<RelPath> {
        let parent = path.parent().unwrap_or_else(RelPath::root);
        let name = path.file_name();
        let (stem, ext) = match name.rfind('.') {
            Some(i) if i > 0 => (&name[..i], &name[i..]),
            _ => (name, ""),
        };
        for n in 1..1000 {
            let candidate = if n == 1 {
                format!("{stem} ({word}){ext}")
            } else {
                format!("{stem} ({word} {n}){ext}")
            };
            let rel = parent.join(&candidate)?;
            if self.check_collision(&rel).is_ok() {
                return Ok(rel);
            }
        }
        Err(CoreError::AlreadyExists {
            path: format!("{stem} ({word} …){ext}"),
        })
    }

    /// Delete, and **say which of the two happened**.
    ///
    /// Scope §7.7 forbids a silent fallback from the trash to a permanent
    /// delete: those are different events for the person who did it, and
    /// [`DeleteOutcome`] is what the interface has to show. The notes that go
    /// with it leave the registry — their identity has nothing left to name —
    /// and the ids come back so open tabs can be closed.
    pub fn delete_entry(&mut self, path: &RelPath) -> Result<Deleted> {
        // The tree is about to change shape; quick open must not offer a
        // path that is no longer there.
        self.invalidate_paths();
        if self.open()?.read_only {
            return Err(CoreError::Unavailable {
                root: self.open()?.registry.root.clone(),
                reason: UnavailableReason::PermissionRevoked,
            });
        }
        let dir = self.open()?.dir.clone();
        let outcome = self.open()?.fs.delete(path)?;

        let open = self.open_mut()?;
        let note_ids = open.registry.forget(path);
        for id in &note_ids {
            open.suspended.remove(id);
        }
        let registry = open.registry.clone();
        store_registry(&dir, &registry)?;

        // The drafts stay. A note the user deleted while holding unsaved edits
        // is exactly the case where the only copy of those edits is the draft,
        // and §4.2 removes one on a confirmed write or on an explicit discard —
        // never as a side effect of something else.
        Ok(Deleted {
            path: path.clone(),
            outcome: match outcome {
                notes_fs::DeleteOutcome::Trashed => DeleteKind::Trashed,
                notes_fs::DeleteOutcome::Permanent => DeleteKind::Permanent,
            },
            note_ids,
        })
    }

    // ---- conflicts ------------------------------------------------------

    /// Take a note out of conflict, keeping the version that was not chosen.
    ///
    /// **"Compare" is not here, and that is deliberate** — `ARCHITECTURE.md`
    /// §17.1 settled it: comparing changes nothing on disk and reads two strings
    /// the frontend is already holding, so it is a screen rather than a command.
    /// The three that *do* something are below, and each one writes the version
    /// it is discarding into `conflicts/` **before** it acts (§4.3). Resolving a
    /// conflict is the one moment a user can lose a morning by answering a
    /// dialog quickly.
    ///
    /// `SaveAsCopy` returns the **copy**, opened: the tab the user was typing in
    /// goes on holding their text, which is now a file of its own, and the note
    /// they were editing is left exactly as the other program wrote it.
    pub fn resolve_conflict(
        &mut self,
        note_id: NoteId,
        text: &str,
        base_rev: &BaseRev,
        choice: ConflictChoice,
    ) -> Result<OpenedNote> {
        let (dir, path) = {
            let open = self.open()?;
            if open.read_only {
                return Err(CoreError::ReadOnly {
                    note_id,
                    reason: ReadOnlyReason::Workspace,
                });
            }
            let rec = open
                .registry
                .record(note_id)
                .ok_or_else(|| CoreError::NotFound {
                    path: note_id.to_string(),
                })?;
            (open.dir.clone(), rec.path.clone())
        };

        let mut guard = lock::acquire(&paths::lock_file(&dir))?;
        let resolved = guard.with(|| -> Result<Resolution> {
            let open = self.open.as_ref().expect("checked above");
            let disk = open.fs.read(&path).ok();
            let disk_rev = disk.as_ref().map(|b| BaseRev {
                size: b.len() as u64,
                mtime_ns: open.fs.stat(&path).map(|s| s.mtime_ns).unwrap_or(0),
                hash: notes_fs::hash(b),
            });
            // The shape the file has *now* decides the bytes written, exactly as
            // in `save_note`: the disk is the authority on what the file is.
            let profile = match &disk {
                Some(b) => TextProfile::detect(b).0,
                None => TextProfile {
                    encoding: notes_model::Encoding::Utf8,
                    bom: false,
                    eol: notes_model::Eol::Lf,
                    final_newline: text.ends_with('\n'),
                },
            };
            if let (Some(reason), true) = (profile.read_only_reason(), disk.is_some()) {
                return Err(CoreError::ReadOnly { note_id, reason });
            }
            let rev = disk_rev.clone().unwrap_or_else(|| base_rev.clone());

            match choice {
                ConflictChoice::KeepLocal => {
                    if let Some(bytes) = &disk {
                        conflicts::snapshot(
                            &dir,
                            note_id,
                            &path,
                            conflicts::Side::Disk,
                            &rev,
                            choice,
                            bytes,
                        )?;
                    }
                    // `expect: None` on purpose. The user has just been shown
                    // both versions and said which one wins; re-checking the
                    // base revision here would refuse the very thing they
                    // answered. Everything they are overwriting is in
                    // `conflicts/` one line above.
                    let encoded = profile.encode(text);
                    open.arm(&path, notes_fs::hash(&encoded));
                    match open.fs.write_atomic(&path, &encoded, None) {
                        Ok(_) => {}
                        Err(CoreError::Io {
                            kind: notes_model::IoKind::NotFound,
                            ..
                        }) => {
                            // Removed externally. `KeepLocal` is the user asking
                            // for it back, which is the only circumstance in
                            // which this application recreates a path it did not
                            // create (scope §12 forbids doing it *on its own*).
                            open.fs.create_new(&path, &encoded)?;
                        }
                        Err(e) => return Err(e),
                    }
                    Ok(Resolution::Reopen(path.clone()))
                }

                ConflictChoice::UseDisk => {
                    conflicts::snapshot(
                        &dir,
                        note_id,
                        &path,
                        conflicts::Side::Local,
                        base_rev,
                        choice,
                        text.as_bytes(),
                    )?;
                    if disk.is_none() {
                        // Accepting a deletion. The buffer is safe in
                        // `conflicts/`, and the tab has nothing left to show.
                        return Ok(Resolution::Gone);
                    }
                    Ok(Resolution::Reopen(path.clone()))
                }

                ConflictChoice::SaveAsCopy => {
                    let copy = self.copy_name(&path)?;
                    let bytes = profile.encode(text);
                    open.arm(&copy, notes_fs::hash(&bytes));
                    open.fs.create_new(&copy, &bytes)?;
                    Ok(Resolution::Reopen(copy))
                }
            }
        })??;

        // Whatever happened, this note is no longer suspended and its draft has
        // served its purpose: the buffer is on disk, or in `conflicts/`, or
        // both.
        let open = self.open_mut()?;
        open.suspended.remove(&note_id);
        drafts::discard(&paths::drafts_dir(&dir), note_id)?;

        match resolved {
            Resolution::Reopen(p) => self.open_note(&p),
            Resolution::Gone => Err(CoreError::NotFound {
                path: path.to_string(),
            }),
        }
    }

    /// `nome (local).md`, or `nome (local 2).md` when that is taken.
    ///
    /// Scope §12 names the first form. The numbered fallback exists because
    /// `create_new` never overwrites, and a second conflict on the same note
    /// otherwise has nowhere to go.
    fn copy_name(&self, path: &RelPath) -> Result<RelPath> {
        let parent = path.parent().unwrap_or_else(RelPath::root);
        let name = path.file_name();
        let (stem, ext) = match name.rfind('.') {
            Some(i) if i > 0 => (&name[..i], &name[i..]),
            _ => (name, ""),
        };
        for n in 1..1000 {
            let candidate = if n == 1 {
                format!("{stem} (local){ext}")
            } else {
                format!("{stem} (local {n}){ext}")
            };
            let rel = parent.join(&candidate)?;
            if self.check_collision(&rel).is_ok() {
                return Ok(rel);
            }
        }
        Err(CoreError::AlreadyExists {
            path: format!("{stem} (local …){ext}"),
        })
    }

    /// Everything kept in `conflicts/`, and what it costs.
    pub fn list_conflicts(&self) -> Result<Conflicts> {
        Ok(conflicts::list(&self.open()?.dir))
    }

    // ---- reload, close, convert -----------------------------------------

    /// Re-read a note from disk, discarding nothing: the caller decides when a
    /// buffer is clean enough to be replaced. Reconciliation calls it after an
    /// external change to a note whose buffer has no edits (§8).
    pub fn reload_note(&mut self, note_id: NoteId) -> Result<OpenedNote> {
        let path = self
            .open()?
            .registry
            .record(note_id)
            .ok_or_else(|| CoreError::NotFound {
                path: note_id.to_string(),
            })?
            .path
            .clone();
        self.open_note(&path)
    }

    /// Forget the per-note state a closed tab no longer needs.
    ///
    /// **The draft is not touched.** A draft outlives the tab on purpose: it is
    /// removed by a confirmed write or by the user saying discard, and by
    /// nothing else (§4.2).
    pub fn close_note(&mut self, note_id: NoteId) -> Result<()> {
        self.open_mut()?.suspended.remove(&note_id);
        Ok(())
    }

    /// Rewrite a note's line endings, **because the user asked**.
    ///
    /// This is the one command in the application that changes a file the user
    /// did not edit, and it exists for one situation: a mixed-EOL note opens
    /// read-only (`ReadOnlyReason::MixedEol`), and without a way to convert it
    /// the application would be refusing to edit a file while offering no way
    /// forward. The old bytes go to `conflicts/` first, so an unwanted
    /// conversion is recoverable like any other resolution.
    pub fn convert_eol(&mut self, note_id: NoteId, eol: notes_model::Eol) -> Result<OpenedNote> {
        if eol == notes_model::Eol::Mixed {
            return Err(CoreError::InvalidPath {
                path: note_id.to_string(),
                reason: "mixed line endings are what conversion exists to remove".into(),
            });
        }
        let (dir, path) = {
            let open = self.open()?;
            if open.read_only {
                return Err(CoreError::ReadOnly {
                    note_id,
                    reason: ReadOnlyReason::Workspace,
                });
            }
            let rec = open
                .registry
                .record(note_id)
                .ok_or_else(|| CoreError::NotFound {
                    path: note_id.to_string(),
                })?;
            (open.dir.clone(), rec.path.clone())
        };

        let mut guard = lock::acquire(&paths::lock_file(&dir))?;
        guard.with(|| -> Result<()> {
            let open = self.open.as_ref().expect("checked above");
            let bytes = open.fs.read(&path)?;
            let (profile, text) = TextProfile::detect(&bytes);
            // A file that is not UTF-8 has no line endings to convert; guessing
            // an encoding is what scope §7.5 forbids.
            let Some(text) = text else {
                return Err(CoreError::ReadOnly {
                    note_id,
                    reason: ReadOnlyReason::NotUtf8,
                });
            };
            let rev = BaseRev {
                size: bytes.len() as u64,
                mtime_ns: open.fs.stat(&path)?.mtime_ns,
                hash: notes_fs::hash(&bytes),
            };
            conflicts::snapshot(
                &dir,
                note_id,
                &path,
                conflicts::Side::Disk,
                &rev,
                ConflictChoice::KeepLocal,
                &bytes,
            )?;
            // **`detect` does not normalise a mixed file**, and that is the
            // one thing this function has to know: it rewrites `\r\n` only when
            // the whole file is CRLF, because for every other profile the
            // editor's text and the file's text agree. A mixed file is the
            // exception, so the flattening happens here — `\r\n` first, then the
            // lone `\r`s that made it mixed in the first place.
            let flat = text.replace("\r\n", "\n").replace('\r', "\n");
            let target = TextProfile {
                eol,
                final_newline: flat.ends_with('\n'),
                ..profile
            };
            let converted = target.encode(&flat);
            open.arm(&path, notes_fs::hash(&converted));
            open.fs.write_atomic(&path, &converted, None)?;
            Ok(())
        })??;

        self.open_note(&path)
    }

    pub fn create_note(&mut self, dir: &RelPath, name: &str) -> Result<Entry> {
        // The tree is about to change shape; quick open must not offer a
        // path that is no longer there.
        self.invalidate_paths();
        // Validate what the user typed **before** the extension is appended.
        // Otherwise `trailing-dot.` becomes `trailing-dot..md`, which is legal
        // — so the app would silently accept a name it had just been asked to
        // refuse, and produce a file the user did not name.
        notes_model::portable_name(name).map_err(|rule| CoreError::InvalidPath {
            path: name.to_string(),
            reason: rule.to_string(),
        })?;
        let name = if RelPath::parse(name).map(|p| p.is_note()).unwrap_or(false) {
            name.to_string()
        } else {
            format!("{name}.md")
        };
        let path = dir.join(&name)?;
        self.check_name(&name, &path)?;
        self.open()?.arm(&path, notes_fs::hash(b""));
        self.open()?.fs.create_new(&path, b"")?;
        Ok(Entry {
            name,
            is_note: path.is_note(),
            kind: notes_model::EntryKind::File,
            size: Some(0),
            path,
        })
    }

    pub fn create_dir(&mut self, dir: &RelPath, name: &str) -> Result<Entry> {
        // The tree is about to change shape; quick open must not offer a
        // path that is no longer there.
        self.invalidate_paths();
        let path = dir.join(name)?;
        self.check_name(name, &path)?;
        self.open()?.fs.create_dir(&path)?;
        Ok(Entry {
            name: name.to_string(),
            is_note: false,
            kind: notes_model::EntryKind::Dir,
            size: None,
            path,
        })
    }

    /// Refuse a new name that is illegal on a platform the workspace might be
    /// carried to, and one that collides under the root's own case and
    /// normalisation rules — not just one that is byte-identical (scope §7.6).
    fn check_name(&self, name: &str, path: &RelPath) -> Result<()> {
        notes_model::portable_name(name).map_err(|rule| CoreError::InvalidPath {
            path: name.to_string(),
            reason: rule.to_string(),
        })?;
        self.check_collision(path)
    }

    fn check_collision(&self, path: &RelPath) -> Result<()> {
        let open = self.open()?;
        let parent = path.parent().unwrap_or_else(RelPath::root);
        let key = notes_model::CompareKey::new(path, open.registry.case_insensitive);
        for e in open.fs.list(&parent)? {
            if notes_model::CompareKey::new(&e.path, open.registry.case_insensitive) == key {
                return Err(CoreError::AlreadyExists {
                    path: path.to_string(),
                });
            }
        }
        Ok(())
    }

    /// What the watcher has managed so far.
    ///
    /// The walk that installs one watch per directory runs on its own thread, so
    /// this is a progress report and not a result: `walking` is true while it is
    /// still going. `unreadable` and `over_limit` are the two ways a workspace
    /// ends up **partly** watched, and both are numbers the interface shows
    /// rather than a flag that says "degraded" and hides which.
    pub fn watch_status(&self) -> WatchStatus {
        let Some(open) = self.open.as_ref() else {
            return WatchStatus::default();
        };
        let Some(watch) = open.watch.as_ref() else {
            return WatchStatus::default();
        };
        let p = watch.progress();
        WatchStatus {
            watching: watch.degraded.is_none(),
            walking: p.walking,
            dirs: p.dirs,
            unreadable: p.unreadable,
            over_limit: p.over_limit,
            degraded: watch.degraded.as_ref().map(|d| match d {
                notes_fs::Degraded::Unsupported(m) | notes_fs::Degraded::WatchLimit(m) => m.clone(),
            }),
        }
    }

    // ---- search (0.1c) --------------------------------------------------

    /// Drop the quick-open path cache. Called by everything that changes the
    /// shape of the tree, so `Ctrl+P` never offers a note that has been renamed
    /// out from under it.
    fn invalidate_paths(&self) {
        if let Some(open) = self.open.as_ref() {
            open.paths.lock().unwrap().stale = true;
        }
    }

    /// Fuzzy match over paths, from memory — and **never blocking**.
    ///
    /// The list is built on its own thread. A call made while it is still
    /// filling matches what exists so far and says so, which is the answer that
    /// keeps `Ctrl+P` instant on a folder of hundreds of thousands of
    /// directories (`docs/DECISIONS-0.1c.md` D-09).
    pub fn quick_open(&self, query: &str, limit: usize) -> Result<search::QuickOpen> {
        let open = self.open()?;
        let show_hidden = open
            .registry
            .settings
            .show_hidden
            .unwrap_or(self.settings.files.show_hidden);

        let mut state = open.paths.lock().unwrap();
        // A walk that is still running is **left alone**, stale or not.
        //
        // Restarting on every invalidation is what the first version did, and on
        // a folder whose index takes fifteen seconds it means a folder with any
        // background activity in it — a build, a `git` operation — restarts the
        // walk faster than it can finish, so every `Ctrl+P` sees an empty list
        // forever. A list a few seconds old is a better answer than a rebuild
        // that never lands, and the palette already says it is still filling.
        // The staleness is remembered, not lost: the next call after the walk
        // ends starts a fresh one.
        let rebuild = match state.index.as_ref() {
            None => true,
            Some(ix) => state.stale && !ix.snapshot().building,
        };
        if rebuild {
            // Dropping the previous index cancels its walk.
            state.index = Some(index::PathIndex::start(
                open.fs.root(),
                open.extra_ignore.clone(),
                show_hidden,
            ));
            state.stale = false;
        }
        let snapshot = state.index.as_ref().expect("just set").snapshot();
        drop(state);

        Ok(search::QuickOpen {
            matches: search::quick_match(&snapshot.paths, query, limit),
            indexed: snapshot.paths.len(),
            building: snapshot.building,
            unreadable: snapshot.unreadable,
        })
    }

    /// Start a content search. **Starting one cancels the previous**, because
    /// the caller is a search box and the previous query is no longer wanted.
    pub fn search_start(
        &mut self,
        query: &str,
        opts: search::SearchOpts,
    ) -> Result<search::SearchId> {
        let root = self.open()?.fs.root().to_path_buf();
        // Dropping the old `Search` cancels its walk.
        self.search = None;
        let s = search::Search::start(&root, query, opts)?;
        let id = s.id();
        self.search = Some(s);
        Ok(id)
    }

    /// Take the hits found since the last poll.
    ///
    /// An id that is not the running search answers `done` with nothing rather
    /// than an error: by the time a late poll arrives the user has already typed
    /// again, and that is not a failure.
    pub fn search_poll(&self, id: search::SearchId) -> Result<search::SearchProgress> {
        match self.search.as_ref() {
            Some(s) if s.id() == id => Ok(s.poll()),
            _ => Ok(search::SearchProgress {
                id,
                hits: Vec::new(),
                files_scanned: 0,
                total_hits: 0,
                done: true,
                cancelled: true,
                truncated: false,
            }),
        }
    }

    pub fn search_cancel(&mut self, id: search::SearchId) -> Result<()> {
        if self.search.as_ref().is_some_and(|s| s.id() == id) {
            self.search = None;
        }
        Ok(())
    }

    // ---- session and settings -------------------------------------------

    pub fn session(&self) -> Result<Session> {
        Ok(
            match state::load::<Session>(&paths::session_file(&self.open()?.dir))? {
                Loaded::Ok(s) => s,
                Loaded::Fresh | Loaded::TooNew { .. } => Session::default(),
            },
        )
    }

    pub fn save_session(&self, s: &Session) -> Result<()> {
        state::store(&paths::session_file(&self.open()?.dir), s)
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn set_settings(&mut self, s: Settings) -> Result<()> {
        state::store(&paths::global_settings(&self.data_dir), &s)?;
        self.settings = s;
        Ok(())
    }
}

/// What `resolve_conflict` decided to do once the lock is released.
enum Resolution {
    Reopen(RelPath),
    /// The note is gone and the user accepted that.
    Gone,
}

/// Persist the registry. One place, so every caller writes it the same way.
pub(crate) fn store_registry(dir: &Path, registry: &Registry) -> Result<()> {
    state::store(&paths::registry_file(dir), registry)
}

/// `.notes/config.json`'s `ignore` list, when the user has turned it on.
/// Extends `IGNORE_DEFAULT`, never replaces it.
fn read_portable_ignore(root: &Path) -> Vec<String> {
    let p = root.join(".notes/config.json");
    let Ok(bytes) = std::fs::read(&p) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Vec::new();
    };
    v.get("ignore")
        .and_then(|i| i.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// An RFC 3339 timestamp in UTC, without a date-time dependency.
pub(crate) fn now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (mut y, mut days) = (1970i64, secs.div_euclid(86_400));
    let tod = secs.rem_euclid(86_400);
    loop {
        let len = if is_leap(y) { 366 } else { 365 };
        if days < len {
            break;
        }
        days -= len;
        y += 1;
    }
    let months = [
        31,
        if is_leap(y) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0;
    while days >= months[m] {
        days -= months[m];
        m += 1;
    }
    format!(
        "{y:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        m + 1,
        days + 1,
        tod / 3600,
        (tod % 3600) / 60,
        tod % 60
    )
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    #[test]
    fn timestamps_look_like_rfc3339() {
        let s = super::now();
        assert_eq!(s.len(), 20, "{s}");
        assert!(s.ends_with('Z'));
        assert!(s.starts_with("20"), "{s}");
        assert_eq!(&s[4..5], "-");
        assert_eq!(&s[10..11], "T");
    }
}
