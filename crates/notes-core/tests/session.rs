//! Milestone 0.1c: *"reabrir o app restaura workspace, abas, aba ativa e
//! cursor"*.
//!
//! The half a core test can own is that the session **survives a restart** with
//! its tabs, its active tab and its cursor positions intact, and that a
//! workspace whose session is unreadable still opens. Putting the cursor back
//! into a mounted editor is the frontend's half, in `stores/tabs.test.ts`.

use notes_core::{Session, Tab, WorkspaceService};
use notes_model::RelPath;

fn rel(s: &str) -> RelPath {
    RelPath::parse(s).unwrap()
}

struct Fixture {
    data: tempfile::TempDir,
    work: tempfile::TempDir,
}

fn fixture() -> Fixture {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    for n in ["um.md", "dois.md", "tres.md"] {
        std::fs::write(work.path().join(n), format!("# {n}\n\ncorpo\n").as_bytes()).unwrap();
    }
    Fixture { data, work }
}

fn open(f: &Fixture) -> WorkspaceService {
    let mut svc = WorkspaceService::with_data_dir(f.data.path()).unwrap();
    svc.open_workspace(f.work.path()).unwrap();
    svc
}

#[test]
fn tabs_the_active_tab_and_the_cursor_survive_a_restart() {
    let f = fixture();
    let (ids, saved) = {
        let mut svc = open(&f);
        let a = svc.open_note(&rel("um.md")).unwrap().note_id;
        let b = svc.open_note(&rel("dois.md")).unwrap().note_id;

        let session = Session {
            tabs: vec![
                Tab {
                    note_id: a,
                    path: rel("um.md"),
                    line: 1,
                    col: 1,
                    scroll_top: 0,
                    pinned: false,
                },
                Tab {
                    note_id: b,
                    path: rel("dois.md"),
                    line: 42,
                    col: 7,
                    scroll_top: 640,
                    pinned: false,
                },
            ],
            active_tab: Some(b),
            ..Session::default()
        };
        svc.save_session(&session).unwrap();
        ((a, b), session)
    };

    // A different service over the same data directory: a restart.
    let mut svc = WorkspaceService::with_data_dir(f.data.path()).unwrap();
    let info = svc
        .restore_last_workspace()
        .unwrap()
        .expect("a workspace was open");
    assert!(info.restored);

    let back = svc.session().unwrap();
    assert_eq!(back.tabs.len(), 2);
    assert_eq!(
        back.active_tab,
        Some(ids.1),
        "the active tab is the one that was active"
    );

    let second = &back.tabs[1];
    assert_eq!(second.note_id, ids.1);
    assert_eq!(second.path, rel("dois.md"));
    assert_eq!(
        (second.line, second.col),
        (42, 7),
        "the cursor is where it was left"
    );
    assert_eq!(second.scroll_top, 640);
    assert_eq!(back.view_mode, saved.view_mode);
}

#[test]
fn a_note_keeps_its_identity_across_the_restart_so_a_tab_can_find_it() {
    let f = fixture();
    let before = {
        let mut svc = open(&f);
        svc.open_note(&rel("um.md")).unwrap().note_id
    };
    let mut svc = WorkspaceService::with_data_dir(f.data.path()).unwrap();
    svc.restore_last_workspace().unwrap().unwrap();
    let after = svc.open_note(&rel("um.md")).unwrap().note_id;
    assert_eq!(
        before, after,
        "a tab is addressed by NoteId, so it must not move"
    );
}

#[test]
fn an_unreadable_session_starts_empty_rather_than_refusing_to_open() {
    let f = fixture();
    let id = {
        let mut svc = open(&f);
        let note_id = svc.open_note(&rel("um.md")).unwrap().note_id;
        svc.save_session(&Session {
            tabs: vec![Tab {
                note_id,
                path: rel("um.md"),
                line: 3,
                col: 1,
                scroll_top: 0,
                pinned: false,
            }],
            ..Session::default()
        })
        .unwrap();
        svc.workspace_id().unwrap()
    };

    let path =
        notes_core::paths::session_file(&notes_core::paths::workspace_dir(f.data.path(), id));
    std::fs::write(&path, b"{ this is not json").unwrap();

    let mut svc = WorkspaceService::with_data_dir(f.data.path()).unwrap();
    svc.restore_last_workspace()
        .unwrap()
        .expect("the workspace still opens");
    let s = svc.session().unwrap();
    assert!(
        s.tabs.is_empty(),
        "an empty session, not a failure to start"
    );
    assert_eq!(s.active_tab, None);
}

#[test]
fn a_session_from_a_newer_build_is_replaced_rather_than_breaking_the_open() {
    let f = fixture();
    let id = {
        let svc = open(&f);
        svc.save_session(&Session::default()).unwrap();
        svc.workspace_id().unwrap()
    };
    let path =
        notes_core::paths::session_file(&notes_core::paths::workspace_dir(f.data.path(), id));
    std::fs::write(&path, br#"{"schema":999,"tabs":[],"unknown":true}"#).unwrap();

    let mut svc = WorkspaceService::with_data_dir(f.data.path()).unwrap();
    svc.restore_last_workspace().unwrap().unwrap();
    // Session is resettable UI state, unlike the registry: a future one costs
    // an empty session rather than a read-only workspace.
    assert!(svc.session().unwrap().tabs.is_empty());
}

#[test]
fn saving_a_session_does_not_touch_the_users_folder() {
    let f = fixture();
    let before = snapshot(f.work.path());
    let svc = open(&f);
    svc.save_session(&Session {
        tabs: vec![Tab {
            note_id: notes_model::NoteId::new(),
            path: rel("um.md"),
            line: 9,
            col: 2,
            scroll_top: 0,
            pinned: true,
        }],
        ..Session::default()
    })
    .unwrap();
    assert_eq!(
        before,
        snapshot(f.work.path()),
        "session state lives in app data"
    );
}

fn snapshot(root: &std::path::Path) -> Vec<(String, u64)> {
    let mut v: Vec<(String, u64)> = std::fs::read_dir(root)
        .unwrap()
        .flatten()
        .map(|e| {
            (
                e.file_name().to_string_lossy().to_string(),
                e.metadata().map(|m| m.len()).unwrap_or(0),
            )
        })
        .collect();
    v.sort();
    v
}
