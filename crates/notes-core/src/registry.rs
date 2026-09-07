use std::collections::BTreeMap;

use notes_model::{BaseRev, ContentHash, NativeId, NoteId, RelPath, Stat, WorkspaceId};
use serde::{Deserialize, Serialize};

use crate::state::Schemad;

/// One note's identity, as the app knows it. **None of this is ever written into
/// the `.md` file** (`ARCHITECTURE.md` §18.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteRecord {
    pub path: RelPath,
    pub size: u64,
    pub mtime_ns: i128,
    pub hash: ContentHash,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_id: Option<NativeId>,
    pub rev: u64,
    pub first_seen: String,
    pub last_seen: String,
}

impl NoteRecord {
    pub fn base_rev(&self) -> BaseRev {
        BaseRev { size: self.size, mtime_ns: self.mtime_ns, hash: self.hash.clone() }
    }
}

/// Per-workspace overrides. `ARCHITECTURE.md` §4.5 says these live in the
/// registry; its schema example omitted the field, which is the whole of gap G6.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autosave_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_hidden: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub schema: u32,
    pub workspace_id: WorkspaceId,
    pub root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_native_id: Option<NativeId>,
    pub case_insensitive: bool,
    pub created_at: String,
    #[serde(default)]
    pub settings: WorkspaceSettings,
    pub notes: BTreeMap<NoteId, NoteRecord>,
    /// Reserved for 0.6. Empty before it, and stated so rather than left to be
    /// guessed at.
    #[serde(default)]
    pub tombstones: BTreeMap<String, String>,
}

impl Schemad for Registry {
    const CURRENT: u32 = 1;
    const NAME: &'static str = "registry.json";
    fn schema(&self) -> u32 {
        self.schema
    }
}

impl Registry {
    pub fn new(workspace_id: WorkspaceId, root: &str, case_insensitive: bool, root_native_id: Option<NativeId>) -> Self {
        Self {
            schema: Self::CURRENT,
            workspace_id,
            root: root.to_string(),
            root_native_id,
            case_insensitive,
            created_at: crate::now(),
            settings: WorkspaceSettings::default(),
            notes: BTreeMap::new(),
            tombstones: BTreeMap::new(),
        }
    }

    pub fn find_by_path(&self, path: &RelPath) -> Option<(NoteId, &NoteRecord)> {
        self.notes.iter().find(|(_, r)| &r.path == path).map(|(id, r)| (*id, r))
    }

    /// Assign or refresh the identity of a note that has just been opened.
    ///
    /// **The registry is populated lazily, when a note is opened — never by
    /// listing** (`docs/DECISIONS-0.1a.md` D-09). Assigning an id at listing
    /// time would mean hashing every file to fill `hash`, and 0.1a has an
    /// acceptance criterion that a 10 000-note workspace lists in under a second
    /// without reading content.
    pub fn observe(&mut self, path: &RelPath, stat: &Stat, hash: ContentHash) -> NoteId {
        let now = crate::now();
        if let Some((id, _)) = self.find_by_path(path) {
            let rec = self.notes.get_mut(&id).expect("just found");
            if rec.hash != hash {
                rec.rev += 1;
            }
            rec.size = stat.size;
            rec.mtime_ns = stat.mtime_ns;
            rec.hash = hash;
            rec.native_id = stat.native_id.clone();
            rec.last_seen = now;
            return id;
        }
        let id = NoteId::new();
        self.notes.insert(
            id,
            NoteRecord {
                path: path.clone(),
                size: stat.size,
                mtime_ns: stat.mtime_ns,
                hash,
                native_id: stat.native_id.clone(),
                rev: 1,
                first_seen: now.clone(),
                last_seen: now,
            },
        );
        id
    }

    pub fn record(&self, id: NoteId) -> Option<&NoteRecord> {
        self.notes.get(&id)
    }
}

/// The global index. `last_workspace` is an addition to `ARCHITECTURE.md` §4's
/// shape: "persistir o último workspace" is 0.1a scope, and neither the
/// per-workspace directory nor a list of recents answers *which one was last*
/// without inventing a tie-break (`docs/DECISIONS-0.1a.md` D-10).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacesIndex {
    pub schema: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_workspace: Option<WorkspaceId>,
    #[serde(default)]
    pub workspaces: Vec<WorkspaceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    pub id: WorkspaceId,
    /// The canonical root at the time of registration. A moved folder is not
    /// found by this and is offered for reconnection rather than adopted
    /// silently (scope §6.2).
    pub root: String,
    pub display_name: String,
    pub last_opened: String,
}

impl Schemad for WorkspacesIndex {
    const CURRENT: u32 = 1;
    const NAME: &'static str = "workspaces.json";
    fn schema(&self) -> u32 {
        self.schema
    }
}

impl Default for WorkspacesIndex {
    fn default() -> Self {
        Self { schema: Self::CURRENT, last_workspace: None, workspaces: Vec::new() }
    }
}

impl WorkspacesIndex {
    pub fn by_root(&self, root: &str) -> Option<&WorkspaceEntry> {
        self.workspaces.iter().find(|w| w.root == root)
    }

    pub fn touch(&mut self, id: WorkspaceId, root: &str, display_name: &str) {
        let now = crate::now();
        match self.workspaces.iter_mut().find(|w| w.id == id) {
            Some(w) => {
                w.root = root.to_string();
                w.last_opened = now;
            }
            None => self.workspaces.push(WorkspaceEntry {
                id,
                root: root.to_string(),
                display_name: display_name.to_string(),
                last_opened: now,
            }),
        }
        self.last_workspace = Some(id);
    }
}
