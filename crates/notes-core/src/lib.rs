//! `WorkspaceService` — the only public API of this application.
//!
//! `src-tauri` and, from 0.3, `notes-mcp` are clients of it and hold no policy
//! of their own (ADR-003). That is what lets the MCP server work with the window
//! closed, and what lets every rule below be tested with `cargo test` and no
//! Tauri (`ARCHITECTURE.md` §14).

pub mod drafts;
pub mod ignore;
mod lock;
pub mod paths;
pub mod preview;
pub mod registry;
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

pub use drafts::{DraftInfo, DraftReason};
pub use preview::{Asset, Document, Rendered};
pub use registry::{Registry, WorkspaceEntry, WorkspacesIndex};
pub use settings::{Session, Settings, Tab};

use state::Loaded;

type Result<T> = std::result::Result<T, CoreError>;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DraftChoice {
    Restore,
    Discard,
}

struct Open {
    id: WorkspaceId,
    fs: LocalFs,
    dir: PathBuf,
    registry: Registry,
    read_only: bool,
    extra_ignore: Vec<String>,
    suspended: BTreeSet<NoteId>,
}

pub struct WorkspaceService {
    data_dir: PathBuf,
    open: Option<Open>,
    settings: Settings,
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

    pub fn create_note(&mut self, dir: &RelPath, name: &str) -> Result<Entry> {
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
