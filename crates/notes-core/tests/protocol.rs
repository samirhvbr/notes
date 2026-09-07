//! The 0.1a acceptance criteria that say "teste automatizado no core".
//!
//! Every test here runs with no Tauri, no window and no global state: the data
//! directory is redirected with `NOTES_DATA_DIR`, which is why that override
//! exists (`ARCHITECTURE.md` §4).

use notes_core::{DraftChoice, DraftReason, SaveResult, WorkspaceService};
use notes_model::{CoreError, IoKind, RelPath};
use std::path::PathBuf;

struct Fixture {
    _data: tempfile::TempDir,
    work: tempfile::TempDir,
    svc: WorkspaceService,
}

fn setup() -> Fixture {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("nota.md"), b"# nota\n\ncorpo\n").unwrap();
    std::fs::write(work.path().join("outra.md"), b"# outra\n").unwrap();
    std::fs::create_dir(work.path().join("sub")).unwrap();
    std::fs::write(work.path().join("sub/deep.md"), b"# deep\n").unwrap();

    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(work.path()).unwrap();
    Fixture {
        _data: data,
        work,
        svc,
    }
}

fn rel(s: &str) -> RelPath {
    RelPath::parse(s).unwrap()
}

fn drafts_dir(svc: &WorkspaceService, id: notes_model::WorkspaceId) -> PathBuf {
    notes_core::paths::drafts_dir(&notes_core::paths::workspace_dir(svc.data_dir(), id))
}

// ---------------------------------------------------------------------------
// "Abrir uma pasta não cria nenhum arquivo nela"
// ---------------------------------------------------------------------------

#[test]
fn opening_a_folder_creates_nothing_inside_it() {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("nota.md"), b"# n\n").unwrap();

    let before = snapshot(work.path());
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    let info = svc.open_workspace(work.path()).unwrap();
    svc.list_dir(&RelPath::root()).unwrap();
    svc.open_note(&rel("nota.md")).unwrap();
    let after = snapshot(work.path());

    assert_eq!(
        before, after,
        "the workspace folder was modified by opening it"
    );
    assert!(!info.read_only);
    // The case probe is part of open, and it must not have written either.
    assert!(!work.path().join(".notes").exists());
}

fn snapshot(root: &std::path::Path) -> Vec<(String, u64)> {
    let mut v = Vec::new();
    fn walk(root: &std::path::Path, dir: &std::path::Path, v: &mut Vec<(String, u64)>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let name = p.strip_prefix(root).unwrap().to_string_lossy().to_string();
            if e.file_type().unwrap().is_dir() {
                v.push((name, 0));
                walk(root, &p, v);
            } else {
                v.push((name, e.metadata().unwrap().len()));
            }
        }
    }
    walk(root, root, &mut v);
    v.sort();
    v
}

// ---------------------------------------------------------------------------
// "Nenhum command aceita path resolvido fora da raiz"
// ---------------------------------------------------------------------------

#[test]
fn no_command_accepts_a_path_outside_the_root() {
    let f = setup();
    for bad in ["../escape.md", "sub/../../escape.md", "/etc/passwd"] {
        assert!(RelPath::parse(bad).is_err(), "{bad} must not even parse");
    }

    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.md"), b"# secret\n").unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret.md"),
            f.work.path().join("link.md"),
        )
        .unwrap();
        let mut svc = f.svc;
        assert!(matches!(
            svc.open_note(&rel("link.md")),
            Err(CoreError::SymlinkNotFollowed { .. })
        ));
        assert_eq!(
            std::fs::read(outside.path().join("secret.md")).unwrap(),
            b"# secret\n"
        );
    }
}

// ---------------------------------------------------------------------------
// "Buffer sujo + `echo x >> nota.md` externo → autosave suspende, nada
//  sobrescrito, rascunho em app data"
// ---------------------------------------------------------------------------

#[test]
fn an_external_append_suspends_autosave_and_writes_a_draft_without_overwriting() {
    let mut f = setup();
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();
    let id = opened.note_id;
    let base = opened.base_rev.clone();

    // The user is typing; nothing saved yet.
    let dirty = format!("{}editado pelo usuário\n", opened.text);

    // Somebody else appends. `echo x >> nota.md`, in effect.
    std::thread::sleep(std::time::Duration::from_millis(10));
    std::fs::write(f.work.path().join("nota.md"), b"# nota\n\ncorpo\nx\n").unwrap();

    let result = f.svc.save_note(id, &dirty, 7, &base).unwrap();
    assert!(
        matches!(result, SaveResult::Conflict { .. }),
        "got {result:?}"
    );

    // Nothing overwritten.
    assert_eq!(
        std::fs::read(f.work.path().join("nota.md")).unwrap(),
        b"# nota\n\ncorpo\nx\n",
        "the external change must survive intact"
    );

    // Autosave suspended for this note.
    assert!(f.svc.is_suspended(id));

    // The buffer is recoverable from app data.
    let drafts = f.svc.list_drafts().unwrap();
    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].note_id, id);
    assert_eq!(drafts[0].reason, DraftReason::Conflict);

    let restored = f.svc.resolve_draft(id, DraftChoice::Restore).unwrap();
    assert_eq!(
        restored.text, dirty,
        "the draft holds exactly what was typed"
    );
}

