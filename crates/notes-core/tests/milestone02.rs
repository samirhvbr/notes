use notes_core::WorkspaceService;
use notes_model::RelPath;
use std::{
    fs, thread,
    time::{Duration, Instant},
};
fn p(s: &str) -> RelPath {
    RelPath::parse(s).unwrap()
}
fn indexed(s: &mut WorkspaceService, force: bool) -> notes_core::content_index::IndexStatus {
    s.index_start(force).unwrap();
    let start = Instant::now();
    loop {
        let status = s.index_status().unwrap();
        if !status.running {
            assert!(status.error.is_none(), "{:?}", status);
            return status;
        }
        assert!(start.elapsed() < Duration::from_secs(20));
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn incremental_rebuild_preserves_bytes_and_identity_and_recents() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let bytes = b"\xef\xbb\xbf# Test\r\naction word\r\n";
    fs::write(root.path().join("a.md"), bytes).unwrap();
    let mut s = WorkspaceService::with_data_dir(data.path()).unwrap();
    let ws = s.open_workspace(root.path()).unwrap();
    let first = s.open_note(&p("a.md")).unwrap();
    assert_eq!(indexed(&mut s, false).indexed, 1);
    assert_eq!(indexed(&mut s, false).unchanged, 1);
    assert_eq!(s.word_hits("action").unwrap().0.len(), 1);
    assert_eq!(indexed(&mut s, true).indexed, 1);
    assert_eq!(s.open_note(&p("a.md")).unwrap().note_id, first.note_id);
    assert_eq!(fs::read(root.path().join("a.md")).unwrap(), bytes);
    let dir = data.path().join("workspaces").join(ws.id.to_string());
    assert!(dir.join("registry.db").exists());
    drop(s);
    fs::remove_file(dir.join("index.db")).unwrap();
    let mut s = WorkspaceService::with_data_dir(data.path()).unwrap();
    s.open_workspace(root.path()).unwrap();
    indexed(&mut s, false);
    assert_eq!(s.open_note(&p("a.md")).unwrap().note_id, first.note_id);
    assert_eq!(s.recent_notes().unwrap()[0].note_id, first.note_id);
    fs::write(root.path().join("a.md"), "replacement").unwrap();
    indexed(&mut s, false);
    assert!(s.word_hits("action").unwrap().0.is_empty());
    assert_eq!(s.word_hits("replacement").unwrap().0.len(), 1);
    fs::remove_file(root.path().join("a.md")).unwrap();
    indexed(&mut s, false);
    assert!(s.word_hits("replacement").unwrap().0.is_empty());
}
#[test]
fn reviewed_move_updates_both_directions_and_keeps_backup() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("folder")).unwrap();
    fs::create_dir(root.path().join("next")).unwrap();
    fs::write(root.path().join("folder/a.md"), "[b](../b.md)\n").unwrap();
    fs::write(root.path().join("b.md"), "[a](folder/a.md#h)\n").unwrap();
    let mut s = WorkspaceService::with_data_dir(data.path()).unwrap();
    s.open_workspace(root.path()).unwrap();
    let id = s.open_note(&p("folder/a.md")).unwrap().note_id;
    indexed(&mut s, false);
    let plan = s
        .reference_preview(&p("folder/a.md"), &p("next/renamed.md"))
        .unwrap();
    assert!(!plan.files.is_empty());
    let result = s
        .reference_apply(&plan.token, &(0..plan.files.len()).collect::<Vec<_>>())
        .unwrap();
    assert!(result.failed.is_empty());
    assert_eq!(
        fs::read_to_string(root.path().join("b.md")).unwrap(),
        "[a](next/renamed.md#h)\n"
    );
    assert!(std::path::Path::new(&result.backup)
        .join("plan.json")
        .exists());
    assert_eq!(s.open_note(&p("next/renamed.md")).unwrap().note_id, id);
}
#[test]
fn changed_review_refuses_before_moving() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a.md"), "[b](b.md)").unwrap();
    fs::write(root.path().join("b.md"), "target").unwrap();
    let mut s = WorkspaceService::with_data_dir(data.path()).unwrap();
    s.open_workspace(root.path()).unwrap();
    indexed(&mut s, false);
    let plan = s.reference_preview(&p("b.md"), &p("c.md")).unwrap();
    fs::write(root.path().join("a.md"), "external").unwrap();
    assert!(s.reference_apply(&plan.token, &[0]).is_err());
    assert!(root.path().join("b.md").exists());
    assert!(!root.path().join("c.md").exists());
}
#[test]
fn legacy_registry_migration_keeps_identity_and_a_backup() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a.md"), "text").unwrap();
    let mut s = WorkspaceService::with_data_dir(data.path()).unwrap();
    let ws = s.open_workspace(root.path()).unwrap();
    let note = s.open_note(&p("a.md")).unwrap();
    drop(s);
    let dir = data.path().join("workspaces").join(ws.id.to_string());
    let db = dir.join("registry.db");
    let bytes = notes_index::RegistryStore::open(&db)
        .unwrap()
        .read()
        .unwrap()
        .unwrap();
    fs::write(dir.join("registry.json"), &bytes).unwrap();
    fs::remove_file(db).unwrap();
    let mut s = WorkspaceService::with_data_dir(data.path()).unwrap();
    s.open_workspace(root.path()).unwrap();
    assert_eq!(s.open_note(&p("a.md")).unwrap().note_id, note.note_id);
    assert_eq!(fs::read(dir.join("registry.json.bak-1")).unwrap(), bytes);
}

