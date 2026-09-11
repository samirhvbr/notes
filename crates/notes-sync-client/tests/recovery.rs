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
    fn acknowledge(&mut self, r: &notes_sync::transfer::ApplicationAcknowledgment) -> Result<()> {
        if r.workspace != self.journal.workspace {
            return Err(Error::Protocol);
        }
        let note = self.journal.revisions[&r.revision].note;
        self.journal
            .acknowledge(r.device, [(note, r.revision)].into())
            .map_err(|_| Error::Conflict)?;
        if std::mem::take(&mut self.lose_receipt) {
            Err(Error::Offline)
        } else {
            Ok(())
        }
    }

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
        let mut next = self.journal.clone();
        notes_sync::transfer::append(&mut next, p).map_err(|_| Error::Conflict)?;
        next.validate().map_err(|_| Error::Conflict)?;
        self.journal = next;
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

fn receiver(dir: &std::path::Path, peer: &mut Peer) -> (std::path::PathBuf, Store) {
    let root = dir.join("receiver-root");
    fs::create_dir(&root).unwrap();
    let store = Store::open(&dir.join("receiver")).unwrap();
    store
        .initialize(&root, endpoint(), Mode::Receive, peer)
        .unwrap();
    store.transfer(peer).unwrap();
    (root, store)
}

#[test]
fn apply_checkpoints_updates_and_preserves_local_edits() {
    let (dir, root, sender, mut peer) = fixture();
    let bytes = b"\xef\xbb\xbfhello\r\n\xff";
    fs::write(root.join("test.md"), bytes).unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    let (target, receiver) = receiver(dir.path(), &mut peer);
    let data = dir.path().join("app-data");
    assert_eq!(receiver.apply(&data).unwrap(), 1);
    assert_eq!(fs::read(target.join("test.md")).unwrap(), bytes);
    assert!(receiver.status().unwrap().applied);
    assert_eq!(receiver.apply(&data).unwrap(), 0);
    fs::write(root.join("test.md"), b"second\r\n").unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    receiver.transfer(&mut peer).unwrap();
    assert_eq!(receiver.apply(&data).unwrap(), 1);
    assert_eq!(fs::read(target.join("test.md")).unwrap(), b"second\r\n");
    fs::write(target.join("test.md"), b"local work").unwrap();
    fs::write(root.join("test.md"), b"third").unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    receiver.transfer(&mut peer).unwrap();
    assert!(matches!(
        receiver.apply(&data),
        Err(Error::ApplicationBlocked)
    ));
    assert_eq!(receiver.status().unwrap().applied_revisions, 2);
    assert_eq!(receiver.status().unwrap().received, 3);
    assert_eq!(fs::read(target.join("test.md")).unwrap(), b"local work");
    assert!(receiver.apply(&dir.path().join("wrong-app-data")).is_err());
}

#[test]
fn durable_intent_recovers_lost_receipt_without_rewriting_but_rejects_later_edits() {
    let (dir, root, sender, mut peer) = fixture();
    fs::write(root.join("test.md"), b"remote").unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    let (target, receiver) = receiver(dir.path(), &mut peer);
    let data = dir.path().join("app-data");
    receiver.apply(&data).unwrap();
    let meta = fs::metadata(target.join("test.md"))
        .unwrap()
        .modified()
        .unwrap();
    let checkpoint = serde_json::json!({"schema":1,"core_data":fs::canonicalize(&data).unwrap(),"next":0,"notes":{},"intent":peer.log[0].revision.id});
    let state_path = dir.path().join("receiver/application.json");
    // Crash boundary: intent was durable and the source was written, but the
    // application receipt did not reach disk. Replay the exact durable state.
    fs::write(&state_path, serde_json::to_vec(&checkpoint).unwrap()).unwrap();
    assert_eq!(receiver.apply(&data).unwrap(), 1);
    assert_eq!(
        fs::metadata(target.join("test.md"))
            .unwrap()
            .modified()
            .unwrap(),
        meta
    );
    fs::write(&state_path, serde_json::to_vec(&checkpoint).unwrap()).unwrap();
    fs::write(target.join("test.md"), b"newer local").unwrap();
    assert!(matches!(
        receiver.apply(&data),
        Err(Error::ApplicationBlocked)
    ));
    assert_eq!(fs::read(target.join("test.md")).unwrap(), b"newer local");
    assert_eq!(receiver.status().unwrap().applied_revisions, 0);
}

