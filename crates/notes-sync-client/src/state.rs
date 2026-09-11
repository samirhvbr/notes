use crate::{
    remote::{Endpoint, Transport},
    Error, Result,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use notes_sync::{
    transfer::{content, Publication},
    Journal, Revision,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;
const MAX_STATE: usize = 64 * 1024 * 1024;
const MAX_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Upload,
    Receive,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    schema: u32,
    endpoint: Endpoint,
    source: PathBuf,
    mode: Mode,
    device: Uuid,
    local: Journal,
    pending: Vec<Publication>,
    received: Vec<Publication>,
    cursor: usize,
}
impl State {
    fn validate(&self) -> Result<()> {
        if self.schema != 1
            || !self.source.is_absolute()
            || self.pending.len() + self.received.len() > 10_000
        {
            return Err(Error::Invalid);
        }
        self.endpoint.validate()?;
        self.local.validate().map_err(|_| Error::Invalid)?;
        let mut bytes = 0usize;
        let mut ids = BTreeSet::new();
        for p in &self.pending {
            if p.workspace != self.local.workspace
                || self.local.revisions.get(&p.revision.id) != Some(&p.revision)
                || !ids.insert(p.revision.id)
            {
                return Err(Error::Invalid);
            }
        }
        let mut incoming = Journal::new(self.local.workspace);
        for p in &self.received {
            if p.workspace != self.local.workspace {
                return Err(Error::Invalid);
            }
            notes_sync::transfer::append(&mut incoming, p).map_err(|_| Error::Invalid)?;
        }
        incoming.validate().map_err(|_| Error::Invalid)?;
        if incoming.revisions.len() + self.local.revisions.len() > 20_000 {
            return Err(Error::Limit);
        }
        for p in &self.pending {
            if p.branches
                .iter()
                .any(|b| self.local.revisions.get(&b.revision.id) != Some(&b.revision))
            {
                return Err(Error::Invalid);
            }
        }
        if self.cursor != self.received.len() {
            return Err(Error::Invalid);
        }
        for p in self.pending.iter().chain(&self.received) {
            bytes = bytes
                .saturating_add(notes_sync::transfer::payload_size(p).map_err(|_| Error::Invalid)?);
            if bytes > MAX_BYTES {
                return Err(Error::Limit);
            }
        }
        Ok(())
    }
}
#[derive(Serialize)]
pub struct Status {
    pub pending: usize,
    pub received: usize,
    pub cursor: usize,
    pub applied: bool,
    pub applied_revisions: usize,
    pub acknowledged_revisions: usize,
}
pub struct Store {
    dir: PathBuf,
}
impl Store {
    pub fn open(dir: &Path) -> Result<Self> {
        if !dir.is_absolute()
            || dir
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            dir: dir.to_path_buf(),
        })
    }
    fn lock(&self) -> Result<fd_lock::RwLock<File>> {
        let meta = fs::symlink_metadata(&self.dir).map_err(|_| Error::Storage)?;
        if !meta.is_dir() {
            return Err(Error::Invalid);
        }
        let path = self.dir.join("client.lock");
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::Invalid);
        }
        let mut opts = OpenOptions::new();
        opts.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        Ok(fd_lock::RwLock::new(
            opts.open(path).map_err(|_| Error::Storage)?,
        ))
    }
    fn load(&self) -> Result<State> {
        let path = self.dir.join("client.json");
        let meta = fs::symlink_metadata(&path).map_err(|_| Error::Storage)?;
        if !meta.is_file() || meta.len() > MAX_STATE as u64 {
            return Err(Error::Invalid);
        }
        let mut bytes = vec![];
        File::open(path)
            .map_err(|_| Error::Storage)?
            .take(MAX_STATE as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Storage)?;
        if bytes.len() > MAX_STATE {
            return Err(Error::Limit);
        }
        let state: State = serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)?;
        state.validate()?;
        notes_core::sync::validate_state_location(&[&state.source], &self.dir)
            .map_err(|_| Error::Invalid)?;
        Ok(state)
    }
    fn save(&self, state: &State, initial: bool) -> Result<()> {
        state.validate()?;
        let bytes = serde_json::to_vec(state).map_err(|_| Error::Storage)?;
        if bytes.len() > MAX_STATE {
            return Err(Error::Limit);
        }
        let mut tmp = tempfile::NamedTempFile::new_in(&self.dir).map_err(|_| Error::Storage)?;
        tmp.write_all(&bytes).map_err(|_| Error::Storage)?;
        tmp.as_file().sync_all().map_err(|_| Error::Storage)?;
        if initial {
            tmp.persist_noclobber(self.dir.join("client.json"))
                .map_err(|_| Error::Storage)?;
        } else {
            tmp.persist(self.dir.join("client.json"))
                .map_err(|_| Error::Storage)?;
        }
        #[cfg(unix)]
        File::open(&self.dir)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Storage)?;
        Ok(())
    }
    /// This explicit command is the user's pairing confirmation. Upload requires
    /// an empty remote inbox; Receive currently caches bytes without application.
    pub fn initialize(
        &self,
        source: &Path,
        endpoint: Endpoint,
        mode: Mode,
        transport: &mut impl Transport,
    ) -> Result<()> {
        endpoint.validate()?;
        let source = fs::canonicalize(source).map_err(|_| Error::Invalid)?;
        notes_core::sync::validate_state_location(&[&source], &self.dir)
            .map_err(|_| Error::Invalid)?;
        let page = transport.page(0)?;
        if mode == Mode::Upload
            && (!page.revisions.is_empty() || page.next_cursor != 0 || page.has_more)
        {
            return Err(Error::Conflict);
        }
        fs::create_dir_all(&self.dir).map_err(|_| Error::Storage)?;
        if fs::symlink_metadata(&self.dir)
            .map_err(|_| Error::Storage)?
            .file_type()
            .is_symlink()
        {
            return Err(Error::Invalid);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.dir, fs::Permissions::from_mode(0o700))
                .map_err(|_| Error::Storage)?;
        }
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        if self.dir.join("client.json").exists() {
            return Err(Error::Invalid);
        }
        self.save(
            &State {
                schema: 1,
                endpoint,
                source,
                mode,
                device: Uuid::new_v4(),
                local: Journal::new(page.workspace),
                pending: vec![],
                received: vec![],
                cursor: 0,
            },
            true,
        )
    }
    pub fn endpoint(&self) -> Result<Endpoint> {
        let lock = self.lock()?;
        let _guard = lock.try_read().map_err(|_| Error::Busy)?;
        Ok(self.load()?.endpoint)
    }
    pub fn status(&self) -> Result<Status> {
        let lock = self.lock()?;
        let _guard = lock.try_read().map_err(|_| Error::Busy)?;
        let s = self.load()?;
        let app = self.application(&s)?;
        let applied = app.as_ref().map(|a| a.next).unwrap_or(0);
        let acknowledged = app.as_ref().map(|a| a.acknowledged).unwrap_or(0);
        Ok(Status {
            pending: s.pending.len(),
            received: s.received.len(),
            cursor: s.cursor,
            applied: applied > 0 && applied == s.received.len(),
            applied_revisions: applied,
            acknowledged_revisions: acknowledged,
        })
    }
    /// Capture only saved bytes. Missing inventory entries are reported, never
    /// silently translated into tombstones. The whole capture is one transaction.
    pub fn stage(&self) -> Result<usize> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let mut state = self.load()?;
        if state.mode != Mode::Upload {
            return Err(Error::Invalid);
        }
        let snapshot = notes_core::sync::capture(&state.source, &self.dir.join("core"))
            .map_err(|_| Error::Invalid)?;
        let present: BTreeSet<_> = snapshot.iter().map(|(f, _)| f.note).collect();
        let missing = state
            .local
            .heads
            .keys()
            .filter(|id| {
                !present.contains(id) && state.local.head(**id).is_some_and(|r| r.content.is_some())
            })
            .count();
        for (file, bytes) in snapshot {
            if state
                .local
                .head(file.note)
                .is_some_and(|r| r.path == file.path && r.content.as_ref() == Some(&file.content))
            {
                continue;
            }
            let expected = state.local.heads.get(&file.note).copied();
            let revision = Revision::new(
                file.note,
                expected.into_iter().collect(),
                state.device,
                file.path,
                Some(file.content),
            );
            state
                .local
                .commit(revision.clone(), expected)
                .map_err(|_| Error::Conflict)?;
            state.pending.push(Publication {
                branches: vec![],
                workspace: state.local.workspace,
                expected,
                revision,
                content_base64: Some(STANDARD.encode(bytes)),
            });
        }
        self.save(&state, false)?;
        Ok(missing)
    }
    /// Execute one bounded batch. Every receipt is checkpointed separately;
    /// unknown outcomes retain the original UUID and bytes for idempotent retry.
    pub fn transfer(&self, transport: &mut impl Transport) -> Result<()> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let mut state = self.load()?;
        if transport.page(state.cursor)?.workspace != state.local.workspace {
            return Err(Error::Protocol);
        }
        for _ in 0..20 {
            let Some(p) = state.pending.first() else {
                break;
            };
            transport.publish(p)?;
            state.pending.remove(0);
            self.save(&state, false)?;
        }
        self.fetch_into(&mut state, transport)
    }
    /// Receive without publishing, so a rejected outbox cannot hide its peer.
    pub fn fetch(&self, transport: &mut impl Transport) -> Result<()> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        self.fetch_into(&mut self.load()?, transport)
    }
    fn fetch_into(&self, state: &mut State, transport: &mut impl Transport) -> Result<()> {
        let page = transport.page(state.cursor)?;
        if page.workspace != state.local.workspace
            || page.revisions.len() > 20
            || page.next_cursor != state.cursor + page.revisions.len()
            || (page.has_more && page.revisions.is_empty())
        {
            return Err(Error::Protocol);
        }
        for revision in &page.revisions {
            let p = transport.fetch(revision.id)?;
            if p.workspace != state.local.workspace || p.revision != *revision {
                return Err(Error::Protocol);
            }
            notes_sync::transfer::payload_size(&p).map_err(|_| Error::Protocol)?;
            state.received.push(p);
        }
        state.cursor = page.next_cursor;
        self.save(state, false)
    }
    fn incoming(state: &State) -> Result<Journal> {
        let mut journal = Journal::new(state.local.workspace);
        for p in &state.received {
            notes_sync::transfer::append(&mut journal, p).map_err(|_| Error::Invalid)?;
        }
        journal.validate().map_err(|_| Error::Invalid)?;
        Ok(journal)
    }
    pub fn conflicts(&self) -> Result<Vec<notes_sync::Action>> {
        let lock = self.lock()?;
        let _guard = lock.try_read().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        Ok(notes_sync::plan(&state.local, &Self::incoming(&state)?)
            .map_err(|_| Error::Conflict)?
            .into_iter()
            .filter(|a| {
                matches!(
                    a,
                    notes_sync::Action::Conflict { .. }
                        | notes_sync::Action::MergeEqual { .. }
                        | notes_sync::Action::PathCollision { .. }
                )
            })
            .collect())
    }
    /// Explicit operator choice of result bytes. This only stages a publication;
    /// it never edits the source, sends traffic, or elects a winner by timestamp.
    pub fn resolve(&self, local: Uuid, remote: Uuid, result: &Path) -> Result<Uuid> {
        self.resolve_choice(local, remote, None, Some(result))
    }
    /// Choose the resulting path and exact bytes, including resurrection after
    /// a remote tombstone. This never renames or writes the source folder.
    pub fn resolve_to(
        &self,
        local: Uuid,
        remote: Uuid,
        path: notes_model::RelPath,
        result: &Path,
    ) -> Result<Uuid> {
        self.resolve_choice(local, remote, Some(path), Some(result))
    }
    /// Choose a tombstone explicitly. Source deletion is a separate operation.
    pub fn resolve_delete(
        &self,
        local: Uuid,
        remote: Uuid,
        path: notes_model::RelPath,
    ) -> Result<Uuid> {
        self.resolve_choice(local, remote, Some(path), None)
    }
    fn resolve_choice(
        &self,
        local: Uuid,
        remote: Uuid,
        path: Option<notes_model::RelPath>,
        result: Option<&Path>,
    ) -> Result<Uuid> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let mut state = self.load()?;
        if state.mode != Mode::Upload {
            return Err(Error::Invalid);
        }
        let incoming = Self::incoming(&state)?;
        let a = state
            .local
            .revisions
            .get(&local)
            .ok_or(Error::Invalid)?
            .clone();
        let b = incoming.revisions.get(&remote).ok_or(Error::Invalid)?;
        if a.note != b.note
            || (path.is_none() && (a.path != b.path || a.content.is_none() || b.content.is_none()))
            || state.local.heads.get(&a.note) != Some(&local)
            || incoming.heads.get(&a.note) != Some(&remote)
        {
            return Err(Error::Conflict);
        }
        let mut graph = state.local.clone();
        graph
            .import(incoming.revisions.values().cloned())
            .map_err(|_| Error::Conflict)?;
        if graph.is_ancestor(local, remote) || graph.is_ancestor(remote, local) {
            return Err(Error::Conflict);
        }
        let path = path.unwrap_or(a.path);
        if !path.is_note()
            || path.as_str().len() > 4096
            || path.as_str().split('/').any(|s| s.starts_with('.'))
        {
            return Err(Error::Invalid);
        }
        let bytes = result
            .map(|result| -> Result<Vec<u8>> {
                let mut bytes = vec![];
                let file = File::open(result).map_err(|_| Error::Storage)?;
                if !file.metadata().map_err(|_| Error::Storage)?.is_file() {
                    return Err(Error::Invalid);
                }
                file.take(notes_sync::transfer::MAX_CONTENT as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| Error::Storage)?;
                if bytes.len() > notes_sync::transfer::MAX_CONTENT {
                    return Err(Error::Limit);
                }
                Ok(bytes)
            })
            .transpose()?;
        let revision = notes_sync::resolve(
            &graph,
            local,
            remote,
            state.device,
            path,
            bytes
                .as_ref()
                .map(|bytes| notes_model::ContentHash::from_bytes(*blake3::hash(bytes).as_bytes())),
        )
        .map_err(|_| Error::Conflict)?;
        let mut branches = vec![];
        let mut seen = BTreeSet::new();
        for p in state.pending.iter().filter(|p| p.revision.note == a.note) {
            for b in
                p.branches
                    .iter()
                    .cloned()
                    .chain(std::iter::once(notes_sync::transfer::Branch {
                        revision: p.revision.clone(),
                        content_base64: p.content_base64.clone(),
                    }))
            {
                if !incoming.revisions.contains_key(&b.revision.id) && seen.insert(b.revision.id) {
                    branches.push(b);
                }
            }
        }
        let publication = Publication {
            workspace: state.local.workspace,
            expected: Some(remote),
            revision: revision.clone(),
            content_base64: bytes.map(|bytes| STANDARD.encode(bytes)),
            branches,
        };
        // Prove the peer can reconstruct the exact chosen parents from this
        // envelope. No pending bytes are removed until the whole state is saved.
        notes_sync::transfer::payload_size(&publication).map_err(|e| match e {
            notes_sync::Error::Limit => Error::Limit,
            _ => Error::Invalid,
        })?;
        let mut replay = incoming;
        notes_sync::transfer::append(&mut replay, &publication).map_err(|_| Error::Conflict)?;
        replay.validate().map_err(|_| Error::Conflict)?;
        graph
            .commit(revision.clone(), Some(local))
            .map_err(|_| Error::Conflict)?;
        state.local = graph;
        state.pending.retain(|p| p.revision.note != a.note);
        state.pending.push(publication);
        self.save(&state, false)?;
        Ok(revision.id)
    }
    /// Export pending, received or retained branch bytes into private operational
    /// storage. It is not a source write and cannot overwrite an existing file.
    pub fn export(&self, id: Uuid) -> Result<PathBuf> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        let p = state
            .pending
            .iter()
            .chain(&state.received)
            .find_map(|p| {
                if p.revision.id == id {
                    return Some(p.clone());
                }
                p.branches
                    .iter()
                    .find(|b| b.revision.id == id)
                    .map(|b| Publication {
                        workspace: p.workspace,
                        expected: None,
                        revision: b.revision.clone(),
                        content_base64: b.content_base64.clone(),
                        branches: vec![],
                    })
            })
            .ok_or(Error::Invalid)?;
        if p.revision.content.is_none() {
            return Err(Error::Invalid);
        }
        let bytes = content(&p).map_err(|_| Error::Invalid)?;
        let mut temp = tempfile::NamedTempFile::new_in(&self.dir).map_err(|_| Error::Storage)?;
        temp.write_all(&bytes).map_err(|_| Error::Storage)?;
        temp.as_file().sync_all().map_err(|_| Error::Storage)?;
        let path = self.dir.join(format!("received-{id}.md"));
        temp.persist_noclobber(&path).map_err(|_| Error::Storage)?;
        Ok(path)
    }
    pub fn received(&self) -> Result<Vec<Revision>> {
        let lock = self.lock()?;
        let _guard = lock.try_read().map_err(|_| Error::Busy)?;
        Ok(self
            .load()?
            .received
            .into_iter()
            .map(|p| p.revision)
            .collect())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplicationReceipt {
    revision: Uuid,
    path: notes_model::RelPath,
    local: notes_core::sync::Applied,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Application {
    schema: u32,
    core_data: PathBuf,
    next: usize,
    notes: std::collections::BTreeMap<notes_model::NoteId, ApplicationReceipt>,
    intent: Option<Uuid>,
    #[serde(default)]
    acknowledged: usize,
}
impl Store {
    fn application(&self, state: &State) -> Result<Option<Application>> {
        let path = self.dir.join("application.json");
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(Error::Storage),
        };
        if !meta.is_file() || meta.len() > MAX_STATE as u64 {
            return Err(Error::Invalid);
        }
        let mut bytes = vec![];
        File::open(path)
            .map_err(|_| Error::Storage)?
            .take(MAX_STATE as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Storage)?;
        if bytes.len() > MAX_STATE {
            return Err(Error::Limit);
        }
        let app: Application = serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)?;
        if app.schema != 1
            || !app.core_data.is_absolute()
            || app.next > state.received.len()
            || app.acknowledged > app.next
            || app.intent.is_some_and(|id| {
                state
                    .received
                    .get(app.next)
                    .is_none_or(|p| p.revision.id != id)
            })
        {
            return Err(Error::Invalid);
        }
        let mut latest = std::collections::BTreeMap::new();
        for p in &state.received[..app.next] {
            latest.insert(p.revision.note, &p.revision);
        }
        if latest.len() != app.notes.len()
            || latest.iter().any(|(id, r)| {
                app.notes.get(id).is_none_or(|n| {
                    n.revision != r.id
                        || n.path != r.path
                        || r.content.as_ref() != Some(&n.local.base_rev.hash)
                })
            })
        {
            return Err(Error::Invalid);
        }
        Ok(Some(app))
    }
    fn save_application(&self, app: &Application) -> Result<()> {
        let bytes = serde_json::to_vec(app).map_err(|_| Error::Storage)?;
        if bytes.len() > MAX_STATE {
            return Err(Error::Limit);
        }
        let mut temp = tempfile::NamedTempFile::new_in(&self.dir).map_err(|_| Error::Storage)?;
        temp.write_all(&bytes).map_err(|_| Error::Storage)?;
        temp.as_file().sync_all().map_err(|_| Error::Storage)?;
        temp.persist(self.dir.join("application.json"))
            .map_err(|_| Error::Storage)?;
        #[cfg(unix)]
        File::open(&self.dir)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Storage)?;
        Ok(())
    }
    /// Apply received creations/updates to the bound folder, with the workspace
    /// closed in every cooperating client using this same application data path.
    pub fn apply(&self, core_data: &Path) -> Result<usize> {
        self.apply_using(core_data, |root, path, bytes, expected, retry, prepare| {
            notes_core::sync::apply_received(root, core_data, path, bytes, expected, retry, prepare)
        })
    }
    fn apply_using(
        &self,
        core_data: &Path,
        mut execute: impl FnMut(
            &Path,
            &notes_model::RelPath,
            &[u8],
            Option<&notes_core::sync::Applied>,
            bool,
            &mut dyn FnMut() -> std::result::Result<(), notes_model::CoreError>,
        ) -> std::result::Result<
            notes_core::sync::Applied,
            notes_model::CoreError,
        >,
    ) -> Result<usize> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        if state.mode != Mode::Receive || !state.pending.is_empty() {
            return Err(Error::Invalid);
        }
        notes_core::sync::validate_state_location(&[&state.source], core_data)
            .map_err(|_| Error::Invalid)?;
        fs::create_dir_all(core_data).map_err(|_| Error::Storage)?;
        let core_data = fs::canonicalize(core_data).map_err(|_| Error::Storage)?;
        let mut app = self.application(&state)?.unwrap_or(Application {
            schema: 1,
            core_data: core_data.clone(),
            next: 0,
            notes: Default::default(),
            intent: None,
            acknowledged: 0,
        });
        if app.core_data != core_data {
            return Err(Error::Invalid);
        }
        let mut count = 0;
        for p in state.received.iter().skip(app.next).take(20) {
            if p.revision.content.is_none() {
                return Err(Error::UnsupportedApplication);
            }
            let previous = app.notes.get(&p.revision.note).cloned();
            if previous.as_ref().map(|n| n.revision) != p.expected {
                return Err(Error::Conflict);
            }
            if previous.as_ref().is_some_and(|n| n.path != p.revision.path) {
                return Err(Error::UnsupportedApplication);
            }
            let bytes = content(p).map_err(|_| Error::Invalid)?;
            let retry = app.intent == Some(p.revision.id);
            let applied = execute(
                &state.source,
                &p.revision.path,
                &bytes,
                previous.as_ref().map(|n| &n.local),
                retry,
                &mut || {
                    app.intent = Some(p.revision.id);
                    self.save_application(&app)
                        .map_err(|_| notes_model::CoreError::Internal {
                            message: "could not persist application intent".into(),
                        })
                },
            )
            .map_err(|e| match e {
                notes_model::CoreError::LockTimeout => Error::Busy,
                _ => Error::ApplicationBlocked,
            })?;
            app.notes.insert(
                p.revision.note,
                ApplicationReceipt {
                    revision: p.revision.id,
                    path: p.revision.path.clone(),
                    local: applied,
                },
            );
            app.next += 1;
            app.intent = None;
            self.save_application(&app)?;
            count += 1;
        }
        Ok(count)
    }
}

