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
            let r = &p.revision;
            if incoming.heads.get(&r.note).copied() != p.expected
                || p.expected.is_some_and(|id| !r.parents.contains(&id))
                || (p.expected.is_none() && !r.parents.is_empty())
                || r.parents
                    .iter()
                    .any(|id| !incoming.revisions.contains_key(id))
                || incoming.revisions.insert(r.id, r.clone()).is_some()
            {
                return Err(Error::Invalid);
            }
            incoming.heads.insert(r.note, r.id);
        }
        incoming.validate().map_err(|_| Error::Invalid)?;
        if self.cursor != self.received.len() {
            return Err(Error::Invalid);
        }
        for p in self.pending.iter().chain(&self.received) {
            bytes = bytes.saturating_add(content(p).map_err(|_| Error::Invalid)?.len());
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
            content(&p).map_err(|_| Error::Protocol)?;
            state.received.push(p);
        }
        state.cursor = page.next_cursor;
        // Heads can reference later pages. They are hints, never applied heads.
        self.save(&state, false)?;
        Ok(())
    }
    /// Export a received publication into a new file in private operational
    /// storage. It is not a source write and cannot overwrite an existing file.
    pub fn export(&self, id: Uuid) -> Result<PathBuf> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        let p = state
            .received
            .iter()
            .find(|p| p.revision.id == id)
            .ok_or(Error::Invalid)?;
        if p.revision.content.is_none() {
            return Err(Error::Invalid);
        }
        let bytes = content(p).map_err(|_| Error::Invalid)?;
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
            let applied = notes_core::sync::apply_received(
                &state.source,
                &core_data,
                &p.revision.path,
                &bytes,
                previous.as_ref().map(|n| &n.local),
                retry,
                || {
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
