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
    collections::{BTreeMap, BTreeSet},
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    capture: Option<ReceiverCapture>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pairing: Option<Pairing>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiverCapture {
    #[serde(default)]
    publishable: bool,
    core_data: PathBuf,
    note: notes_model::NoteId,
    applied: Uuid,
    branch: Uuid,
    path: notes_model::RelPath,
    local: notes_core::sync::Applied,
}
impl State {
    fn attachments_at(&self, id: Uuid) -> Vec<notes_sync::transfer::Attachment> {
        for p in self.pending.iter().chain(&self.received) {
            if p.revision.id == id {
                return p.attachments.clone();
            }
            if let Some(b) = p.branches.iter().find(|b| b.revision.id == id) {
                return b.attachments.clone();
            }
        }
        vec![]
    }
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
        if let Some(c) = &self.capture {
            if self.mode != Mode::Receive
                || !c.core_data.is_absolute()
                || self.local.revisions.get(&c.branch).is_none_or(|r| {
                    r.note != c.note
                        || r.path != c.path
                        || r.content.as_ref() != Some(&c.local.base_rev.hash)
                        || r.parents.len() != 1
                        || !self.local.is_ancestor(c.applied, c.branch)
                })
                || !incoming.revisions.contains_key(&c.applied)
            {
                return Err(Error::Invalid);
            }
        }
        if self.cursor < self.received.len()
            || (self.endpoint.scope.is_none() && self.cursor != self.received.len())
        {
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
    pub superseded_revisions: usize,
    pub deferred_revisions: usize,
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
                capture: None,
                pairing: None,
            },
            true,
        )
    }
    /// A receive queue must not advertise current while saved local bytes differ
    /// from its receipts. This observes files; it never adopts edits as applied.
    pub fn receiver_changes(&self) -> Result<bool> {
        let lock = self.lock()?;
        let _guard = lock.try_read().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        if state.mode != Mode::Receive {
            return Ok(false);
        }
        let Some(app) = self.application(&state)? else {
            return Ok(false);
        };
        let files = notes_core::sync::capture(&state.source, &self.dir.join("inspection"))
            .map_err(|_| Error::ApplicationBlocked)?;
        let live: Vec<_> = app.notes.values().filter(|r| !r.deleted).collect();
        if files.len() != live.len() {
            return Ok(true);
        }
        for (file, bytes) in files {
            let Some(receipt) = live.iter().find(|r| r.path == file.path) else {
                return Ok(true);
            };
            if receipt.local.base_rev.hash != file.content {
                return Ok(true);
            }
            let assets = notes_core::sync::capture_attachments(
                &state.source,
                &self.dir.join("inspection"),
                &file.path,
                &bytes,
            )
            .map_err(|_| Error::ApplicationBlocked)?;
            if assets != state.attachments_at(receipt.revision) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn source_mode(&self) -> Result<(PathBuf, Mode)> {
        let lock = self.lock()?;
        let _guard = lock.try_read().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        Ok((state.source, state.mode))
    }
    pub fn history(&self) -> Result<Vec<crate::control::HistoryRow>> {
        let lock = self.lock()?;
        let _guard = lock.try_read().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        let mut rows = vec![];
        for (pending, pubs) in [(false, &state.received), (true, &state.pending)] {
            for p in pubs {
                for b in &p.branches {
                    rows.push(crate::control::HistoryRow::new(
                        &b.revision,
                        pending,
                        true,
                        b.attachments.iter().map(|a| a.path.to_string()).collect(),
                    ));
                }
                rows.push(crate::control::HistoryRow::new(
                    &p.revision,
                    pending,
                    false,
                    p.attachments.iter().map(|a| a.path.to_string()).collect(),
                ));
            }
        }
        rows.reverse();
        rows.truncate(200);
        Ok(rows)
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
        let superseded = app.as_ref().map(|a| a.superseded.len()).unwrap_or(0);
        let deferred = app.as_ref().map(|a| a.deferred.len()).unwrap_or(0);
        let acknowledged = app.as_ref().map(|a| a.acknowledged).unwrap_or(0);
        Ok(Status {
            pending: s.pending.len(),
            received: s.received.len(),
            cursor: s.cursor,
            applied: applied > 0 && applied == s.received.len() && deferred == 0,
            applied_revisions: applied - superseded - deferred,
            acknowledged_revisions: acknowledged
                - app
                    .as_ref()
                    .map(|a| a.superseded.range(..acknowledged).count())
                    .unwrap_or(0),
            superseded_revisions: superseded,
            deferred_revisions: deferred,
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
        let mut remaining = snapshot;
        let mut temporary_notes = BTreeSet::new();
        while !remaining.is_empty() {
            let ready = remaining.iter().position(|(file, _)| {
                !state.local.heads.keys().any(|id| {
                    *id != file.note
                        && state
                            .local
                            .head(*id)
                            .is_some_and(|r| r.content.is_some() && r.path == file.path)
                })
            });
            let (file, bytes) = if let Some(index) = ready {
                remaining.remove(index)
            } else {
                // Break a cycle without touching the uploader's source files.
                // The original captured revision bytes travel with this step.
                let (mut file, bytes) = remaining[0].clone();
                if !temporary_notes.insert(file.note) {
                    return Err(Error::Conflict);
                }
                let current = state.local.head(file.note).ok_or(Error::Conflict)?;
                if current.path == file.path {
                    return Err(Error::Conflict);
                }
                file.path = file
                    .path
                    .parent()
                    .unwrap_or_else(notes_model::RelPath::root)
                    .join(&format!("sync-move-{}.md", Uuid::new_v4()))
                    .map_err(|_| Error::Invalid)?;
                (file, bytes)
            };
            let attachments = notes_core::sync::capture_attachments(
                &state.source,
                &self.dir.join("core"),
                &file.path,
                &bytes,
            )
            .map_err(|_| Error::ApplicationBlocked)?;
            if state.local.head(file.note).is_some_and(|r| {
                r.path == file.path
                    && r.content.as_ref() == Some(&file.content)
                    && state.attachments_at(r.id) == attachments
            }) {
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
                attachments,
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
    /// Explicit deletion confirmation names the exact observed live head.
    pub fn stage_delete(&self, note: notes_model::NoteId, expected: Uuid) -> Result<Uuid> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let mut state = self.load()?;
        if state.mode != Mode::Upload {
            return Err(Error::Invalid);
        }
        let head = state.local.head(note).ok_or(Error::Invalid)?.clone();
        if head.id != expected || head.content.is_none() {
            return Err(Error::Conflict);
        }
        let inventory = notes_core::sync::inventory(&state.source, &self.dir.join("core"))
            .map_err(|_| Error::ApplicationBlocked)?;
        if inventory.iter().any(|f| f.note == note) {
            return Err(Error::Conflict);
        }
        let revision = Revision::new(note, [expected].into(), state.device, head.path, None);
        state
            .local
            .commit(revision.clone(), Some(expected))
            .map_err(|_| Error::Conflict)?;
        state.pending.push(Publication {
            attachments: vec![],
            workspace: state.local.workspace,
            expected: Some(expected),
            revision: revision.clone(),
            content_base64: None,
            branches: vec![],
        });
        self.save(&state, false)?;
        Ok(revision.id)
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
            if state.mode == Mode::Receive
                && state
                    .capture
                    .as_ref()
                    .is_some_and(|c| !c.publishable && p.revision.id == c.branch)
            {
                return Err(Error::Conflict);
            }
            transport.publish(p)?;
            state.pending.remove(0);
            self.save(&state, false)?;
        }
        self.fetch_into(&mut state, transport)
    }
    /// Explicit recovery from an older server backup. The server must contain
    /// an exact prefix of this unscoped queue's retained publications. Ordinary
    /// transfer never resets cursors or elects a replacement history.
    pub fn recover_server(&self, transport: &mut impl Transport) -> Result<usize> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        // Scoped cursors include invisible publications: they cannot prove a
        // complete prefix and must use an unscoped recovery queue instead.
        if state.endpoint.scope.is_some() {
            return Err(Error::Invalid);
        }
        let cursor = Self::recovery_prefix(&state, transport)?;
        let end = (cursor + 20).min(state.received.len());
        for publication in &state.received[cursor..end] {
            transport.publish(publication)?;
        }
        if Self::recovery_prefix(&state, transport)? != end {
            return Err(Error::Conflict);
        }
        // Re-send only the latest previously acknowledged receipts, never intermediate
        // revisions that could regress a server's surviving acknowledgment.
        // Retrying is safe even when the previous response was lost.
        if end == state.received.len() {
            if let Some(app) = self.application(&state)? {
                let mut receipts = BTreeMap::new();
                for (index, publication) in state.received[..app.acknowledged].iter().enumerate() {
                    if !app.superseded.contains(&index) {
                        receipts.insert(publication.revision.note, publication.revision.id);
                    }
                }
                for revision in receipts.into_values() {
                    transport.acknowledge(&notes_sync::transfer::ApplicationAcknowledgment {
                        workspace: state.local.workspace,
                        device: state.device,
                        revision,
                    })?;
                }
            }
        }
        Ok(end - cursor)
    }

    fn recovery_prefix(state: &State, transport: &mut impl Transport) -> Result<usize> {
        let mut cursor = 0;
        loop {
            let page = transport.page(cursor)?;
            if page.workspace != state.local.workspace
                || page.revisions.len() > 20
                || page.next_cursor != cursor + page.revisions.len()
                || page.next_cursor > state.received.len()
                || (page.has_more && page.next_cursor == cursor)
            {
                return Err(Error::Protocol);
            }
            for revision in &page.revisions {
                let retained = &state.received[cursor];
                if *revision != retained.revision || transport.fetch(revision.id)? != *retained {
                    return Err(Error::Conflict);
                }
                cursor += 1;
            }
            if !page.has_more {
                break;
            }
        }
        Ok(cursor)
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
            || page.next_cursor < state.cursor + page.revisions.len()
            || page.next_cursor > state.cursor.saturating_add(20)
            || (state.endpoint.scope.is_none()
                && page.next_cursor != state.cursor + page.revisions.len())
            || (page.has_more && page.next_cursor == state.cursor)
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
        if state.mode == Mode::Receive {
            let capture = state.capture.as_ref().ok_or(Error::Invalid)?;
            let app = self.application(&state)?.ok_or(Error::Invalid)?;
            if app.intent.is_some() || app.resolution_intent.is_some() || app.asset_intent.is_some()
            {
                return Err(Error::Invalid);
            }
            if capture.note != a.note || !state.local.is_ancestor(capture.branch, local) {
                return Err(Error::Conflict);
            }
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
                        attachments: p.attachments.clone(),
                        revision: p.revision.clone(),
                        content_base64: p.content_base64.clone(),
                    }))
            {
                if !incoming.revisions.contains_key(&b.revision.id) && seen.insert(b.revision.id) {
                    branches.push(b);
                }
            }
        }
        let data = state
            .capture
            .as_ref()
            .map(|c| c.core_data.clone())
            .unwrap_or_else(|| self.dir.join("core"));
        let attachments = match &bytes {
            Some(raw) => {
                notes_core::sync::capture_attachments(&state.source, &data, &revision.path, raw)
                    .map_err(|_| Error::ApplicationBlocked)?
            }
            None => vec![],
        };
        let publication = Publication {
            attachments,
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
        if let Some(capture) = &mut state.capture {
            capture.publishable = false;
        }
        state.pending.retain(|p| p.revision.note != a.note);
        state.pending.push(publication);
        self.save(&state, false)?;
        Ok(revision.id)
    }
    /// Recover a retained attachment into private state without replacing a file.
    pub fn export_attachment(&self, id: Uuid, path: &notes_model::RelPath) -> Result<PathBuf> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        let asset = state
            .attachments_at(id)
            .into_iter()
            .find(|a| a.path == *path)
            .ok_or(Error::Invalid)?;
        let bytes = asset.bytes().map_err(|_| Error::Invalid)?;
        let mut temp = tempfile::NamedTempFile::new_in(&self.dir).map_err(|_| Error::Storage)?;
        temp.write_all(&bytes).map_err(|_| Error::Storage)?;
        temp.as_file().sync_all().map_err(|_| Error::Storage)?;
        let destination = self.dir.join(format!(
            "attachment-{id}-{}.bin",
            asset.hash.to_string().replace(':', "-")
        ));
        temp.persist_noclobber(&destination)
            .map_err(|_| Error::Storage)?;
        Ok(destination)
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
                        attachments: b.attachments.clone(),
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
    #[serde(default)]
    deleted: bool,
    revision: Uuid,
    path: notes_model::RelPath,
    local: notes_core::sync::Applied,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Application {
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    assets: std::collections::BTreeMap<notes_model::RelPath, notes_model::BaseRev>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    asset_intent: Option<(Uuid, notes_model::RelPath)>,
    schema: u32,
    core_data: PathBuf,
    next: usize,
    notes: std::collections::BTreeMap<notes_model::NoteId, ApplicationReceipt>,
    intent: Option<Uuid>,
    #[serde(default)]
    acknowledged: usize,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    superseded: BTreeSet<usize>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    deferred: BTreeSet<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resolution_intent: Option<Uuid>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pairing {
    application: Application,
    created: std::collections::BTreeMap<notes_model::NoteId, ApplicationReceipt>,
}
#[derive(Serialize)]
pub struct PairingPreview {
    pub confirmation: String,
    pub attachment_conflicts: Vec<notes_model::RelPath>,
    pub actions: Vec<notes_sync::PairingAction>,
}
impl Store {
    fn application(&self, state: &State) -> Result<Option<Application>> {
        let path = self.dir.join("application.json");
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return match &state.pairing {
                    Some(p) => Self::validate_application(state, p.application.clone()),
                    None => Ok(None),
                };
            }
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
        Self::validate_application(state, app)
    }
    fn validate_application(state: &State, app: Application) -> Result<Option<Application>> {
        if app.assets.len() > 10_000
            || app.asset_intent.as_ref().is_some_and(|(id, path)| {
                !state
                    .received
                    .iter()
                    .any(|p| p.revision.id == *id && p.attachments.iter().any(|a| a.path == *path))
            })
            || app.schema != 1
            || !app.core_data.is_absolute()
            || app.next > state.received.len()
            || app.acknowledged > app.next
            || app.intent.is_some_and(|id| {
                state
                    .received
                    .get(app.deferred.first().copied().unwrap_or(app.next))
                    .is_none_or(|p| p.revision.id != id)
            })
        {
            return Err(Error::Invalid);
        }
        let mut latest = std::collections::BTreeMap::new();
        for (i, p) in state.received[..app.next].iter().enumerate() {
            if !app.deferred.contains(&i) && !app.superseded.contains(&i) {
                latest.insert(p.revision.note, &p.revision);
            }
        }
        if latest.len() != app.notes.len()
            || latest.iter().any(|(id, r)| {
                app.notes.get(id).is_none_or(|n| {
                    n.revision != r.id
                        || n.path != r.path
                        || n.deleted != r.content.is_none()
                        || (!n.deleted && r.content.as_ref() != Some(&n.local.base_rev.hash))
                })
            })
        {
            return Err(Error::Invalid);
        }
        if app.superseded.iter().any(|i| *i >= app.next)
            || app
                .deferred
                .iter()
                .any(|i| *i >= app.next || *i < app.acknowledged || app.superseded.contains(i))
        {
            return Err(Error::Invalid);
        }
        if !app.superseded.is_empty() {
            let incoming = Self::incoming(state)?;
            for i in &app.superseded {
                let r = &state.received[*i].revision;
                if app
                    .notes
                    .get(&r.note)
                    .is_none_or(|n| n.revision == r.id || !incoming.is_ancestor(r.id, n.revision))
                {
                    return Err(Error::Invalid);
                }
            }
        }
        if let Some(id) = app.resolution_intent {
            if app.intent.is_some()
                || state.capture.is_none()
                || !state.received[app.next..]
                    .iter()
                    .any(|p| p.revision.id == id)
            {
                return Err(Error::Invalid);
            }
        }
        Ok(Some(app))
    }
    fn apply_assets(&self, state: &State, app: &mut Application, p: &Publication) -> Result<()> {
        if app
            .asset_intent
            .as_ref()
            .is_some_and(|(id, _)| *id != p.revision.id)
        {
            return Err(Error::Invalid);
        }
        let mut assets: Vec<_> = p.attachments.iter().collect();
        assets.sort_by_key(|a| {
            app.asset_intent
                .as_ref()
                .is_none_or(|(_, path)| *path != a.path)
        });
        for asset in assets {
            if !app.assets.contains_key(&asset.path) && app.assets.len() >= 10_000 {
                return Err(Error::Limit);
            }
            let expected = app.assets.get(&asset.path).cloned();
            let data = app.core_data.clone();
            let retry = app
                .asset_intent
                .as_ref()
                .is_some_and(|(id, path)| *id == p.revision.id && *path == asset.path);
            let applied = notes_core::sync::apply_attachment(
                &state.source,
                &data,
                asset,
                expected.as_ref(),
                retry,
                || {
                    app.asset_intent = Some((p.revision.id, asset.path.clone()));
                    self.save_application(app)
                        .map_err(|_| notes_model::CoreError::Internal {
                            message: "could not persist attachment intent".into(),
                        })
                },
            )
            .map_err(|_| Error::ApplicationBlocked)?;
            app.assets.insert(asset.path.clone(), applied);
            app.asset_intent = None;
            self.save_application(app)?;
        }
        Ok(())
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
        self.apply_using(
            core_data,
            false,
            |root, path, bytes, expected, retry, prepare| {
                notes_core::sync::apply_received(
                    root, core_data, path, bytes, expected, retry, prepare,
                )
            },
        )
    }
    pub fn apply_effects(&self, core_data: &Path) -> Result<usize> {
        self.apply_using(
            core_data,
            true,
            |root, path, bytes, expected, retry, prepare| {
                notes_core::sync::apply_received(
                    root, core_data, path, bytes, expected, retry, prepare,
                )
            },
        )
    }
    fn apply_using(
        &self,
        core_data: &Path,
        effects: bool,
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
            assets: Default::default(),
            asset_intent: None,
            schema: 1,
            core_data: core_data.clone(),
            next: 0,
            notes: Default::default(),
            intent: None,
            acknowledged: 0,
            superseded: BTreeSet::new(),
            deferred: BTreeSet::new(),
            resolution_intent: None,
        });
        if app.core_data != core_data || app.resolution_intent.is_some() {
            return Err(Error::Invalid);
        }
        if let Some(c) = &state.capture {
            let incoming = Self::incoming(&state)?;
            if app
                .notes
                .get(&c.note)
                .is_none_or(|n| !incoming.is_ancestor(c.branch, n.revision))
            {
                return Err(Error::Conflict);
            }
        }
        let mut count = 0;
        let pending: Vec<_> = app
            .deferred
            .iter()
            .copied()
            .chain(app.next..state.received.len())
            .take(20)
            .collect();
        for index in pending {
            let p = &state.received[index];
            if !effects && p.revision.content.is_none() {
                return Err(Error::UnsupportedApplication);
            }
            let previous = app.notes.get(&p.revision.note).cloned();
            if previous.as_ref().map(|n| n.revision) != p.expected {
                return Err(Error::Conflict);
            }
            if !effects && previous.as_ref().is_some_and(|n| n.path != p.revision.path) {
                return Err(Error::UnsupportedApplication);
            }
            if !p.attachments.is_empty() || app.asset_intent.is_some() {
                if !effects {
                    return Err(Error::UnsupportedApplication);
                }
                self.apply_assets(&state, &mut app, p)?;
            }
            let bytes = content(p).map_err(|_| Error::Invalid)?;
            let retry = app.intent == Some(p.revision.id);
            let mut prepare = || {
                app.intent = Some(p.revision.id);
                self.save_application(&app)
                    .map_err(|_| notes_model::CoreError::Internal {
                        message: "could not persist application intent".into(),
                    })
            };
            let applied = if effects
                && previous
                    .as_ref()
                    .is_some_and(|n| p.revision.content.is_none() || n.path != p.revision.path)
            {
                let before = previous.as_ref().ok_or(Error::Invalid)?;
                notes_core::sync::apply_resolution_effect(
                    &state.source,
                    &core_data,
                    &before.path,
                    &p.revision.path,
                    p.revision.content.as_ref().map(|_| bytes.as_slice()),
                    &before.local,
                    retry || before.deleted,
                    &mut prepare,
                )
            } else {
                execute(
                    &state.source,
                    &p.revision.path,
                    &bytes,
                    previous
                        .as_ref()
                        .filter(|n| !n.deleted)
                        .or_else(|| {
                            if previous.is_some() {
                                return None;
                            }
                            state
                                .pairing
                                .as_ref()
                                .and_then(|pairing| pairing.created.get(&p.revision.note))
                        })
                        .map(|n| &n.local),
                    retry,
                    &mut prepare,
                )
                .map(Some)
            }
            .map_err(|e| match e {
                notes_model::CoreError::LockTimeout => Error::Busy,
                _ => Error::ApplicationBlocked,
            })?;
            app.notes.insert(
                p.revision.note,
                ApplicationReceipt {
                    deleted: applied.is_none(),
                    revision: p.revision.id,
                    path: p.revision.path.clone(),
                    local: applied.unwrap_or_else(|| previous.as_ref().unwrap().local.clone()),
                },
            );
            if !app.deferred.remove(&index) {
                app.next += 1;
            }
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
        let mut scanned = 0;
        while app.acknowledged < app.next && scanned < 20 {
            scanned += 1;
            if app.deferred.contains(&app.acknowledged) {
                break;
            }
            if app.superseded.contains(&app.acknowledged) {
                app.acknowledged += 1;
                self.save_application(&app)?;
                continue;
            }
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
        let outcome = self.apply_using(
            &data,
            false,
            |root, path, bytes, expected, retry, prepare| {
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
            },
        );
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

impl Store {
    /// Capture one saved receiver edit. Application history remains untouched.
    pub fn capture_receiver_conflict(
        &self,
        core_data: &Path,
        note: notes_model::NoteId,
    ) -> Result<Uuid> {
        self.capture_receiver_change(core_data, note, false)
    }
    pub fn capture_receiver_edit(
        &self,
        core_data: &Path,
        note: notes_model::NoteId,
    ) -> Result<Uuid> {
        self.capture_receiver_change(core_data, note, true)
    }
    fn capture_receiver_change(
        &self,
        core_data: &Path,
        note: notes_model::NoteId,
        publishable: bool,
    ) -> Result<Uuid> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let mut state = self.load()?;
        if state.mode != Mode::Receive || !state.pending.is_empty() {
            return Err(Error::Invalid);
        }
        let app = self.application(&state)?.ok_or(Error::Invalid)?;
        let data = fs::canonicalize(core_data).map_err(|_| Error::Invalid)?;
        if data != app.core_data
            || app.intent.is_some()
            || app.resolution_intent.is_some()
            || app.asset_intent.is_some()
        {
            return Err(Error::Invalid);
        }
        let incoming = Self::incoming(&state)?;
        if let Some(c) = &state.capture {
            if app
                .notes
                .get(&c.note)
                .is_none_or(|n| !incoming.is_ancestor(c.branch, n.revision))
            {
                return Err(Error::Conflict);
            }
        }
        let previous = app.notes.get(&note).ok_or(Error::Invalid)?;
        let remote = incoming.head(note).ok_or(Error::Invalid)?;
        if previous.deleted
            || (publishable
                && (remote.id != previous.revision
                    || app.next != state.received.len()
                    || !app.deferred.is_empty()))
            || (!publishable
                && (remote.id == previous.revision
                    || !incoming.is_ancestor(previous.revision, remote.id)))
        {
            return Err(Error::Conflict);
        }
        let (local, bytes) = notes_core::sync::capture_conflict(
            &state.source,
            &data,
            &previous.path,
            &previous.local,
        )
        .map_err(|_| Error::ApplicationBlocked)?;
        let attachments =
            notes_core::sync::capture_attachments(&state.source, &data, &previous.path, &bytes)
                .map_err(|_| Error::ApplicationBlocked)?;
        if local.base_rev.hash == previous.local.base_rev.hash
            && attachments == state.attachments_at(previous.revision)
        {
            return Err(Error::Conflict);
        }
        let revision = Revision::new(
            note,
            [previous.revision].into(),
            state.device,
            previous.path.clone(),
            Some(local.base_rev.hash.clone()),
        );
        state.local = incoming;
        state.local.heads.insert(note, previous.revision);
        state
            .local
            .commit(revision.clone(), Some(previous.revision))
            .map_err(|_| Error::Conflict)?;
        state.pending.push(Publication {
            attachments,
            workspace: state.local.workspace,
            expected: Some(previous.revision),
            revision: revision.clone(),
            content_base64: Some(STANDARD.encode(bytes)),
            branches: vec![],
        });
        state.capture = Some(ReceiverCapture {
            publishable,
            core_data: data,
            note,
            applied: previous.revision,
            branch: revision.id,
            path: previous.path.clone(),
            local,
        });
        self.save(&state, false)?;
        Ok(revision.id)
    }

    /// Preserve a newer saved edit as a child of the previous captured branch.
    /// Publish a prepared resolution first so its history remains recoverable.
    pub fn recapture_receiver_conflict(&self, core_data: &Path) -> Result<Uuid> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let mut state = self.load()?;
        if state.mode != Mode::Receive {
            return Err(Error::Invalid);
        }
        let capture = state.capture.clone().ok_or(Error::Invalid)?;
        if capture.publishable && !state.pending.is_empty() {
            return Err(Error::Busy);
        }
        let app = self.application(&state)?.ok_or(Error::Invalid)?;
        let data = fs::canonicalize(core_data).map_err(|_| Error::Invalid)?;
        if data != app.core_data
            || data != capture.core_data
            || app.intent.is_some()
            || app.resolution_intent.is_some()
            || app.asset_intent.is_some()
            || app
                .notes
                .get(&capture.note)
                .is_none_or(|n| n.revision != capture.applied)
        {
            return Err(Error::Invalid);
        }
        if state.pending.len() > 1
            || state
                .pending
                .first()
                .is_some_and(|p| p.revision.id != capture.branch)
        {
            return Err(Error::Conflict);
        }
        let incoming = Self::incoming(&state)?;
        if state.pending.is_empty() && !incoming.revisions.contains_key(&capture.branch) {
            return Err(Error::Conflict);
        }
        let (local, bytes) =
            notes_core::sync::capture_conflict(&state.source, &data, &capture.path, &capture.local)
                .map_err(|_| Error::ApplicationBlocked)?;
        let attachments =
            notes_core::sync::capture_attachments(&state.source, &data, &capture.path, &bytes)
                .map_err(|_| Error::ApplicationBlocked)?;
        if local.base_rev.hash == capture.local.base_rev.hash
            && attachments == state.attachments_at(capture.branch)
        {
            return Err(Error::Conflict);
        }
        let revision = Revision::new(
            capture.note,
            [capture.branch].into(),
            state.device,
            capture.path.clone(),
            Some(local.base_rev.hash.clone()),
        );
        let mut branches = vec![];
        if let Some(previous) = state.pending.pop() {
            branches = previous.branches;
            branches.push(notes_sync::transfer::Branch {
                attachments: previous.attachments,
                revision: previous.revision,
                content_base64: previous.content_base64,
            });
        }
        let publication = Publication {
            attachments,
            workspace: state.local.workspace,
            expected: Some(capture.branch),
            revision: revision.clone(),
            content_base64: Some(STANDARD.encode(bytes)),
            branches,
        };
        // Reserve one branch slot for this capture when resolving it later.
        if publication.branches.len() >= 20 {
            return Err(Error::Limit);
        }
        notes_sync::transfer::payload_size(&publication).map_err(|_| Error::Limit)?;
        state.local.heads.insert(capture.note, capture.branch);
        state
            .local
            .commit(revision.clone(), Some(capture.branch))
            .map_err(|_| Error::Conflict)?;
        state.pending.push(publication);
        state.capture = Some(ReceiverCapture {
            branch: revision.id,
            local,
            ..capture
        });
        self.save(&state, false)?;
        Ok(revision.id)
    }

    /// Apply only a published receiver resolution. Superseded intermediate
    /// publications get no source write and no application acknowledgment.
    pub fn apply_resolution(&self, core_data: &Path, id: Uuid) -> Result<usize> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        if state.mode != Mode::Receive || !state.pending.is_empty() {
            return Err(Error::Invalid);
        }
        let capture = state.capture.as_ref().ok_or(Error::Invalid)?;
        let mut app = self.application(&state)?.ok_or(Error::Invalid)?;
        let data = fs::canonicalize(core_data).map_err(|_| Error::Invalid)?;
        if data != capture.core_data
            || data != app.core_data
            || app.intent.is_some()
            || app.resolution_intent.is_some_and(|r| r != id)
        {
            return Err(Error::Invalid);
        }
        let incoming = Self::incoming(&state)?;
        let index = state
            .received
            .iter()
            .position(|p| p.revision.id == id)
            .ok_or(Error::Invalid)?;
        let p = &state.received[index];
        if p.revision.note != capture.note
            || state.local.heads.get(&capture.note) != Some(&id)
            || !incoming.is_ancestor(capture.branch, id)
        {
            return Err(Error::Conflict);
        }
        let previous = app.notes.get(&capture.note).ok_or(Error::Invalid)?;
        if index < app.next && incoming.is_ancestor(id, previous.revision) {
            return Ok(0);
        }
        if previous.revision != capture.applied || index < app.next {
            return Err(Error::Conflict);
        }
        let pending: Vec<_> = app
            .deferred
            .iter()
            .copied()
            .chain(app.next..=index)
            .collect();
        if pending.iter().any(|i| {
            let q = &state.received[*i];
            q.revision.note == capture.note && !incoming.is_ancestor(q.revision.id, id)
        }) {
            return Err(Error::Conflict);
        }
        self.apply_assets(&state, &mut app, p)?;
        let bytes = content(p).map_err(|_| Error::Invalid)?;
        let retry = app.resolution_intent == Some(id);
        let applied = notes_core::sync::apply_resolution_effect(
            &state.source,
            &data,
            &capture.path,
            &p.revision.path,
            p.revision.content.as_ref().map(|_| bytes.as_slice()),
            &capture.local,
            retry,
            || {
                app.resolution_intent = Some(id);
                self.save_application(&app)
                    .map_err(|_| notes_model::CoreError::Internal {
                        message: "could not persist receiver resolution intent".into(),
                    })
            },
        )
        .map_err(|_| Error::ApplicationBlocked)?;
        for i in pending.into_iter().filter(|i| *i != index) {
            if state.received[i].revision.note == capture.note {
                app.deferred.remove(&i);
                app.superseded.insert(i);
            } else {
                app.deferred.insert(i);
            }
        }
        app.next = index + 1;
        app.notes.insert(
            capture.note,
            ApplicationReceipt {
                deleted: applied.is_none(),
                revision: id,
                path: p.revision.path.clone(),
                local: applied.unwrap_or_else(|| capture.local.clone()),
            },
        );
        app.resolution_intent = None;
        self.save_application(&app)?;
        Ok(1)
    }
}

type PairingFiles = Vec<(notes_sync::File, Vec<u8>)>;

impl Store {
    fn pairing_snapshot(
        &self,
        state: &State,
        data: &Path,
    ) -> Result<(PairingPreview, PairingFiles)> {
        if state.mode != Mode::Receive
            || state.pairing.is_some()
            || state.capture.is_some()
            || !state.pending.is_empty()
            || self.dir.join("application.json").exists()
        {
            return Err(Error::Invalid);
        }
        let local = notes_core::sync::capture(&state.source, data)
            .map_err(|_| Error::ApplicationBlocked)?;
        let incoming = Self::incoming(state)?;
        let remote: Vec<_> = incoming
            .heads
            .keys()
            .filter_map(|id| {
                let r = incoming.head(*id)?;
                Some(notes_sync::File {
                    note: r.note,
                    path: r.path.clone(),
                    content: r.content.clone()?,
                })
            })
            .collect();
        let actions = notes_sync::pair(
            notes_sync::PairingMode::Reconcile,
            &local.iter().map(|(f, _)| f.clone()).collect::<Vec<_>>(),
            &remote,
        )
        .map_err(|_| Error::Conflict)?;
        let mut asset_hashes = vec![];
        let mut attachment_conflicts = vec![];
        let mut total_assets = 0;
        for (file, bytes) in &local {
            let assets =
                notes_core::sync::capture_attachments(&state.source, data, &file.path, bytes)
                    .map_err(|_| Error::ApplicationBlocked)?;
            if let Some(remote) = incoming
                .heads
                .keys()
                .filter_map(|id| incoming.head(*id))
                .find(|r| r.path == file.path && r.content.as_ref() == Some(&file.content))
            {
                if state.attachments_at(remote.id) != assets {
                    attachment_conflicts.push(file.path.clone());
                }
            }
            for a in assets {
                total_assets += a.content_base64.len();
                if total_assets > MAX_STATE {
                    return Err(Error::Limit);
                }
                asset_hashes.push((file.note, a.path, a.hash));
            }
        }
        let encoded = serde_json::to_vec(&(
            state.local.workspace,
            state.cursor,
            &state.endpoint,
            &state.source,
            data,
            &actions,
            &asset_hashes,
        ))
        .map_err(|_| Error::Invalid)?;
        Ok((
            PairingPreview {
                confirmation: blake3::hash(&encoded).to_hex().to_string(),
                attachment_conflicts,
                actions,
            },
            local,
        ))
    }
    pub fn preview_pairing(&self, data: &Path) -> Result<PairingPreview> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        notes_core::sync::validate_state_location(&[&state.source], data)
            .map_err(|_| Error::Invalid)?;
        fs::create_dir_all(data).map_err(|_| Error::Storage)?;
        let data = fs::canonicalize(data).map_err(|_| Error::Invalid)?;
        Ok(self.pairing_snapshot(&state, &data)?.0)
    }
    /// Confirm an exact cached preview after verifying that the server has no
    /// unseen entries. Equal bytes link identities; differing bytes never do.
    pub fn confirm_pairing(
        &self,
        data: &Path,
        confirmation: &str,
        transport: &mut impl Transport,
    ) -> Result<()> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let mut state = self.load()?;
        let data = fs::canonicalize(data).map_err(|_| Error::Invalid)?;
        let (preview, snapshot) = self.pairing_snapshot(&state, &data)?;
        if !preview.attachment_conflicts.is_empty()
            || preview.confirmation != confirmation
            || preview
                .actions
                .iter()
                .any(|a| matches!(a, notes_sync::PairingAction::Conflict { .. }))
        {
            return Err(Error::Conflict);
        }
        let page = transport.page(state.cursor)?;
        if page.workspace != state.local.workspace
            || page.next_cursor != state.cursor
            || page.has_more
            || !page.revisions.is_empty()
        {
            return Err(Error::Conflict);
        }
        let incoming = Self::incoming(&state)?;
        let mut observed = std::collections::BTreeMap::new();
        for (f, bytes) in &snapshot {
            let expected = notes_core::sync::Applied {
                note_id: f.note,
                base_rev: notes_model::BaseRev {
                    hash: f.content.clone(),
                    size: bytes.len() as u64,
                    mtime_ns: 0,
                },
            };
            let (actual, raw) =
                notes_core::sync::capture_conflict(&state.source, &data, &f.path, &expected)
                    .map_err(|_| Error::ApplicationBlocked)?;
            if raw != *bytes {
                return Err(Error::Conflict);
            }
            observed.insert(f.note, actual);
        }
        let mut app = Application {
            assets: Default::default(),
            asset_intent: None,
            schema: 1,
            core_data: data,
            next: state.received.len(),
            notes: Default::default(),
            intent: None,
            acknowledged: 0,
            superseded: Default::default(),
            deferred: Default::default(),
            resolution_intent: None,
        };
        let mut created = std::collections::BTreeMap::new();
        state.local = incoming.clone();
        for action in preview.actions {
            match action {
                notes_sync::PairingAction::Link { local, remote } => {
                    let head = incoming.head(remote.note).ok_or(Error::Invalid)?;
                    for asset in state.attachments_at(head.id) {
                        let base = notes_core::sync::confirm_attachment(
                            &state.source,
                            &app.core_data,
                            &asset,
                        )
                        .map_err(|_| Error::ApplicationBlocked)?;
                        app.assets.insert(asset.path, base);
                    }
                    app.notes.insert(
                        remote.note,
                        ApplicationReceipt {
                            deleted: false,
                            revision: head.id,
                            path: remote.path,
                            local: observed[&local.note].clone(),
                        },
                    );
                }
                notes_sync::PairingAction::Upload { local } => {
                    let bytes = snapshot
                        .iter()
                        .find(|(f, _)| f.note == local.note)
                        .ok_or(Error::Invalid)?
                        .1
                        .clone();
                    let revision = Revision::new(
                        local.note,
                        Default::default(),
                        state.device,
                        local.path.clone(),
                        Some(local.content),
                    );
                    state
                        .local
                        .commit(revision.clone(), None)
                        .map_err(|_| Error::Conflict)?;
                    created.insert(
                        local.note,
                        ApplicationReceipt {
                            deleted: false,
                            revision: revision.id,
                            path: local.path,
                            local: observed[&local.note].clone(),
                        },
                    );
                    let attachments = notes_core::sync::capture_attachments(
                        &state.source,
                        &app.core_data,
                        &revision.path,
                        &bytes,
                    )
                    .map_err(|_| Error::ApplicationBlocked)?;
                    state.pending.push(Publication {
                        attachments,
                        workspace: state.local.workspace,
                        expected: None,
                        revision,
                        content_base64: Some(STANDARD.encode(bytes)),
                        branches: vec![],
                    });
                }
                notes_sync::PairingAction::Download { .. } => {}
                notes_sync::PairingAction::Conflict { .. } => return Err(Error::Conflict),
            }
        }
        for (index, p) in state.received.iter().enumerate() {
            match app.notes.get(&p.revision.note) {
                Some(receipt) if receipt.revision != p.revision.id => {
                    app.superseded.insert(index);
                }
                Some(_) => {}
                None => {
                    app.deferred.insert(index);
                }
            }
        }
        Self::validate_application(&state, app.clone())?;
        state.pairing = Some(Pairing {
            application: app,
            created,
        });
        self.save(&state, false)
    }
}