#[test]
fn a_suspended_note_keeps_its_edits_going_to_the_draft() {
    let mut f = setup();
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();
    let id = opened.note_id;
    let base = opened.base_rev.clone();

    std::thread::sleep(std::time::Duration::from_millis(10));
    std::fs::write(f.work.path().join("nota.md"), b"# externo\n").unwrap();
    let _ = f.svc.save_note(id, "meu texto", 1, &base).unwrap();
    assert!(f.svc.is_suspended(id));

    // More typing while suspended: the draft moves, the note does not.
    f.svc
        .write_draft(id, "meu texto continuado", 2, &base, DraftReason::Conflict)
        .unwrap();
    assert_eq!(
        std::fs::read(f.work.path().join("nota.md")).unwrap(),
        b"# externo\n"
    );
    let restored = f.svc.resolve_draft(id, DraftChoice::Restore).unwrap();
    assert_eq!(restored.text, "meu texto continuado");
}

// ---------------------------------------------------------------------------
// "Disco cheio / permissão negada → erro visível, buffer recuperável ao reabrir"
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn a_denied_write_is_reported_and_leaves_the_buffer_recoverable() {
    use std::os::unix::fs::PermissionsExt;
    // Root ignores the permission bits, so the denial this test needs cannot be
    // arranged — as it is in the Arch CI container. Skipping is honest;
    // asserting anyway would make the test pass for the wrong reason.
    if unsafe { libc::geteuid() } == 0 {
        eprintln!("skipped: running as root, which cannot be denied write access");
        return;
    }
    let mut f = setup();
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();
    let id = opened.note_id;
    let base = opened.base_rev.clone();
    let ws_id = f.svc.recent_workspaces().unwrap()[0].id;

    // Make the directory unwritable: the temp file cannot be created.
    let dir = f.work.path();
    let original = std::fs::metadata(dir).unwrap().permissions();
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o555)).unwrap();

    let result = f.svc.save_note(id, "texto que não cabe", 3, &base);
    std::fs::set_permissions(dir, original).unwrap();

    let result = result.unwrap();
    match result {
        SaveResult::WriteFailed { kind, .. } => {
            assert!(
                matches!(kind, IoKind::PermissionDenied | IoKind::ReadOnlyFilesystem),
                "expected a permission failure, got {kind:?}"
            );
        }
        other => panic!("expected WriteFailed, got {other:?}"),
    }

    // Recoverable after a restart: a fresh service over the same data directory.
    let path = drafts_dir(&f.svc, ws_id).join(format!("{id}.draft"));
    assert!(
        path.exists(),
        "a failed write must leave a draft at {}",
        path.display()
    );
    let raw = std::fs::read(&path).unwrap();
    assert!(
        String::from_utf8_lossy(&raw).contains("texto que não cabe"),
        "the draft must hold the buffer verbatim"
    );
}

// ---------------------------------------------------------------------------
// Write protocol
// ---------------------------------------------------------------------------

#[test]
fn saving_unchanged_content_writes_nothing_and_does_not_move_mtime() {
    let mut f = setup();
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();
    let before = std::fs::metadata(f.work.path().join("nota.md")).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));
    let r = f
        .svc
        .save_note(opened.note_id, &opened.text, 1, &opened.base_rev)
        .unwrap();
    assert!(
        matches!(
            r,
            SaveResult::Saved {
                unchanged: true,
                ..
            }
        ),
        "got {r:?}"
    );

    let after = std::fs::metadata(f.work.path().join("nota.md")).unwrap();
    assert_eq!(
        before.modified().unwrap(),
        after.modified().unwrap(),
        "mtime moved"
    );
    assert_eq!(
        std::fs::read(f.work.path().join("nota.md")).unwrap(),
        b"# nota\n\ncorpo\n"
    );
}

