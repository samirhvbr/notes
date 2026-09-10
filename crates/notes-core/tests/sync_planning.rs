use notes_core::sync::inventory;
use notes_model::RelPath;
use std::{fs, process::Command};
#[test]
fn explicit_inventory_preserves_raw_bytes_identity_and_note_visit_history() {
    let work = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let bytes = b"\xef\xbb\xbf# Exact\r\nlast line";
    fs::write(work.path().join("a.md"), bytes).unwrap();
    fs::write(work.path().join("binary.md"), [0xff, 0x00, 0xfe]).unwrap();
    let mut svc = notes_core::WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(work.path()).unwrap();
    let id = svc
        .open_note(&RelPath::parse("a.md").unwrap())
        .unwrap()
        .note_id;
    let before = svc.recent_notes().unwrap();
    drop(svc);
    let files = inventory(work.path(), data.path()).unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].note, id);
    assert_eq!(files[0].content, notes_fs::hash(bytes));
    assert_eq!(fs::read(work.path().join("a.md")).unwrap(), bytes);
    assert_eq!(
        fs::read(work.path().join("binary.md")).unwrap(),
        [0xff, 0x00, 0xfe]
    );
    let mut svc = notes_core::WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(work.path()).unwrap();
    assert_eq!(
        serde_json::to_value(svc.recent_notes().unwrap()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
}
#[test]
fn inventory_follows_an_unambiguous_external_rename() {
    let work = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::write(work.path().join("a.md"), "rename").unwrap();
    let before = inventory(work.path(), data.path()).unwrap();
    fs::rename(work.path().join("a.md"), work.path().join("renamed.md")).unwrap();
    let after = inventory(work.path(), data.path()).unwrap();
    assert_eq!(before[0].note, after[0].note);
    assert_eq!(after[0].path.as_str(), "renamed.md");
}
#[test]
fn rejected_state_location_creates_nothing_in_the_workspace() {
    let work = tempfile::tempdir().unwrap();
    let bad = work.path().join("not-created/state");
    assert!(inventory(work.path(), &bad).is_err());
    assert!(!work.path().join("not-created").exists());
}
#[test]
fn real_cli_reports_conflicts_without_changing_either_folder() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    fs::write(a.join("same.md"), b"left\r\n").unwrap();
    fs::write(b.join("same.md"), b"right\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_notes-sync-plan"))
        .arg("reconcile")
        .arg(&a)
        .arg(&b)
        .arg(dir.path().join("state"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plan: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(plan[0]["action"], "conflict");
    assert_eq!(fs::read(a.join("same.md")).unwrap(), b"left\r\n");
    assert_eq!(fs::read(b.join("same.md")).unwrap(), b"right\n");
    let output = Command::new(env!("CARGO_BIN_EXE_notes-sync-plan"))
        .arg("upload")
        .arg(&a)
        .arg(&b)
        .arg(dir.path().join("state"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(fs::read(b.join("same.md")).unwrap(), b"right\n");
}
