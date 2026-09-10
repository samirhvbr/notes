use notes_sync::{transfer::Publication, Journal};
use notes_sync_client::{
    remote::{Endpoint, Page, Transport},
    state::{Mode, Store},
    Error, Result,
};
use std::fs;
use uuid::Uuid;

struct Peer {
    journal: Journal,
    log: Vec<Publication>,
    lose_receipt: bool,
    bad_fetch: bool,
}
impl Peer {
    fn new() -> Self {
        Self {
            journal: Journal::new(Uuid::new_v4()),
            log: vec![],
            lose_receipt: false,
            bad_fetch: false,
        }
    }
}
impl Transport for Peer {
    fn page(&mut self, cursor: usize) -> Result<Page> {
        if cursor > self.log.len() {
            return Err(Error::Protocol);
        }
        let end = (cursor + 20).min(self.log.len());
        Ok(Page {
            workspace: self.journal.workspace,
            revisions: self.log[cursor..end]
                .iter()
                .map(|p| p.revision.clone())
                .collect(),
            heads: self.journal.heads.clone(),
            next_cursor: end,
            has_more: end < self.log.len(),
        })
    }
    fn fetch(&mut self, id: Uuid) -> Result<Publication> {
        let mut p = self
            .log
            .iter()
            .find(|p| p.revision.id == id)
            .unwrap()
            .clone();
        if self.bad_fetch {
            p.content_base64 = Some("YmFk".into());
        }
        Ok(p)
    }
    fn publish(&mut self, p: &Publication) -> Result<()> {
        if let Some(old) = self.log.iter().find(|q| q.revision.id == p.revision.id) {
            return if old == p {
                Ok(())
            } else {
                Err(Error::Conflict)
            };
        }
        self.journal
            .commit(p.revision.clone(), p.expected)
            .map_err(|_| Error::Conflict)?;
        self.log.push(p.clone());
        if std::mem::take(&mut self.lose_receipt) {
            Err(Error::Offline)
        } else {
            Ok(())
        }
    }
}
fn endpoint() -> Endpoint {
    Endpoint {
        origin: "https://notes.example/".into(),
        name: "home".into(),
        allow_private: false,
    }
}
fn fixture() -> (tempfile::TempDir, std::path::PathBuf, Store, Peer) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("notes");
    fs::create_dir(&root).unwrap();
    let store = Store::open(&dir.path().join("state")).unwrap();
    let mut peer = Peer::new();
    store
        .initialize(&root, endpoint(), Mode::Upload, &mut peer)
        .unwrap();
    (dir, root, store, peer)
}
#[test]
fn lost_receipt_restart_offline_edits_and_download_preserve_exact_bytes() {
    let (dir, root, store, mut peer) = fixture();
    let bytes = b"\xef\xbb\xbfhello\r\n\xff";
    fs::write(root.join("test.md"), bytes).unwrap();
    assert_eq!(store.stage().unwrap(), 0);
    let queued = fs::read(dir.path().join("state/client.json")).unwrap();
    peer.lose_receipt = true;
    assert!(matches!(store.transfer(&mut peer), Err(Error::Offline)));
    assert_eq!(
        fs::read(dir.path().join("state/client.json")).unwrap(),
        queued
    );
    assert_eq!(peer.log.len(), 1);
    let store = Store::open(&dir.path().join("state")).unwrap();
    fs::write(root.join("test.md"), b"second\r\n").unwrap();
    store.stage().unwrap();
    assert_eq!(store.status().unwrap().pending, 2);
    store.transfer(&mut peer).unwrap();
    assert_eq!(peer.log.len(), 2);
    assert_eq!(store.status().unwrap().pending, 0);
    let first = peer.log[0].revision.id;
    let path = store.export(first).unwrap();
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert!(store.export(first).is_err());
    assert_eq!(fs::read(root.join("test.md")).unwrap(), b"second\r\n");
    assert!(!store.status().unwrap().applied);
}
#[test]
fn second_device_receives_but_never_applies_and_bad_page_does_not_advance() {
    let (dir, root, sender, mut peer) = fixture();
    fs::write(root.join("test.md"), b"source").unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    let local = dir.path().join("second");
    fs::create_dir(&local).unwrap();
    fs::write(local.join("draft.md"), b"unsaved elsewhere").unwrap();
    let receiver = Store::open(&dir.path().join("receiver")).unwrap();
    receiver
        .initialize(&local, endpoint(), Mode::Receive, &mut peer)
        .unwrap();
    let before = fs::read(dir.path().join("receiver/client.json")).unwrap();
    peer.bad_fetch = true;
    assert!(matches!(receiver.transfer(&mut peer), Err(Error::Protocol)));
    assert_eq!(
        fs::read(dir.path().join("receiver/client.json")).unwrap(),
        before
    );
    peer.bad_fetch = false;
    receiver.transfer(&mut peer).unwrap();
    assert_eq!(receiver.status().unwrap().cursor, 1);
    assert_eq!(
        fs::read(receiver.export(peer.log[0].revision.id).unwrap()).unwrap(),
        b"source"
    );
    assert!(!local.join("test.md").exists());
    assert_eq!(
        fs::read(local.join("draft.md")).unwrap(),
        b"unsaved elsewhere"
    );
    assert!(receiver.stage().is_err());
}
#[test]
fn missing_files_are_not_deletions_and_renames_keep_identity() {
    let (_dir, root, store, mut peer) = fixture();
    fs::write(root.join("before.md"), b"hello").unwrap();
    store.stage().unwrap();
    store.transfer(&mut peer).unwrap();
    fs::rename(root.join("before.md"), root.join("after.md")).unwrap();
    store.stage().unwrap();
    store.transfer(&mut peer).unwrap();
    assert_eq!(peer.log[0].revision.note, peer.log[1].revision.note);
    fs::remove_file(root.join("after.md")).unwrap();
    assert_eq!(store.stage().unwrap(), 1);
    assert_eq!(store.status().unwrap().pending, 0);
    assert!(peer.log.iter().all(|p| p.revision.content.is_some()));
}
#[test]
fn conflicts_and_rebound_server_preserve_pending_publication() {
    let (dir, root, store, mut peer) = fixture();
    fs::write(root.join("test.md"), b"initial").unwrap();
    store.stage().unwrap();
    store.transfer(&mut peer).unwrap();
    fs::write(root.join("test.md"), b"local").unwrap();
    store.stage().unwrap();
    let before = fs::read(dir.path().join("state/client.json")).unwrap();
    let mut foreign = Peer::new();
    assert!(matches!(store.transfer(&mut foreign), Err(Error::Protocol)));
    assert_eq!(
        fs::read(dir.path().join("state/client.json")).unwrap(),
        before
    );
    let mut changed = peer.log[0].clone();
    changed.expected = Some(changed.revision.id);
    changed.revision.parents = [changed.revision.id].into_iter().collect();
    changed.revision.id = Uuid::new_v4();
    peer.publish(&changed).unwrap();
    assert!(matches!(store.transfer(&mut peer), Err(Error::Conflict)));
    assert_eq!(
        fs::read(dir.path().join("state/client.json")).unwrap(),
        before
    );
}
#[test]
fn unsafe_state_and_future_schema_are_refused_without_reset() {
    let (dir, root, store, mut peer) = fixture();
    let unsafe_state = Store::open(&root.join("state")).unwrap();
    assert!(unsafe_state
        .initialize(&root, endpoint(), Mode::Upload, &mut peer)
        .is_err());
    assert!(!root.join("state").exists());
    let path = dir.path().join("state/client.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["schema"] = serde_json::json!(99);
    let future = serde_json::to_vec(&value).unwrap();
    fs::write(&path, &future).unwrap();
    assert!(store.stage().is_err());
    assert!(store.status().is_err());
    assert_eq!(fs::read(&path).unwrap(), future);
}

#[test]
fn oversized_capture_preserves_existing_queue_and_source() {
    let (dir, root, store, _) = fixture();
    fs::write(root.join("small.md"), b"queued").unwrap();
    store.stage().unwrap();
    let before = fs::read(dir.path().join("state/client.json")).unwrap();
    let file = fs::File::create(root.join("large.md")).unwrap();
    file.set_len(8 * 1024 * 1024 + 1).unwrap();
    assert!(store.stage().is_err());
    assert_eq!(
        fs::read(dir.path().join("state/client.json")).unwrap(),
        before
    );
    assert_eq!(fs::read(root.join("small.md")).unwrap(), b"queued");
    assert_eq!(file.metadata().unwrap().len(), 8 * 1024 * 1024 + 1);
}