impl Store {
    /// Capture at most one already applied, same-path saved edit. Captures are
    /// opt-in; new paths and missing files never imply creation or deletion.
    pub fn stage_receiver_edits(&self) -> Result<usize> {
        let (state, app) = {
            let lock = self.lock()?;
            let _guard = lock.try_read().map_err(|_| Error::Busy)?;
            let state = self.load()?;
            if state.mode != Mode::Receive {
                return Err(Error::Invalid);
            }
            let app = self.application(&state)?;
            (state, app)
        };
        let Some(app) = app else { return Ok(0) };
        if !state.pending.is_empty() {
            return Ok(0);
        }
        let incoming = Self::incoming(&state)?;
        if let Some(c) = &state.capture {
            let completed = app
                .notes
                .get(&c.note)
                .is_some_and(|n| incoming.is_ancestor(c.branch, n.revision));
            if !completed {
                if !c.publishable {
                    return Err(Error::Conflict);
                }
                if !incoming.revisions.contains_key(&c.branch) {
                    // Publication was accepted beyond this bounded cache page.
                    // Keep fetching before attempting another capture.
                    return Ok(0);
                }
                let (local, bytes) = notes_core::sync::capture_conflict(
                    &state.source,
                    &app.core_data,
                    &c.path,
                    &c.local,
                )
                .map_err(|_| Error::ApplicationBlocked)?;
                let assets = notes_core::sync::capture_attachments(
                    &state.source,
                    &app.core_data,
                    &c.path,
                    &bytes,
                )
                .map_err(|_| Error::ApplicationBlocked)?;
                if local.base_rev.hash == c.local.base_rev.hash
                    && assets == state.attachments_at(c.branch)
                {
                    self.confirm_receiver_edit()?;
                    return Ok(0);
                }
                self.recapture_receiver_conflict(&app.core_data)?;
                return Ok(1);
            }
        }
        if app.next != state.received.len() || !app.deferred.is_empty() {
            return Ok(0);
        }
        let inspection = self.dir.join("inspection");
        let files = notes_core::sync::capture(&state.source, &inspection)
            .map_err(|_| Error::ApplicationBlocked)?;
        for (note, previous) in &app.notes {
            if previous.deleted {
                continue;
            }
            let Some((file, bytes)) = files.iter().find(|(f, _)| f.path == previous.path) else {
                continue;
            };
            let assets = notes_core::sync::capture_attachments(
                &state.source,
                &inspection,
                &file.path,
                bytes,
            )
            .map_err(|_| Error::ApplicationBlocked)?;
            if file.content != previous.local.base_rev.hash
                || assets != state.attachments_at(previous.revision)
            {
                self.capture_receiver_edit(&app.core_data, *note)?;
                return Ok(1);
            }
        }
        Ok(0)
    }