#[test]
fn an_ordinary_save_writes_and_clears_the_draft() {
    let mut f = setup();
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();
    let id = opened.note_id;

    f.svc
        .write_draft(id, "rascunho", 1, &opened.base_rev, DraftReason::Stale)
        .unwrap();
    assert_eq!(f.svc.list_drafts().unwrap().len(), 1);

    let r = f
        .svc
        .save_note(id, "# nova\n\nversão\n", 2, &opened.base_rev)
        .unwrap();
    match r {
        SaveResult::Saved {
            unchanged,
            buffer_version,
            ..
        } => {
            assert!(!unchanged);
            assert_eq!(buffer_version, 2);
        }
        other => panic!("expected Saved, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(f.work.path().join("nota.md")).unwrap(),
        "# nova\n\nversão\n".as_bytes()
    );
    assert!(
        f.svc.list_drafts().unwrap().is_empty(),
        "a confirmed write clears the draft"
    );
}

#[test]
fn convergence_is_not_a_conflict() {
    let mut f = setup();
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();

    // Someone else writes exactly what the buffer holds.
    std::thread::sleep(std::time::Duration::from_millis(10));
    std::fs::write(f.work.path().join("nota.md"), b"# convergiu\n").unwrap();

    let r = f
        .svc
        .save_note(opened.note_id, "# convergiu\n", 5, &opened.base_rev)
        .unwrap();
    assert!(
        matches!(
            r,
            SaveResult::Saved {
                unchanged: true,
                ..
            }
        ),
        "got {r:?}"
    );
    assert!(!f.svc.is_suspended(opened.note_id));
}

#[test]
fn a_stale_save_reports_the_version_it_was_given() {
    let mut f = setup();
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();
    let r = f
        .svc
        .save_note(opened.note_id, "novo", 41, &opened.base_rev)
        .unwrap();
    // The frontend paints a tab clean only when this equals its current
    // version, which is what stops an old save clearing a newer buffer.
    match r {
        SaveResult::Saved { buffer_version, .. } => assert_eq!(buffer_version, 41),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_byte_profile_survives_a_round_trip_through_the_service() {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let original: &[u8] = b"\xef\xbb\xbf# crlf com bom\r\n\r\ncorpo\r\n";
    std::fs::write(work.path().join("n.md"), original).unwrap();

    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(work.path()).unwrap();
    let opened = svc.open_note(&rel("n.md")).unwrap();
    assert!(opened.profile.bom);
    assert_eq!(
        opened.text, "# crlf com bom\n\ncorpo\n",
        "the editor sees only \\n"
    );

    svc.save_note(opened.note_id, &opened.text, 1, &opened.base_rev)
        .unwrap();
    assert_eq!(std::fs::read(work.path().join("n.md")).unwrap(), original);
}

#[test]
fn a_read_only_note_refuses_to_be_saved() {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("misto.md"), b"a\nb\r\nc\n").unwrap();
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(work.path()).unwrap();

    let opened = svc.open_note(&rel("misto.md")).unwrap();
    assert_eq!(
        opened.read_only,
        Some(notes_model::ReadOnlyReason::MixedEol)
    );
    let err = svc
        .save_note(opened.note_id, "qualquer coisa", 1, &opened.base_rev)
        .unwrap_err();
    assert!(matches!(err, CoreError::ReadOnly { .. }), "got {err:?}");
    assert_eq!(
        std::fs::read(work.path().join("misto.md")).unwrap(),
        b"a\nb\r\nc\n"
    );
}

// ---------------------------------------------------------------------------
// Registry, tree, workspace
// ---------------------------------------------------------------------------

#[test]
fn listing_assigns_no_identity() {
    let mut f = setup();
    f.svc.list_dir(&RelPath::root()).unwrap();
    f.svc.list_dir(&rel("sub")).unwrap();
    assert!(f.svc.list_drafts().unwrap().is_empty());

    // Opening one note is what creates one record — not listing three.
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;
    let again = f.svc.open_note(&rel("nota.md")).unwrap().note_id;
    assert_eq!(id, again, "reopening a note keeps its identity");
}

#[test]
fn the_tree_hides_the_default_ignore_list_and_shows_notes() {
    let f = setup();
    std::fs::create_dir(f.work.path().join(".git")).unwrap();
    std::fs::write(f.work.path().join(".git/HEAD"), b"ref: x\n").unwrap();
    std::fs::create_dir(f.work.path().join(".obsidian")).unwrap();

    let names: Vec<_> = f
        .svc
        .list_dir(&RelPath::root())
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert!(names.contains(&"nota.md".to_string()));
    assert!(names.contains(&"sub".to_string()));
    assert!(
        !names.contains(&".git".to_string()),
        "IGNORE_DEFAULT applies with no config"
    );
    assert!(!names.contains(&".obsidian".to_string()));
}

#[test]
fn creating_a_note_refuses_a_collision_and_never_overwrites() {
    let mut f = setup();
    let err = f.svc.create_note(&RelPath::root(), "nota").unwrap_err();
    assert!(
        matches!(err, CoreError::AlreadyExists { .. }),
        "got {err:?}"
    );
    assert_eq!(
        std::fs::read(f.work.path().join("nota.md")).unwrap(),
        b"# nota\n\ncorpo\n"
    );

    let e = f.svc.create_note(&RelPath::root(), "terceira").unwrap();
    assert_eq!(e.name, "terceira.md");
    assert_eq!(
        std::fs::read(f.work.path().join("terceira.md")).unwrap(),
        b""
    );
}

#[test]
fn the_last_workspace_comes_back_after_a_restart() {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("n.md"), b"# n\n").unwrap();

    let id = {
        let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
        svc.open_workspace(work.path()).unwrap().id
    };

    // A different service over the same data directory: a restart.
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    let restored = svc
        .restore_last_workspace()
        .unwrap()
        .expect("a workspace was open");
    assert_eq!(
        restored.id, id,
        "the same workspace, with the same identity"
    );
    assert!(restored.restored);
}

#[test]
fn a_workspace_that_moved_is_reported_rather_than_treated_as_absent() {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("n.md"), b"# n\n").unwrap();
    {
        let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
        svc.open_workspace(work.path()).unwrap();
    }
    // The canonical path, not the one handed to `open_workspace`: on macOS
    // `/var` is a symlink to `/private/var`, so a temp directory has two names
    // and the registry stores the resolved one.
    let path = work.path().canonicalize().unwrap();
    drop(work); // the folder disappears

    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    match svc.restore_last_workspace() {
        Err(CoreError::Unavailable { root, .. }) => assert_eq!(root, path.display().to_string()),
        other => panic!("expected Unavailable, got {other:?}"),
    }
}

