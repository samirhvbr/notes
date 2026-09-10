use notes_core::{
    sync::{apply_in_workspace, BufferSnapshot},
    WorkspaceService,
};
use notes_model::{CoreError, RelPath};
use std::fs;

fn path() -> RelPath {
    RelPath::parse("note.md").unwrap()
}
fn snapshot(service: &mut WorkspaceService) -> BufferSnapshot {
    let note = service.open_note(&path()).unwrap();
    BufferSnapshot {
        note_id: note.note_id,
        base_rev: note.base_rev,
        buffer_version: 3,
        saved_version: 3,
    }
}

#[test]
fn an_exclusive_open_session_applies_and_reloads_clean_notes() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut service = WorkspaceService::with_data_dir(data.path()).unwrap();
    let ws = service.open_sync_workspace(root.path()).unwrap();
    let first = apply_in_workspace(
        &mut service,
        &path(),
        b"first\r\n",
        None,
        false,
        &[],
        || Ok(()),
    )
    .unwrap();
    let clean = snapshot(&mut service);
    let second = apply_in_workspace(
        &mut service,
        &path(),
        b"second\r\n",
        Some(&first),
        false,
        &[clean],
        || Ok(()),
    )
    .unwrap();
    let reloaded = service.reload_note(first.note_id).unwrap();
    assert_eq!(reloaded.base_rev, second.base_rev);
    assert_eq!(reloaded.text, "second\n");
    assert_eq!(service.workspace_id(), Some(ws.id));
    assert_eq!(
        fs::read(root.path().join("note.md")).unwrap(),
        b"second\r\n"
    );
    let mut peer = WorkspaceService::with_data_dir(data.path()).unwrap();
    assert!(matches!(
        peer.open_workspace(root.path()),
        Err(CoreError::LockTimeout)
    ));
    service.close_workspace(&[]).unwrap();
    peer.open_workspace(root.path()).unwrap();
}

#[test]
fn dirty_inactive_and_stale_clean_buffers_prevent_intent() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut service = WorkspaceService::with_data_dir(data.path()).unwrap();
    service.open_sync_workspace(root.path()).unwrap();
    let receipt =
        apply_in_workspace(&mut service, &path(), b"first", None, false, &[], || Ok(())).unwrap();
    let mut dirty = snapshot(&mut service);
    dirty.buffer_version += 1;
    let unrelated = RelPath::parse("new.md").unwrap();
    assert!(apply_in_workspace(
        &mut service,
        &unrelated,
        b"new",
        None,
        false,
        &[dirty],
        || panic!("must not prepare")
    )
    .is_err());
    assert!(!root.path().join("new.md").exists());
    let clean = snapshot(&mut service);
    fs::write(root.path().join("note.md"), b"local edit").unwrap();
    assert!(apply_in_workspace(
        &mut service,
        &path(),
        b"remote",
        Some(&receipt),
        false,
        &[clean],
        || panic!("must not prepare")
    )
    .is_err());
    assert_eq!(
        fs::read(root.path().join("note.md")).unwrap(),
        b"local edit"
    );
}

#[test]
fn shared_sessions_cannot_apply_or_upgrade_away_their_lease() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut service = WorkspaceService::with_data_dir(data.path()).unwrap();
    service.open_workspace(root.path()).unwrap();
    assert!(service.open_sync_workspace(root.path()).is_err());
    assert!(
        apply_in_workspace(&mut service, &path(), b"new", None, false, &[], || panic!(
            "must not prepare"
        ))
        .is_err()
    );
    let mut peer = WorkspaceService::with_data_dir(data.path()).unwrap();
    assert!(matches!(
        peer.open_sync_workspace(root.path()),
        Err(CoreError::LockTimeout)
    ));
    assert!(service.workspace_id().is_some());
    service.close_workspace(&[]).unwrap();
    // A failed exclusive open must not turn subsequent ordinary opens exclusive.
    peer.open_workspace(root.path()).unwrap();
    service.open_workspace(root.path()).unwrap();
}

#[test]
fn drafts_and_failed_intents_survive_an_open_session() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut service = WorkspaceService::with_data_dir(data.path()).unwrap();
    let ws = service.open_sync_workspace(root.path()).unwrap();
    assert!(
        apply_in_workspace(&mut service, &path(), b"new", None, false, &[], || Err(
            CoreError::LockTimeout
        ))
        .is_err()
    );
    assert!(!root.path().join("note.md").exists());
    let drafts =
        notes_core::paths::drafts_dir(&notes_core::paths::workspace_dir(data.path(), ws.id));
    fs::create_dir_all(&drafts).unwrap();
    fs::write(drafts.join("pending"), b"unsaved text").unwrap();
    assert!(
        apply_in_workspace(&mut service, &path(), b"new", None, false, &[], || panic!(
            "must not prepare"
        ))
        .is_err()
    );
    assert_eq!(fs::read(drafts.join("pending")).unwrap(), b"unsaved text");
    assert!(!root.path().join("note.md").exists());
}

#[test]
fn exclusive_state_must_stay_outside_the_source() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("private-state");
    let mut service = WorkspaceService::with_data_dir(&data).unwrap();
    assert!(service.open_sync_workspace(root.path()).is_err());
    assert!(service.workspace_id().is_none());
    assert!(!data.join("workspaces").exists());
}