    /// Record a published capture already present in the source. This performs
    /// guarded reads only; a later saved edit is never replaced by older bytes.
    pub fn confirm_receiver_edit(&self) -> Result<usize> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let state = self.load()?;
        if state.mode != Mode::Receive {
            return Err(Error::Invalid);
        }
        let Some(c) = state.capture.as_ref().filter(|c| c.publishable) else {
            return Ok(0);
        };
        let mut app = self.application(&state)?.ok_or(Error::Invalid)?;
        let incoming = Self::incoming(&state)?;
        let previous = app.notes.get(&c.note).ok_or(Error::Invalid)?;
        if incoming.is_ancestor(c.branch, previous.revision) {
            return Ok(0);
        }
        if !state.pending.is_empty() {
            return Ok(0);
        }
        let Some(index) = state
            .received
            .iter()
            .position(|p| p.revision.id == c.branch)
        else {
            return Ok(0);
        };
        if previous.revision != c.applied
            || index < app.next
            || app.intent.is_some()
            || app.asset_intent.is_some()
            || app.resolution_intent.is_some()
            || app.core_data != c.core_data
        {
            return Err(Error::Conflict);
        }
        let publication = &state.received[index];
        if state.local.revisions.get(&c.branch) != Some(&publication.revision) {
            return Err(Error::Protocol);
        }

