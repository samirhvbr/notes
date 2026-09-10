use notes_core::{sync::apply_received, WorkspaceService};
use notes_model::{CoreError, RelPath};
use std::{fs, path::Path};

fn path() -> RelPath {
    RelPath::parse("nested/note.md").unwrap()
}

#[test]
fn raw_creation_update_and_retry_preserve_bytes() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let original = b"\xef\xbb\xbfhello\r\n\xff";
    let first = apply_received(
        root.path(),
        data.path(),
        &path(),
        original,
        None,
        false,
        || Ok(()),
    )
    .unwrap();
    assert_eq!(
        fs::read(root.path().join(path().as_str())).unwrap(),
        original
    );
    let changed = b"changed\r\n\xfe";
    let second = apply_received(
        root.path(),
        data.path(),
        &path(),
        changed,
        Some(&first),
        false,
        || Ok(()),
    )
    .unwrap();
    let retried = apply_received(
        root.path(),
        data.path(),
        &path(),
        changed,
        Some(&first),
        true,
        || Ok(()),
    )
    .unwrap();
    assert_eq!(second.base_rev, retried.base_rev);
    assert_eq!(second.note_id, retried.note_id);
    fs::write(root.path().join(path().as_str()), b"local work").unwrap();
    assert!(apply_received(
        root.path(),
        data.path(),
        &path(),
        changed,
        Some(&first),
        true,
        || panic!("must not prepare")
    )
    .is_err());
    assert_eq!(
        fs::read(root.path().join(path().as_str())).unwrap(),
        b"local work"
    );
}

#[test]
fn preexisting_equal_content_is_not_an_authorized_retry() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("nested")).unwrap();
    fs::write(root.path().join(path().as_str()), b"same").unwrap();
    for _ in 0..2 {
        assert!(apply_received(
            root.path(),
            data.path(),
            &path(),
            b"same",
            None,
            false,
            || panic!("must not prepare")
        )
        .is_err());
    }
}

#[test]
fn failed_intent_persistence_creates_no_source_entry() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    assert!(apply_received(
        root.path(),
        data.path(),
        &path(),
        b"new",
        None,
        false,
        || Err(CoreError::LockTimeout)
    )
    .is_err());
    assert!(!root.path().join("nested").exists());
}

#[test]
fn any_pending_draft_blocks_application() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut service = WorkspaceService::with_data_dir(data.path()).unwrap();
    service.open_workspace(root.path()).unwrap();
    let workspaces = data.path().join("workspaces");
    let workspace = fs::read_dir(workspaces)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let drafts = workspace.join("drafts");
    fs::create_dir_all(&drafts).unwrap();
    let draft = drafts.join("unfinished.json");
    fs::write(&draft, b"even a damaged draft must survive").unwrap();
    drop(service);
    assert!(apply_received(
        root.path(),
        data.path(),
        &path(),
        b"new",
        None,
        false,
        || panic!("must not prepare")
    )
    .is_err());
    assert_eq!(
        fs::read(draft).unwrap(),
        b"even a damaged draft must survive"
    );
    assert!(!root.path().join(path().as_str()).exists());
}

#[test]
fn activity_child() {
    let Some(root) = std::env::var_os("NOTES_TEST_ACTIVE_ROOT") else {
        return;
    };
    let data = std::env::var_os("NOTES_TEST_ACTIVE_DATA").unwrap();
    let mut service = WorkspaceService::with_data_dir(Path::new(&data)).unwrap();
    service.open_workspace(Path::new(&root)).unwrap();
    fs::write(Path::new(&data).join("ready"), b"").unwrap();
    let until = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !Path::new(&data).join("stop").exists() && std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn another_process_with_an_open_workspace_blocks_writes() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "activity_child", "--nocapture"])
        .env("NOTES_TEST_ACTIVE_ROOT", root.path())
        .env("NOTES_TEST_ACTIVE_DATA", data.path())
        .spawn()
        .unwrap();
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !data.path().join("ready").exists() && std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(data.path().join("ready").exists());
    let result = apply_received(
        root.path(),
        data.path(),
        &path(),
        b"new",
        None,
        false,
        || panic!("must not prepare"),
    );
    fs::write(data.path().join("stop"), b"").unwrap();
    assert!(child.wait().unwrap().success());
    assert!(matches!(result, Err(CoreError::LockTimeout)));
    apply_received(
        root.path(),
        data.path(),
        &path(),
        b"new",
        None,
        false,
        || Ok(()),
    )
    .unwrap();
}