impl Store {
    /// Send at most twenty durable application receipts. Never touches source files.
    pub fn acknowledge(&self, transport: &mut impl Transport) -> Result<usize> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        if state.mode != Mode::Receive {
            return Err(Error::Invalid);
        }
        let Some(mut app) = self.application(&state)? else {
            return Ok(0);
        };
        let mut count = 0;
        while app.acknowledged < app.next && count < 20 {
            let p = &state.received[app.acknowledged];
            transport.acknowledge(&notes_sync::transfer::ApplicationAcknowledgment {
                workspace: state.local.workspace,
                device: state.device,
                revision: p.revision.id,
            })?;
            app.acknowledged += 1;
            self.save_application(&app)?;
            count += 1;
        }
        Ok(count)
    }
}

impl Store {
    pub fn open_for_editor(
        &self,
        service: &mut notes_core::WorkspaceService,
    ) -> Result<notes_core::WorkspaceInfo> {
        let lock = self.lock()?;
        let _guard = lock.try_read().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        if state.mode != Mode::Receive {
            return Err(Error::Invalid);
        }
        let data = fs::canonicalize(service.data_dir()).map_err(|_| Error::Storage)?;
        if self
            .application(&state)?
            .is_some_and(|a| a.core_data != data)
        {
            return Err(Error::Invalid);
        }
        service
            .open_sync_workspace(&state.source)
            .map_err(|_| Error::ApplicationBlocked)
    }