#[test]
fn collision_never_creates_intent_and_future_application_state_is_preserved() {
    let (dir, root, sender, mut peer) = fixture();
    fs::write(root.join("test.md"), b"same").unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    let (target, receiver) = receiver(dir.path(), &mut peer);
    let data = dir.path().join("app-data");
    fs::write(target.join("test.md"), b"same").unwrap();
    let app = dir.path().join("receiver/application.json");
    for _ in 0..2 {
        assert!(matches!(
            receiver.apply(&data),
            Err(Error::ApplicationBlocked)
        ));
        assert!(!app.exists());
    }
    fs::write(&app, b"{\"schema\":999}").unwrap();
    assert!(receiver.apply(&data).is_err());
    assert_eq!(fs::read(app).unwrap(), b"{\"schema\":999}");
}

#[test]
fn rename_remains_received_without_moving_or_advancing_application() {
    let (dir, root, sender, mut peer) = fixture();
    fs::write(root.join("test.md"), b"remote").unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    let (target, receiver) = receiver(dir.path(), &mut peer);
    let data = dir.path().join("app-data");
    receiver.apply(&data).unwrap();
    fs::rename(root.join("test.md"), root.join("renamed.md")).unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    receiver.transfer(&mut peer).unwrap();
    assert!(matches!(
        receiver.apply(&data),
        Err(Error::UnsupportedApplication)
    ));
    assert_eq!(receiver.status().unwrap().applied_revisions, 1);
    assert_eq!(receiver.status().unwrap().received, 2);
    assert_eq!(fs::read(target.join("test.md")).unwrap(), b"remote");
    assert!(!target.join("renamed.md").exists());
}

#[test]
fn application_acknowledgments_resume_after_lost_response_and_exclude_unapplied_bytes() {
    let (dir, root, sender, mut peer) = fixture();
    fs::write(root.join("test.md"), b"first").unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    let (target, receiver) = receiver(dir.path(), &mut peer);
    assert_eq!(receiver.acknowledge(&mut peer).unwrap(), 0);
    assert!(peer.journal.acknowledgments.is_empty());
    let data = dir.path().join("app-data");
    receiver.apply(&data).unwrap();
    let checkpoint = dir.path().join("receiver/application.json");
    let before = fs::read(&checkpoint).unwrap();
    peer.lose_receipt = true;
    assert!(matches!(
        receiver.acknowledge(&mut peer),
        Err(Error::Offline)
    ));
    assert_eq!(fs::read(&checkpoint).unwrap(), before);
    assert_eq!(peer.journal.acknowledgments.len(), 1);
    let receiver = Store::open(&dir.path().join("receiver")).unwrap();
    assert_eq!(receiver.acknowledge(&mut peer).unwrap(), 1);
    assert_eq!(receiver.acknowledge(&mut peer).unwrap(), 0);
    assert_eq!(receiver.status().unwrap().acknowledged_revisions, 1);
    fs::write(root.join("test.md"), b"second").unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    receiver.transfer(&mut peer).unwrap();
    fs::write(target.join("test.md"), b"local work").unwrap();
    assert!(receiver.apply(&data).is_err());
    assert_eq!(receiver.acknowledge(&mut peer).unwrap(), 0);
    assert_eq!(
        peer.journal.acknowledgments.values().next().unwrap()[&peer.log[0].revision.note],
        peer.log[0].revision.id
    );
    assert_eq!(fs::read(target.join("test.md")).unwrap(), b"local work");
}