#[test]
fn a_corrupt_derived_index_can_be_rebuilt_without_losing_notes() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a.md"), "recoverable").unwrap();
    let mut s = WorkspaceService::with_data_dir(data.path()).unwrap();
    let ws = s.open_workspace(root.path()).unwrap();
    let note = s.open_note(&p("a.md")).unwrap();
    let dir = data.path().join("workspaces").join(ws.id.to_string());
    fs::write(dir.join("index.db"), "broken").unwrap();
    indexed(&mut s, true);
    assert_eq!(s.word_hits("recoverable").unwrap().0.len(), 1);
    assert_eq!(s.open_note(&p("a.md")).unwrap().note_id, note.note_id);
}

#[test]
#[ignore = "read-only performance measurement on NOTES_BENCH_ROOT"]
fn measure_real_workspace_incremental_index() {
    let root = std::env::var("NOTES_BENCH_ROOT").unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut service = WorkspaceService::with_data_dir(data.path()).unwrap();
    service.open_workspace(std::path::Path::new(&root)).unwrap();
    for pass in 1..=2 {
        let start = Instant::now();
        service.index_start(false).unwrap();
        loop {
            let status = service.index_status().unwrap();
            if !status.running {
                assert!(status.error.is_none(), "{status:?}");
                println!(
                    "pass={pass} elapsed_ms={} scanned={} indexed={} unchanged={} skipped={}",
                    start.elapsed().as_millis(),
                    status.scanned,
                    status.indexed,
                    status.unchanged,
                    status.skipped
                );
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(300));
            thread::sleep(Duration::from_millis(50));
        }
    }
}

#[test]
fn review_selection_preserves_unselected_bytes_and_updates_outgoing_links() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("folder")).unwrap();
    fs::write(root.path().join("a.md"), b"\xef\xbb\xbf[b](b.md)\r\n").unwrap();
    fs::write(root.path().join("b.md"), "[a](a.md)\n").unwrap();
    fs::write(root.path().join("invalid.md"), [255]).unwrap();
    let mut s = WorkspaceService::with_data_dir(data.path()).unwrap();
    s.open_workspace(root.path()).unwrap();
    indexed(&mut s, false);
    let plan = s.reference_preview(&p("a.md"), &p("folder/a.md")).unwrap();
    assert!(plan.skipped > 0);
    let selected = plan.files.iter().position(|f| f.path == p("a.md")).unwrap();
    let result = s.reference_apply(&plan.token, &[selected]).unwrap();
    assert!(result.failed.is_empty());
    assert_eq!(
        fs::read(root.path().join("folder/a.md")).unwrap(),
        b"\xef\xbb\xbf[b](../b.md)\r\n"
    );
    assert_eq!(
        fs::read_to_string(root.path().join("b.md")).unwrap(),
        "[a](a.md)\n"
    );
}