    /// The host owns its input barrier until these refreshed buffers are installed.
    pub fn apply_for_editor(
        &self,
        service: &mut notes_core::WorkspaceService,
        mut buffers: Vec<notes_core::sync::BufferSnapshot>,
    ) -> notes_core::sync::SyncApplyResult {
        use notes_core::sync::{apply_in_workspace, SyncApplyResult};
        if buffers.iter().any(|b| b.buffer_version != b.saved_version) {
            return SyncApplyResult {
                applied: None,
                error: Some(notes_model::CoreError::DirtyBuffers {
                    note_ids: buffers.iter().map(|b| b.note_id).collect(),
                    count: buffers.len(),
                }),
                refreshed: vec![],
                reload_failed: false,
            };
        }
        let data = service.data_dir().to_path_buf();
        let outcome = self.apply_using(&data, |root, path, bytes, expected, retry, prepare| {
            if service.workspace_root()? != root {
                return Err(notes_model::CoreError::Unsupported {
                    cap: "receive queue belongs to another workspace".into(),
                });
            }
            let applied =
                apply_in_workspace(service, path, bytes, expected, retry, &buffers, prepare)?;
            for buffer in &mut buffers {
                if buffer.note_id == applied.note_id {
                    buffer.base_rev = applied.base_rev.clone();
                }
            }
            Ok(applied)
        });
        let mut report = Self::reload_for_editor(service, &buffers);
        report.applied = outcome.as_ref().ok().map(|n| *n as u32);
        if let Err(error) = outcome {
            report.error = Some(notes_model::CoreError::Unsupported {
                cap: error.to_string(),
            });
        }
        report
    }

    pub fn reload_for_editor(
        service: &mut notes_core::WorkspaceService,
        buffers: &[notes_core::sync::BufferSnapshot],
    ) -> notes_core::sync::SyncApplyResult {
        if buffers.iter().any(|b| b.buffer_version != b.saved_version) {
            return notes_core::sync::SyncApplyResult {
                applied: None,
                error: Some(notes_model::CoreError::DirtyBuffers {
                    note_ids: buffers.iter().map(|b| b.note_id).collect(),
                    count: buffers.len(),
                }),
                refreshed: vec![],
                reload_failed: true,
            };
        }
        let mut report = notes_core::sync::SyncApplyResult {
            applied: None,
            error: None,
            refreshed: vec![],
            reload_failed: false,
        };
        for buffer in buffers {
            match service.reload_note(buffer.note_id) {
                Ok(note) => report.refreshed.push(note),
                Err(error) => {
                    report.reload_failed = true;
                    report.error = Some(error);
                }
            }
        }
        report
    }
}