#[test]
fn acknowledgment_batches_are_bounded_and_old_checkpoints_default_to_pending() {
    let (dir, root, sender, mut peer) = fixture();
    for i in 0..21 {
        fs::write(root.join(format!("{i}.md")), b"content").unwrap();
    }
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    sender.transfer(&mut peer).unwrap();
    let (_, receiver) = receiver(dir.path(), &mut peer);
    receiver.transfer(&mut peer).unwrap();
    let data = dir.path().join("app-data");
    receiver.apply(&data).unwrap();
    receiver.apply(&data).unwrap();
    let path = dir.path().join("receiver/application.json");
    let mut old: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    old.as_object_mut().unwrap().remove("acknowledged");
    fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    assert_eq!(receiver.status().unwrap().acknowledged_revisions, 0);
    assert_eq!(receiver.acknowledge(&mut peer).unwrap(), 20);
    assert_eq!(receiver.acknowledge(&mut peer).unwrap(), 1);
    old["acknowledged"] = serde_json::json!(22);
    fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    assert!(receiver.acknowledge(&mut peer).is_err());
}

#[test]
fn editor_batch_advances_buffer_bases_and_keeps_partial_receipts() {
    let (dir, root, sender, mut peer) = fixture();
    fs::write(root.join("test.md"), b"first").unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    let (target, receiver) = receiver(dir.path(), &mut peer);
    let data = dir.path().join("app-data");
    receiver.apply(&data).unwrap();
    let mut service = notes_core::WorkspaceService::with_data_dir(&data).unwrap();
    receiver.open_for_editor(&mut service).unwrap();
    let note = service
        .open_note(&notes_model::RelPath::parse("test.md").unwrap())
        .unwrap();
    let buffer = notes_core::sync::BufferSnapshot {
        note_id: note.note_id,
        base_rev: note.base_rev,
        buffer_version: 0,
        saved_version: 0,
    };
    for bytes in [b"second".as_slice(), b"third".as_slice()] {
        fs::write(root.join("test.md"), bytes).unwrap();
        sender.stage().unwrap();
    }
    sender.transfer(&mut peer).unwrap();
    receiver.transfer(&mut peer).unwrap();
    let report = receiver.apply_for_editor(&mut service, vec![buffer]);
    assert_eq!(report.applied, Some(2));
    assert!(report.error.is_none());
    assert!(!report.reload_failed);
    assert_eq!(report.refreshed[0].text, "third");
    assert_eq!(fs::read(target.join("test.md")).unwrap(), b"third");
    let buffer = notes_core::sync::BufferSnapshot {
        note_id: note.note_id,
        base_rev: report.refreshed[0].base_rev.clone(),
        buffer_version: 0,
        saved_version: 0,
    };
    fs::write(root.join("test.md"), b"fourth").unwrap();
    sender.stage().unwrap();
    fs::rename(root.join("test.md"), root.join("renamed.md")).unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    receiver.transfer(&mut peer).unwrap();
    let report = receiver.apply_for_editor(&mut service, vec![buffer]);
    assert!(report.error.is_some());
    assert!(!report.reload_failed);
    assert_eq!(report.refreshed[0].text, "fourth");
    assert_eq!(receiver.status().unwrap().applied_revisions, 4);
    assert!(!target.join("renamed.md").exists());
}

#[test]
fn editor_refuses_dirty_buffers_without_reloading_or_advancing() {
    let (dir, root, sender, mut peer) = fixture();
    fs::write(root.join("test.md"), b"first").unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    let (target, receiver) = receiver(dir.path(), &mut peer);
    let data = dir.path().join("app-data");
    receiver.apply(&data).unwrap();
    let mut service = notes_core::WorkspaceService::with_data_dir(&data).unwrap();
    receiver.open_for_editor(&mut service).unwrap();
    let note = service
        .open_note(&notes_model::RelPath::parse("test.md").unwrap())
        .unwrap();
    fs::write(root.join("test.md"), b"remote").unwrap();
    sender.stage().unwrap();
    sender.transfer(&mut peer).unwrap();
    receiver.transfer(&mut peer).unwrap();
    let dirty = notes_core::sync::BufferSnapshot {
        note_id: note.note_id,
        base_rev: note.base_rev,
        buffer_version: 1,
        saved_version: 0,
    };
    let report = receiver.apply_for_editor(&mut service, vec![dirty]);
    assert!(report.error.is_some());
    assert!(report.refreshed.is_empty());
    assert_eq!(receiver.status().unwrap().applied_revisions, 1);
    assert_eq!(fs::read(target.join("test.md")).unwrap(), b"first");
}