        let (local, bytes) =
            notes_core::sync::capture_conflict(&state.source, &app.core_data, &c.path, &c.local)
                .map_err(|_| Error::ApplicationBlocked)?;
        if local.base_rev.hash != c.local.base_rev.hash
            || bytes != content(publication).map_err(|_| Error::Invalid)?
        {
            return Err(Error::ApplicationBlocked);
        }
        for asset in &publication.attachments {
            let base = notes_core::sync::confirm_attachment(&state.source, &app.core_data, asset)
                .map_err(|_| Error::ApplicationBlocked)?;
            app.assets.insert(asset.path.clone(), base);
        }
        let pending: Vec<_> = app
            .deferred
            .iter()
            .copied()
            .chain(app.next..index)
            .collect();
        for i in pending {
            let revision = &state.received[i].revision;
            if revision.note == c.note {
                if !incoming.is_ancestor(revision.id, c.branch) {
                    return Err(Error::Conflict);
                }
                app.deferred.remove(&i);
                app.superseded.insert(i);
            } else {
                app.deferred.insert(i);
            }
        }
        app.next = index + 1;
        app.notes.insert(
            c.note,
            ApplicationReceipt {
                deleted: false,
                revision: c.branch,
                path: c.path.clone(),
                local,
            },
        );
        Self::validate_application(&state, app.clone())?;
        self.save_application(&app)?;
        Ok(1)
    }
}