#[test]
fn closing_refuses_while_buffers_are_dirty() {
    let mut f = setup();
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;
    match f.svc.close_workspace(&[id]) {
        Err(CoreError::DirtyBuffers { count, .. }) => assert_eq!(count, 1),
        other => panic!("expected DirtyBuffers, got {other:?}"),
    }
    f.svc.close_workspace(&[]).unwrap();
}

#[test]
fn state_written_by_a_newer_build_is_never_overwritten() {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("n.md"), b"# n\n").unwrap();

    let ws_id = {
        let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
        svc.open_workspace(work.path()).unwrap().id
    };
    let reg =
        notes_core::paths::registry_file(&notes_core::paths::workspace_dir(data.path(), ws_id));
    let from_the_future = br#"{"schema":999,"a_field_we_do_not_know":true}"#;
    std::fs::write(&reg, from_the_future).unwrap();

    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    let info = svc.open_workspace(work.path()).unwrap();
    assert!(
        info.read_only,
        "a workspace with future state opens read-only"
    );
    assert_eq!(
        info.state_schema_ahead,
        Some(999),
        "the message can say how far ahead"
    );
    assert_eq!(svc.workspace_id(), Some(ws_id));
    assert_eq!(
        std::fs::read(&reg).unwrap(),
        from_the_future,
        "state we cannot interpret must not be destroyed"
    );
}

#[test]
fn creating_a_name_that_breaks_another_platform_is_refused() {
    let mut f = setup();
    for bad in ["trailing-dot.", "a:b", "CON", "nul", "a?b"] {
        let err = f.svc.create_note(&RelPath::root(), bad).unwrap_err();
        assert!(
            matches!(err, CoreError::InvalidPath { .. }),
            "{bad} should be refused, got {err:?}"
        );
    }
    // The rule applies to new names only. Nothing on disk was created.
    let names: Vec<_> = f
        .svc
        .list_dir(&RelPath::root())
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert!(!names.iter().any(|n| n.ends_with('.')));
}

/// An odd name that is *already on disk* is listed, never renamed (scope §7.6).
/// Created at runtime because a trailing dot cannot be committed: git aborts a
/// Windows checkout with `invalid path` before any test runs.
#[cfg(unix)]
#[test]
fn an_existing_name_with_a_trailing_dot_is_listed_and_left_alone() {
    let f = setup();
    let odd = f.work.path().join("legado.md.");
    std::fs::write(&odd, b"# legado\n").unwrap();

    let names: Vec<_> = f
        .svc
        .list_dir(&RelPath::root())
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert!(
        names.contains(&"legado.md.".to_string()),
        "an existing odd name is shown: {names:?}"
    );
    assert!(odd.exists(), "and never renamed");
}
