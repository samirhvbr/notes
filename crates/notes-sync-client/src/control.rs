//! Desktop-owned coordination. No editor buffer access and no implicit source writes.
use crate::{
    remote::{Endpoint, Remote, Transport},
    state::{Mode, Store},
    Error, Result,
};
use notes_sync::{
    transfer::{ApplicationAcknowledgment, Publication},
    Revision,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
    time::{Duration, Instant},
};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct SyncConnection {
    pub state_dir: String,
    pub token_file: String,
    pub enabled: bool,
    pub interval_seconds: u32,
    pub allow_metered: bool,
    pub allow_battery: bool,
    #[serde(default)]
    pub capture_saved: bool,
}
#[derive(Clone, Default, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct SyncConditions {
    pub online: bool,
    pub metered: Option<bool>,
    pub charging: Option<bool>,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DevicePhase {
    Disabled,
    Offline,
    Pending,
    Syncing,
    Current,
    Conflict,
    Error,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct SyncPairRequest {
    pub state_dir: String,
    pub source: String,
    pub origin: String,
    pub workspace: String,
    pub scope: Option<String>,
    pub token_file: String,
    pub allow_private: bool,
    pub mode: String,
}
#[derive(Clone, Serialize, TS)]
#[ts(export)]
pub struct HistoryRow {
    pub id: String,
    pub note: String,
    pub path: String,
    pub deleted: bool,
    pub pending: bool,
    pub branch: bool,
    pub resolution: bool,
    pub attachments: Vec<String>,
}
impl HistoryRow {
    pub(crate) fn new(r: &Revision, pending: bool, branch: bool, attachments: Vec<String>) -> Self {
        Self {
            id: r.id.to_string(),
            note: r.note.to_string(),
            path: r.path.to_string(),
            deleted: r.content.is_none(),
            pending,
            branch,
            resolution: r.parents.len() == 2,
            attachments,
        }
    }
}
#[derive(Clone, Serialize, TS)]
#[ts(export)]
pub struct SyncConflictRow {
    pub note: String,
    pub local: String,
    pub remote: String,
    pub collision: bool,
}
#[derive(Clone, Serialize, TS)]
#[ts(export)]
pub struct DeviceSnapshot {
    pub receive: bool,
    pub connection: Option<SyncConnection>,
    pub phase: DevicePhase,
    pub reason: Option<String>,
    pub pending: u32,
    pub unapplied: u32,
    pub history: Vec<HistoryRow>,
    pub conflicts: Vec<SyncConflictRow>,
}
#[derive(Clone, Serialize, TS)]
#[ts(export)]
pub struct SyncPairPreview {
    pub confirmation: String,
    pub rows: Vec<SyncPairRow>,
    pub attachment_conflicts: Vec<String>,
}
#[derive(Clone, Serialize, TS)]
#[ts(export)]
pub struct SyncPairRow {
    pub action: String,
    pub path: String,
}

struct Session {
    connection: Option<SyncConnection>,
    conditions: SyncConditions,
    observed: Option<Instant>,
    phase: DevicePhase,
    reason: Option<String>,
    next: Instant,
    failures: u32,
    cancel: u64,
}
pub struct Controller {
    session: Mutex<Session>,
    operation: Mutex<()>,
    file: PathBuf,
    data: PathBuf,
}
impl Controller {
    pub fn new(data: &Path) -> Arc<Self> {
        let file = data.join("sync-control.json");
        let loaded = (|| -> Result<Option<SyncConnection>> {
            if !file.exists() {
                return Ok(None);
            }
            let meta = fs::symlink_metadata(&file).map_err(|_| Error::Storage)?;
            if !meta.is_file() || meta.len() > 16_384 {
                return Err(Error::Invalid);
            }
            let c: SyncConnection =
                serde_json::from_slice(&fs::read(&file).map_err(|_| Error::Storage)?)
                    .map_err(|_| Error::Invalid)?;
            validate_connection(&c)?;
            Ok(Some(c))
        })();
        let (connection, phase, reason) = match loaded {
            Ok(c) => (c, DevicePhase::Disabled, None),
            Err(e) => (None, DevicePhase::Error, Some(e.to_string())),
        };
        let controller = Arc::new(Self {
            session: Mutex::new(Session {
                connection,
                conditions: SyncConditions::default(),
                observed: None,
                phase,
                reason,
                next: Instant::now(),
                failures: 0,
                cancel: 0,
            }),
            operation: Mutex::new(()),
            file,
            data: data.to_owned(),
        });
        let weak = Arc::downgrade(&controller);
        std::thread::spawn(move || worker(weak));
        controller
    }
    fn config(&self) -> Result<SyncConnection> {
        self.session
            .lock()
            .map_err(|_| Error::Busy)?
            .connection
            .clone()
            .ok_or(Error::Invalid)
    }
    pub fn configure(&self, c: SyncConnection) -> Result<()> {
        let _guard = self.operation.try_lock().map_err(|_| Error::Busy)?;
        validate_connection(&c)?;
        let mut s = self.session.lock().map_err(|_| Error::Busy)?;
        fs::create_dir_all(&self.data).map_err(|_| Error::Storage)?;
        let mut tmp = tempfile::NamedTempFile::new_in(&self.data).map_err(|_| Error::Storage)?;
        tmp.write_all(&serde_json::to_vec(&c).map_err(|_| Error::Invalid)?)
            .map_err(|_| Error::Storage)?;
        tmp.as_file().sync_all().map_err(|_| Error::Storage)?;
        tmp.persist(&self.file).map_err(|_| Error::Storage)?;
        #[cfg(unix)]
        fs::File::open(&self.data)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Storage)?;
        s.phase = if c.enabled {
            DevicePhase::Pending
        } else {
            DevicePhase::Disabled
        };
        s.connection = Some(c);
        s.reason = None;
        s.next = Instant::now();
        s.failures = 0;
        Ok(())
    }
    /// Pause without waiting for an in-flight HTTP request. The request already
    /// sent may finish; the next request checks this cancellation generation.
    pub fn pause(&self) -> Result<()> {
        let mut s = self.session.lock().map_err(|_| Error::Busy)?;
        let mut c = s.connection.clone().ok_or(Error::Invalid)?;
        c.enabled = false;
        let mut tmp = tempfile::NamedTempFile::new_in(&self.data).map_err(|_| Error::Storage)?;
        tmp.write_all(&serde_json::to_vec(&c).map_err(|_| Error::Storage)?)
            .map_err(|_| Error::Storage)?;
        tmp.as_file().sync_all().map_err(|_| Error::Storage)?;
        tmp.persist(&self.file).map_err(|_| Error::Storage)?;
        #[cfg(unix)]
        fs::File::open(&self.data)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Storage)?;
        s.connection = Some(c);
        s.cancel = s.cancel.wrapping_add(1);
        s.phase = DevicePhase::Disabled;
        s.reason = None;
        Ok(())
    }
    pub fn conditions(&self, c: SyncConditions) -> Result<()> {
        let mut s = self.session.lock().map_err(|_| Error::Busy)?;
        s.conditions = c;
        s.observed = Some(Instant::now());
        Ok(())
    }
    pub fn snapshot(&self) -> Result<DeviceSnapshot> {
        let (c, phase, reason) = {
            let s = self.session.lock().map_err(|_| Error::Busy)?;
            (s.connection.clone(), s.phase.clone(), s.reason.clone())
        };
        let mut result = DeviceSnapshot {
            receive: false,
            connection: c.clone(),
            phase,
            reason,
            pending: 0,
            unapplied: 0,
            history: vec![],
            conflicts: vec![],
        };
        if let Some(c) = c {
            let store = Store::open(Path::new(&c.state_dir))?;
            let status = store.status()?;
            result.pending = status.pending as u32;
            result.receive = store.source_mode()?.1 == Mode::Receive;
            if result.receive {
                result.unapplied = (status.received
                    - status.applied_revisions
                    - status.superseded_revisions) as u32;
            }
            result.history = store.history()?;
            for a in store.conflicts()? {
                result.conflicts.push(match a {
                    notes_sync::Action::Conflict {
                        note,
                        local,
                        remote,
                    }
                    | notes_sync::Action::MergeEqual {
                        note,
                        local,
                        remote,
                    } => SyncConflictRow {
                        note: note.to_string(),
                        local: local.to_string(),
                        remote: remote.to_string(),
                        collision: false,
                    },
                    notes_sync::Action::PathCollision { local, remote, .. } => SyncConflictRow {
                        note: local.to_string(),
                        local: local.to_string(),
                        remote: remote.to_string(),
                        collision: true,
                    },
                    _ => continue,
                });
            }
        }
        Ok(result)
    }
    pub fn run(&self, manual: bool) -> Result<()> {
        self.run_with(manual, |store, c| {
            Remote::connect(&store.endpoint()?, Path::new(&c.token_file), None)
        })
    }
    fn run_with<T: Transport>(
        &self,
        manual: bool,
        connect: impl FnOnce(&Store, &SyncConnection) -> Result<T>,
    ) -> Result<()> {
        let _guard = self.operation.try_lock().map_err(|_| Error::Busy)?;
        let c = self.config()?;
        let generation;
        {
            let mut s = self.session.lock().map_err(|_| Error::Busy)?;
            if !manual && (!c.enabled || Instant::now() < s.next) {
                return Ok(());
            }
            if let Some(reason) = pause_reason(
                &c,
                &s.conditions,
                s.observed
                    .is_some_and(|t| t.elapsed() < Duration::from_secs(45)),
            ) {
                s.phase = if reason == "offline" {
                    DevicePhase::Offline
                } else {
                    DevicePhase::Pending
                };
                s.reason = Some(reason.into());
                return Ok(());
            }
            generation = s.cancel;
            s.phase = DevicePhase::Syncing;
            s.reason = None;
        }
        let mut saved_changes = false;
        let outcome = (|| {
            let store = Store::open(Path::new(&c.state_dir))?;
            if store.source_mode()?.1 == Mode::Upload {
                store.stage()?;
            } else if c.capture_saved {
                store.stage_receiver_edits()?;
            }
            let mut remote = Budget::new(connect(&store, &c)?);
            remote.check = Some(Box::new(|| {
                let s = self.session.lock().map_err(|_| Error::Busy)?;
                if s.cancel != generation {
                    return Err(Error::Busy);
                }
                if pause_reason(
                    &c,
                    &s.conditions,
                    s.observed
                        .is_some_and(|t| t.elapsed() < Duration::from_secs(45)),
                )
                .is_some()
                {
                    return Err(Error::Offline);
                }
                Ok(())
            }));
            match store.transfer(&mut remote) {
                Err(Error::Conflict) => {
                    store.fetch(&mut remote)?;
                    return Err(Error::Conflict);
                }
                other => other?,
            }
            if !store.conflicts()?.is_empty() {
                return Err(Error::Conflict);
            }
            if store.source_mode()?.1 == Mode::Receive {
                if c.capture_saved {
                    store.confirm_receiver_edit()?;
                }
                store.acknowledge(&mut remote)?;
            }
            let status = store.status()?;
            saved_changes = store.receiver_changes()?;
            Ok(saved_changes
                || status.pending > 0
                || (store.source_mode()?.1 == Mode::Receive
                    && status.received > status.applied_revisions + status.superseded_revisions))
        })();
        let mut s = self.session.lock().map_err(|_| Error::Busy)?;
        if s.cancel != generation {
            return outcome.map(|_| ());
        }
        match &outcome {
            Ok(pending) => {
                s.failures = 0;
                s.phase = if *pending {
                    DevicePhase::Pending
                } else {
                    DevicePhase::Current
                };
                s.reason = saved_changes.then(|| "saved_receiver_changes".into());
            }
            Err(e) => {
                s.failures = (s.failures + 1).min(5);
                s.phase = match e {
                    Error::Offline => DevicePhase::Offline,
                    Error::Conflict => DevicePhase::Conflict,
                    _ => DevicePhase::Error,
                };
                s.reason = Some(e.to_string());
            }
        }
        s.next = Instant::now() + Duration::from_secs(backoff(c.interval_seconds, s.failures));
        outcome.map(|_| ())
    }
    pub fn pair(&self, r: SyncPairRequest) -> Result<()> {
        let _guard = self.operation.try_lock().map_err(|_| Error::Busy)?;
        if !["upload", "download", "reconcile"].contains(&r.mode.as_str()) {
            return Err(Error::Invalid);
        }
        notes_core::sync::validate_state_location(
            &[Path::new(&r.source)],
            Path::new(&r.token_file),
        )
        .map_err(|_| Error::Invalid)?;
        let endpoint = Endpoint {
            origin: r.origin,
            name: r.workspace,
            allow_private: r.allow_private,
            scope: r
                .scope
                .filter(|s| !s.is_empty())
                .map(|s| notes_model::RelPath::parse(&s))
                .transpose()
                .map_err(|_| Error::Invalid)?,
        };
        let store = Store::open(Path::new(&r.state_dir))?;
        if r.mode == "download"
            && !notes_core::sync::inventory(Path::new(&r.source), &self.data)
                .map_err(|_| Error::ApplicationBlocked)?
                .is_empty()
        {
            return Err(Error::Conflict);
        }
        let mut remote = Budget::new(Remote::connect(&endpoint, Path::new(&r.token_file), None)?);
        store.initialize(
            Path::new(&r.source),
            endpoint,
            if r.mode == "upload" {
                Mode::Upload
            } else {
                Mode::Receive
            },
            &mut remote,
        )?;
        // Enrollment is durable before fetching; a network failure can be resumed
        // by attaching this same state directory, never by reinitializing it.
        if r.mode == "upload" {
            store.stage()?;
        } else {
            store.fetch(&mut remote)?;
        }
        Ok(())
    }
    pub fn preview(&self) -> Result<SyncPairPreview> {
        let _guard = self.operation.try_lock().map_err(|_| Error::Busy)?;
        let store = Store::open(Path::new(&self.config()?.state_dir))?;
        let p = store.preview_pairing(&self.data)?;
        Ok(SyncPairPreview {
            confirmation: p.confirmation,
            attachment_conflicts: p
                .attachment_conflicts
                .into_iter()
                .map(|p| p.to_string())
                .collect(),
            rows: p
                .actions
                .into_iter()
                .map(|a| match a {
                    notes_sync::PairingAction::Upload { local } => SyncPairRow {
                        action: "upload".into(),
                        path: local.path.to_string(),
                    },
                    notes_sync::PairingAction::Download { remote } => SyncPairRow {
                        action: "download".into(),
                        path: remote.path.to_string(),
                    },
                    notes_sync::PairingAction::Link { local, .. } => SyncPairRow {
                        action: "link".into(),
                        path: local.path.to_string(),
                    },
                    notes_sync::PairingAction::Conflict { local, .. } => SyncPairRow {
                        action: "conflict".into(),
                        path: local.path.to_string(),
                    },
                })
                .collect(),
        })
    }
    pub fn confirm(&self, confirmation: String) -> Result<()> {
        let _guard = self.operation.try_lock().map_err(|_| Error::Busy)?;
        let c = self.config()?;
        let store = Store::open(Path::new(&c.state_dir))?;
        let mut remote = Budget::new(Remote::connect(
            &store.endpoint()?,
            Path::new(&c.token_file),
            None,
        )?);
        store.confirm_pairing(&self.data, &confirmation, &mut remote)
    }
    pub fn apply(&self, resolution: Option<String>) -> Result<()> {
        let _guard = self.operation.try_lock().map_err(|_| Error::Busy)?;
        let store = Store::open(Path::new(&self.config()?.state_dir))?;
        match resolution {
            Some(id) => store
                .apply_resolution(&self.data, id.parse().map_err(|_| Error::Invalid)?)
                .map(|_| ()),
            None => store.apply_effects(&self.data).map(|_| ()),
        }
    }
    pub fn capture(&self, note: String) -> Result<()> {
        let _guard = self.operation.try_lock().map_err(|_| Error::Busy)?;
        Store::open(Path::new(&self.config()?.state_dir))?
            .capture_receiver_conflict(&self.data, note.parse().map_err(|_| Error::Invalid)?)
            .map(|_| ())
    }
    pub fn recapture(&self) -> Result<()> {
        let _guard = self.operation.try_lock().map_err(|_| Error::Busy)?;
        Store::open(Path::new(&self.config()?.state_dir))?
            .recapture_receiver_conflict(&self.data)
            .map(|_| ())
    }
    pub fn resolve(
        &self,
        local: String,
        remote: String,
        path: String,
        result: Option<String>,
    ) -> Result<String> {
        let _guard = self.operation.try_lock().map_err(|_| Error::Busy)?;
        let store = Store::open(Path::new(&self.config()?.state_dir))?;
        let local = local.parse().map_err(|_| Error::Invalid)?;
        let remote = remote.parse().map_err(|_| Error::Invalid)?;
        let path = notes_model::RelPath::parse(&path).map_err(|_| Error::Invalid)?;
        let id = match result {
            Some(file) => store.resolve_to(local, remote, path, Path::new(&file))?,
            None => store.resolve_delete(local, remote, path)?,
        };
        Ok(id.to_string())
    }
    pub fn export(&self, id: String, attachment: Option<String>) -> Result<String> {
        let _guard = self.operation.try_lock().map_err(|_| Error::Busy)?;
        let store = Store::open(Path::new(&self.config()?.state_dir))?;
        let id = id.parse().map_err(|_| Error::Invalid)?;
        let path = match attachment {
            Some(path) => store.export_attachment(
                id,
                &notes_model::RelPath::parse(&path).map_err(|_| Error::Invalid)?,
            )?,
            None => store.export(id)?,
        };
        Ok(path.to_string_lossy().into_owned())
    }
}
fn validate_connection(c: &SyncConnection) -> Result<()> {
    if !(120..=3600).contains(&c.interval_seconds) || !Path::new(&c.token_file).is_absolute() {
        return Err(Error::Invalid);
    }
    let store = Store::open(Path::new(&c.state_dir))?;
    let (source, _) = store.source_mode()?;
    notes_core::sync::validate_state_location(&[&source], Path::new(&c.token_file))
        .map_err(|_| Error::Invalid)?;
    store.endpoint()?.validate()?;
    Ok(())
}
fn pause_reason(c: &SyncConnection, h: &SyncConditions, fresh: bool) -> Option<&'static str> {
    if !fresh || !h.online {
        Some("offline")
    } else if !c.allow_metered && h.metered != Some(false) {
        Some("network_limited_or_unknown")
    } else if !c.allow_battery && h.charging != Some(true) {
        Some("power_limited_or_unknown")
    } else {
        None
    }
}
fn backoff(interval: u32, failures: u32) -> u64 {
    (u64::from(interval) * 2u64.pow(failures.min(5))).min(3600)
}
fn worker(weak: Weak<Controller>) {
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let Some(c) = weak.upgrade() else { break };
        let _ = c.run(false);
    }
}
/// Count metadata, publication and receipt requests. Exhaustion preserves the
/// original queue; a later pass resumes its durable checkpoints.
struct Budget<'a, T> {
    check: Option<Box<dyn Fn() -> Result<()> + 'a>>,
    inner: T,
    requests: u32,
}
impl<T> Budget<'_, T> {
    fn new(inner: T) -> Self {
        Self {
            inner,
            check: None,
            requests: 48,
        }
    }
    fn take(&mut self) -> Result<()> {
        if let Some(check) = &self.check {
            check()?;
        }
        if self.requests == 0 {
            return Err(Error::Busy);
        }
        self.requests -= 1;
        Ok(())
    }
}
impl<T: Transport> Transport for Budget<'_, T> {
    fn page(&mut self, cursor: usize) -> Result<crate::remote::Page> {
        self.take()?;
        self.inner.page(cursor)
    }
    fn fetch(&mut self, id: Uuid) -> Result<Publication> {
        self.take()?;
        self.inner.fetch(id)
    }
    fn publish(&mut self, p: &Publication) -> Result<()> {
        self.take()?;
        self.inner.publish(p)
    }
    fn acknowledge(&mut self, a: &ApplicationAcknowledgment) -> Result<()> {
        self.take()?;
        self.inner.acknowledge(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::Page;
    use std::cell::RefCell;
    struct Peer {
        journal: notes_sync::Journal,
        log: Vec<Publication>,
    }
    impl Peer {
        fn new() -> Self {
            Self {
                journal: notes_sync::Journal::new(Uuid::new_v4()),
                log: vec![],
            }
        }
    }
    impl Transport for &RefCell<Peer> {
        fn page(&mut self, cursor: usize) -> Result<Page> {
            let p = self.borrow();
            let end = (cursor + 20).min(p.log.len());
            Ok(Page {
                workspace: p.journal.workspace,
                revisions: p.log[cursor..end]
                    .iter()
                    .map(|p| p.revision.clone())
                    .collect(),
                heads: p.journal.heads.clone(),
                next_cursor: end,
                has_more: end < p.log.len(),
            })
        }
        fn fetch(&mut self, id: Uuid) -> Result<Publication> {
            self.borrow()
                .log
                .iter()
                .find(|p| p.revision.id == id)
                .cloned()
                .ok_or(Error::Protocol)
        }
        fn publish(&mut self, p: &Publication) -> Result<()> {
            let mut peer = self.borrow_mut();
            notes_sync::transfer::append(&mut peer.journal, p).map_err(|_| Error::Conflict)?;
            peer.log.push(p.clone());
            Ok(())
        }
        fn acknowledge(&mut self, _: &ApplicationAcknowledgment) -> Result<()> {
            Ok(())
        }
    }
    fn config(dir: &Path) -> SyncConnection {
        SyncConnection {
            state_dir: dir.join("queue").to_string_lossy().into(),
            token_file: dir.join("operator.secret").to_string_lossy().into(),
            enabled: false,
            interval_seconds: 120,
            allow_metered: false,
            allow_battery: false,
            capture_saved: false,
        }
    }
    #[test]
    fn older_connections_do_not_enable_receiver_capture() {
        let mut value = serde_json::to_value(config(Path::new("/private"))).unwrap();
        value.as_object_mut().unwrap().remove("capture_saved");
        let older: SyncConnection = serde_json::from_value(value).unwrap();
        assert!(!older.capture_saved);
    }
    #[test]
    fn conservative_conditions_and_backoff_are_bounded() {
        let c = config(Path::new("/private"));
        let mut h = SyncConditions {
            online: true,
            metered: None,
            charging: None,
        };
        assert_eq!(
            pause_reason(&c, &h, true),
            Some("network_limited_or_unknown")
        );
        h.metered = Some(false);
        assert_eq!(pause_reason(&c, &h, true), Some("power_limited_or_unknown"));
        h.charging = Some(true);
        assert_eq!(pause_reason(&c, &h, true), None);
        assert_eq!(pause_reason(&c, &h, false), Some("offline"));
        assert_eq!(backoff(120, 0), 120);
        assert_eq!(backoff(120, 1), 240);
        assert_eq!(backoff(3600, 32), 3600);
    }
    #[test]
    fn worker_retains_offline_edits_and_restarts_without_implicit_application() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("notes");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("a.md"), b"original\r\n").unwrap();
        let c = config(temp.path());
        let store = Store::open(Path::new(&c.state_dir)).unwrap();
        let peer = RefCell::new(Peer::new());
        store
            .initialize(
                &root,
                Endpoint {
                    origin: "https://notes.example/".into(),
                    name: "home".into(),
                    scope: None,
                    allow_private: false,
                },
                Mode::Upload,
                &mut &peer,
            )
            .unwrap();
        let app = temp.path().join("app-data");
        let controller = Controller::new(&app);
        controller.configure(c.clone()).unwrap();
        controller
            .run_with(false, |_, _| -> Result<&RefCell<Peer>> {
                panic!("disabled worker connected")
            })
            .unwrap();
        controller
            .run_with(true, |_, _| -> Result<&RefCell<Peer>> {
                panic!("unknown conditions connected")
            })
            .unwrap();
        assert_eq!(store.status().unwrap().pending, 0);
        controller
            .conditions(SyncConditions {
                online: true,
                metered: Some(false),
                charging: Some(true),
            })
            .unwrap();
        assert!(controller
            .run_with(true, |_, _| -> Result<&RefCell<Peer>> {
                Err(Error::Offline)
            })
            .is_err());
        assert_eq!(store.status().unwrap().pending, 1);
        assert!(matches!(
            controller.snapshot().unwrap().phase,
            DevicePhase::Offline
        ));
        controller.run_with(true, |_, _| Ok(&peer)).unwrap();
        assert_eq!(store.status().unwrap().pending, 0);
        assert_eq!(peer.borrow().log.len(), 1);
        assert_eq!(fs::read(root.join("a.md")).unwrap(), b"original\r\n");
        assert!(matches!(
            controller.snapshot().unwrap().phase,
            DevicePhase::Current
        ));
        drop(controller);
        let reopened = Controller::new(&app);
        assert_eq!(reopened.config().unwrap().state_dir, c.state_dir);
        reopened
            .run_with(true, |_, _| -> Result<&RefCell<Peer>> {
                panic!("restart reused stale conditions")
            })
            .unwrap();
    }
    #[test]
    fn received_bytes_remain_pending_until_explicit_application() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("a.md"), b"exact\r\n").unwrap();
        let peer = RefCell::new(Peer::new());
        let endpoint = Endpoint {
            origin: "https://notes.example/".into(),
            name: "home".into(),
            scope: None,
            allow_private: false,
        };
        let sender = Store::open(&temp.path().join("sender")).unwrap();
        sender
            .initialize(&source, endpoint.clone(), Mode::Upload, &mut &peer)
            .unwrap();
        sender.stage().unwrap();
        sender.transfer(&mut &peer).unwrap();
        let target = temp.path().join("target");
        fs::create_dir(&target).unwrap();
        let c = config(temp.path());
        let receiver = Store::open(Path::new(&c.state_dir)).unwrap();
        receiver
            .initialize(&target, endpoint, Mode::Receive, &mut &peer)
            .unwrap();
        let controller = Controller::new(&temp.path().join("app"));
        controller.configure(c).unwrap();
        controller
            .conditions(SyncConditions {
                online: true,
                metered: Some(false),
                charging: Some(true),
            })
            .unwrap();
        controller.run_with(true, |_, _| Ok(&peer)).unwrap();
        assert!(!target.join("a.md").exists());
        let snapshot = controller.snapshot().unwrap();
        assert!(matches!(snapshot.phase, DevicePhase::Pending));
        assert_eq!(snapshot.unapplied, 1);
        controller.apply(None).unwrap();
        controller.run_with(true, |_, _| Ok(&peer)).unwrap();
        assert!(matches!(
            controller.snapshot().unwrap().phase,
            DevicePhase::Current
        ));
        assert_eq!(fs::read(target.join("a.md")).unwrap(), b"exact\r\n");
        fs::write(target.join("a.md"), b"saved offline edit").unwrap();
        controller.run_with(true, |_, _| Ok(&peer)).unwrap();
        let snapshot = controller.snapshot().unwrap();
        assert!(matches!(snapshot.phase, DevicePhase::Pending));
        assert_eq!(snapshot.reason.as_deref(), Some("saved_receiver_changes"));
        assert_eq!(peer.borrow().log.len(), 1);
        let mut enabled = controller.config().unwrap();
        enabled.capture_saved = true;
        controller.configure(enabled).unwrap();
        controller.run_with(true, |_, _| Ok(&peer)).unwrap();
        assert_eq!(peer.borrow().log.len(), 2);
        assert!(matches!(
            controller.snapshot().unwrap().phase,
            DevicePhase::Current
        ));
        assert_eq!(
            fs::read(target.join("a.md")).unwrap(),
            b"saved offline edit"
        );
        fs::write(target.join("a.md"), b"next saved edit").unwrap();
        controller.run_with(true, |_, _| Ok(&peer)).unwrap();
        assert_eq!(peer.borrow().log.len(), 3);
        assert!(matches!(
            controller.snapshot().unwrap().phase,
            DevicePhase::Current
        ));
        controller
            .run_with(true, |_, _| {
                controller.pause().unwrap();
                Ok(&peer)
            })
            .unwrap_err();
        assert!(matches!(
            controller.snapshot().unwrap().phase,
            DevicePhase::Disabled
        ));
        assert!(!controller.config().unwrap().enabled);
    }
    #[test]
    fn request_budget_and_changed_conditions_stop_before_next_request() {
        let peer = RefCell::new(Peer::new());
        let mut transport = Budget::new(&peer);
        for _ in 0..48 {
            transport.page(0).unwrap();
        }
        assert!(matches!(transport.page(0), Err(Error::Busy)));
        let mut transport = Budget::new(&peer);
        transport.check = Some(Box::new(|| Err(Error::Offline)));
        assert!(matches!(transport.page(0), Err(Error::Offline)));
        assert_eq!(transport.requests, 48);
    }
    #[test]
    fn future_configuration_is_preserved_on_startup() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("sync-control.json");
        let bytes = b"{\"future_schema\":42}";
        fs::write(&path, bytes).unwrap();
        let controller = Controller::new(temp.path());
        assert!(matches!(
            controller.snapshot().unwrap().phase,
            DevicePhase::Error
        ));
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}