#[test]
fn explicit_resolution_retains_branches_across_remote_races_and_lost_receipts() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use notes_sync::{transfer::Branch, Revision};
    let (dir, root, store, mut peer) = fixture();
    fs::write(root.join("test.md"), b"base").unwrap();
    store.stage().unwrap();
    store.transfer(&mut peer).unwrap();
    let base = peer.log[0].clone();
    fs::write(root.join("test.md"), b"local offline").unwrap();
    store.stage().unwrap();
    let remote_publication = |parent: &Publication, bytes: &[u8]| Publication {
        workspace: parent.workspace,
        expected: Some(parent.revision.id),
        revision: Revision::new(
            parent.revision.note,
            [parent.revision.id].into(),
            Uuid::new_v4(),
            parent.revision.path.clone(),
            Some(notes_model::ContentHash::from_bytes(
                *blake3::hash(bytes).as_bytes(),
            )),
        ),
        content_base64: Some(STANDARD.encode(bytes)),
        branches: vec![],
    };
    let remote = remote_publication(&base, b"remote");
    peer.publish(&remote).unwrap();
    assert!(matches!(store.transfer(&mut peer), Err(Error::Conflict)));
    store.fetch(&mut peer).unwrap();
    let conflicts = store.conflicts().unwrap();
    let notes_sync::Action::Conflict {
        local,
        remote: remote_id,
        ..
    } = conflicts[0]
    else {
        panic!("missing divergence")
    };
    assert_eq!(remote_id, remote.revision.id);
    let result = dir.path().join("chosen.md");
    let chosen = b"\xef\xbb\xbfchosen\r\n\xff";
    fs::write(&result, chosen).unwrap();
    let before = fs::read(dir.path().join("state/client.json")).unwrap();
    assert!(store.resolve(Uuid::new_v4(), remote_id, &result).is_err());
    assert_eq!(
        fs::read(dir.path().join("state/client.json")).unwrap(),
        before
    );
    let merge = store.resolve(local, remote_id, &result).unwrap();
    assert_eq!(fs::read(root.join("test.md")).unwrap(), b"local offline");
    // A peer races after the operator chose its observed heads. Never overwrite it.
    let newer = remote_publication(&remote, b"remote advanced");
    peer.publish(&newer).unwrap();
    assert!(matches!(store.transfer(&mut peer), Err(Error::Conflict)));
    store.fetch(&mut peer).unwrap();
    let final_id = store.resolve(merge, newer.revision.id, &result).unwrap();
    assert_eq!(
        fs::read(store.export(local).unwrap()).unwrap(),
        b"local offline"
    );
    let queued = fs::read(dir.path().join("state/client.json")).unwrap();
    peer.lose_receipt = true;
    assert!(matches!(store.transfer(&mut peer), Err(Error::Offline)));
    assert_eq!(
        fs::read(dir.path().join("state/client.json")).unwrap(),
        queued
    );
    let restarted = Store::open(&dir.path().join("state")).unwrap();
    restarted.transfer(&mut peer).unwrap();
    assert_eq!(restarted.status().unwrap().pending, 0);
    assert!(restarted.conflicts().unwrap().is_empty());
    let envelope = peer.log.last().unwrap();
    assert_eq!(envelope.revision.id, final_id);
    assert_eq!(envelope.revision.parents, [merge, newer.revision.id].into());
    assert!(envelope.branches.iter().any(
        |Branch {
             revision,
             content_base64,
         }| revision.id == local
            && content_base64.as_deref() == Some(&STANDARD.encode(b"local offline"))
    ));
    assert!(peer.journal.is_ancestor(local, final_id));
    assert!(peer.journal.is_ancestor(remote_id, final_id));
    // A receive client writes only accepted heads, never transient branch values.
    let target = dir.path().join("receiver-notes");
    fs::create_dir(&target).unwrap();
    let receiver = Store::open(&dir.path().join("receiver")).unwrap();
    receiver
        .initialize(&target, endpoint(), Mode::Receive, &mut peer)
        .unwrap();
    receiver.fetch(&mut peer).unwrap();
    assert_eq!(
        receiver.apply(&dir.path().join("receiver-data")).unwrap(),
        4
    );
    assert_eq!(fs::read(target.join("test.md")).unwrap(), chosen);
    assert_eq!(receiver.acknowledge(&mut peer).unwrap(), 4);
}

#[test]
fn explicit_paths_and_tombstones_resolve_rename_delete_conflicts_without_source_mutations() {
    for remote_deleted in [false, true] {
        for choose_delete in [false, true] {
            let (dir, root, store, mut peer) = fixture();
            fs::write(root.join("test.md"), b"base").unwrap();
            fs::write(root.join("occupied.md"), b"another note").unwrap();
            store.stage().unwrap();
            store.transfer(&mut peer).unwrap();
            let base = peer
                .log
                .iter()
                .find(|p| p.revision.path.as_str() == "test.md")
                .unwrap()
                .clone();
            fs::write(root.join("test.md"), b"local edit").unwrap();
            store.stage().unwrap();
            let mut remote = base.clone();
            remote.revision.id = Uuid::new_v4();
            remote.revision.device = Uuid::new_v4();
            remote.expected = Some(base.revision.id);
            remote.revision.parents = [base.revision.id].into();
            if remote_deleted {
                remote.revision.content = None;
                remote.content_base64 = None;
            } else {
                remote.revision.path = notes_model::RelPath::parse("renamed.md").unwrap();
            }
            peer.publish(&remote).unwrap();
            store.fetch(&mut peer).unwrap();
            let notes_sync::Action::Conflict {
                local,
                remote: remote_id,
                ..
            } = store.conflicts().unwrap()[0]
            else {
                panic!("missing conflict")
            };
            let result = dir.path().join("chosen.md");
            fs::write(&result, b"chosen\r\n").unwrap();
            let before = fs::read(dir.path().join("state/client.json")).unwrap();
            assert!(store.resolve(local, remote_id, &result).is_err());
            assert!(store
                .resolve_to(
                    local,
                    remote_id,
                    notes_model::RelPath::parse("occupied.md").unwrap(),
                    &result
                )
                .is_err());
            assert!(store
                .resolve_to(
                    local,
                    remote_id,
                    notes_model::RelPath::parse(".private/result.md").unwrap(),
                    &result
                )
                .is_err());
            assert_eq!(
                fs::read(dir.path().join("state/client.json")).unwrap(),
                before
            );
            let path = notes_model::RelPath::parse("chosen-path.md").unwrap();
            let id = if choose_delete {
                store
                    .resolve_delete(local, remote_id, path.clone())
                    .unwrap()
            } else {
                store
                    .resolve_to(local, remote_id, path.clone(), &result)
                    .unwrap()
            };
            let queued = fs::read(dir.path().join("state/client.json")).unwrap();
            peer.lose_receipt = true;
            assert!(matches!(store.transfer(&mut peer), Err(Error::Offline)));
            assert_eq!(
                fs::read(dir.path().join("state/client.json")).unwrap(),
                queued
            );
            Store::open(&dir.path().join("state"))
                .unwrap()
                .transfer(&mut peer)
                .unwrap();
            let merge = peer.log.last().unwrap();
            assert_eq!(merge.revision.id, id);
            assert_eq!(merge.revision.path, path);
            assert_eq!(merge.revision.parents, [local, remote_id].into());
            assert_eq!(merge.revision.content.is_none(), choose_delete);
            assert_eq!(merge.content_base64.is_none(), choose_delete);
            assert_eq!(
                fs::read(store.export(local).unwrap()).unwrap(),
                b"local edit"
            );
            assert_eq!(fs::read(root.join("test.md")).unwrap(), b"local edit");
            assert_eq!(fs::read(root.join("occupied.md")).unwrap(), b"another note");
            assert!(!root.join("chosen-path.md").exists());
            assert!(!root.join("renamed.md").exists());
        }
    }
}
