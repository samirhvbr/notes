//! Reconciliation and identity correlation — `ARCHITECTURE.md` §8 and §9, and
//! the 0.1b acceptance criteria that depend on them.
//!
//! Every test here drives the reconciler directly with a set of hints, exactly
//! as the shell does with what the watcher reported. The watcher itself is
//! exercised by `a_real_watcher_reports_a_change_within_the_debounce`, which is
//! the only test in this file that sleeps.

use std::collections::BTreeSet;
use std::time::Duration;

use notes_core::{ChangeKind, ConflictKind, CoreEvent, WorkspaceService};
use notes_model::{NoteId, RelPath};

struct Fixture {
    _data: tempfile::TempDir,
    work: tempfile::TempDir,
    svc: WorkspaceService,
}

fn rel(s: &str) -> RelPath {
    RelPath::parse(s).unwrap()
}

fn setup() -> Fixture {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("nota.md"), b"# nota\n").unwrap();
    std::fs::write(work.path().join("outra.md"), b"# outra\n").unwrap();
    std::fs::create_dir(work.path().join("sub")).unwrap();
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(work.path()).unwrap();
    Fixture {
        _data: data,
        work,
        svc,
    }
}

/// A full scan — what a window regaining focus triggers.
fn scan(f: &mut Fixture, dirty: &[NoteId]) -> Vec<CoreEvent> {
    f.svc.reconcile(&BTreeSet::new(), dirty).unwrap().events
}

fn hint(f: &mut Fixture, paths: &[&str], dirty: &[NoteId]) -> Vec<CoreEvent> {
    let set: BTreeSet<RelPath> = paths.iter().map(|p| rel(p)).collect();
    f.svc.reconcile(&set, dirty).unwrap().events
}

/// `mtime` has a resolution, and two writes in the same nanosecond would make
/// the cheap check pass when the content had changed. Real editors are slower
/// than this loop.
fn write(f: &Fixture, name: &str, body: &str) {
    std::thread::sleep(Duration::from_millis(5));
    std::fs::write(f.work.path().join(name), body.as_bytes()).unwrap();
}

// ---------------------------------------------------------------------------
// "Disco mudou, buffer limpo → recarrega" (scope §12)
// ---------------------------------------------------------------------------

#[test]
fn an_external_change_to_a_clean_buffer_is_a_reload_not_a_conflict() {
    let mut f = setup();
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;
    write(&f, "nota.md", "# mudou por fora\n");

    let events = hint(&mut f, &["nota.md"], &[]);
    assert_eq!(
        events,
        vec![CoreEvent::FsChanged {
            path: rel("nota.md"),
            kind: ChangeKind::Modified,
            note_id: Some(id),
        }]
    );
    // The registry learned the new bytes, so a second tick says nothing.
    assert!(hint(&mut f, &["nota.md"], &[]).is_empty());
}

// ---------------------------------------------------------------------------
// "Disco mudou, buffer sujo → suspende autosave, aba em conflito"
// ---------------------------------------------------------------------------

#[test]
fn an_external_change_under_a_dirty_buffer_suspends_autosave() {
    let mut f = setup();
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;
    write(&f, "nota.md", "# eles escreveram\n");

    let events = hint(&mut f, &["nota.md"], &[id]);
    assert_eq!(
        events,
        vec![CoreEvent::NoteConflict {
            note_id: id,
            kind: ConflictKind::Modified,
        }]
    );
    assert!(f.svc.is_suspended(id), "autosave must be suspended");
    assert_eq!(
        std::fs::read_to_string(f.work.path().join("nota.md")).unwrap(),
        "# eles escreveram\n",
        "reconciliation never writes"
    );
}

/// "Arquivo removido externamente com edição local → preserva buffer, oferece
/// recuperar; não recria o path sozinho."
#[test]
fn a_removal_under_a_dirty_buffer_keeps_the_note_in_the_registry() {
    let mut f = setup();
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;
    std::fs::remove_file(f.work.path().join("nota.md")).unwrap();

    let events = scan(&mut f, &[id]);
    assert_eq!(
        events,
        vec![CoreEvent::NoteConflict {
            note_id: id,
            kind: ConflictKind::Removed,
        }]
    );
    assert!(f.svc.is_suspended(id));
    assert!(
        !f.work.path().join("nota.md").exists(),
        "the application never recreates a path on its own"
    );
    // The record survives, because the buffer still needs an identity to be
    // resolved against.
    assert!(f.svc.reload_note(id).is_err());
}

#[test]
fn a_removal_with_no_buffer_at_risk_is_reported_and_forgotten() {
    let mut f = setup();
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;
    std::fs::remove_file(f.work.path().join("nota.md")).unwrap();

    let events = scan(&mut f, &[]);
    assert_eq!(
        events,
        vec![CoreEvent::FsChanged {
            path: rel("nota.md"),
            kind: ChangeKind::Removed,
            note_id: Some(id),
        }]
    );
}

// ---------------------------------------------------------------------------
// The self-write filter
// ---------------------------------------------------------------------------

/// The application's own save must not come back as news. Without this, every
/// autosave would look like an external change one tick later.
#[test]
fn our_own_save_is_not_reported_as_an_external_change() {
    let mut f = setup();
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();
    std::thread::sleep(Duration::from_millis(5));
    f.svc
        .save_note(opened.note_id, "# eu escrevi\n", 1, &opened.base_rev)
        .unwrap();

    assert!(
        hint(&mut f, &["nota.md"], &[opened.note_id]).is_empty(),
        "the application watching itself work is not news"
    );
}

/// …and the expectation is consumed on its first match, so **an external write
/// of the same bytes right afterwards is still seen** (`ARCHITECTURE.md` §8).
#[test]
fn an_external_write_after_ours_is_seen_rather_than_swallowed() {
    let mut f = setup();
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();
    std::thread::sleep(Duration::from_millis(5));
    f.svc
        .save_note(opened.note_id, "# eu escrevi\n", 1, &opened.base_rev)
        .unwrap();
    // Consume the expectation with our own event…
    hint(&mut f, &["nota.md"], &[]);
    // …then someone else writes something different.
    write(&f, "nota.md", "# eles escreveram\n");

    let events = hint(&mut f, &["nota.md"], &[opened.note_id]);
    assert_eq!(
        events,
        vec![CoreEvent::NoteConflict {
            note_id: opened.note_id,
            kind: ConflictKind::Modified,
        }]
    );
}

// ---------------------------------------------------------------------------
// Identity correlation (§9), and the acceptance criterion built on it
// ---------------------------------------------------------------------------

/// "rename externo não ambíguo reconecta". On a filesystem with native ids this
/// is rule 1; the hash is the fallback.
#[test]
fn an_unambiguous_external_rename_reconnects_the_same_note() {
    let mut f = setup();
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;
    std::fs::rename(
        f.work.path().join("nota.md"),
        f.work.path().join("sub/renomeada.md"),
    )
    .unwrap();

    let events = scan(&mut f, &[]);
    assert_eq!(
        events,
        vec![CoreEvent::NoteMoved {
            note_id: id,
            from: rel("nota.md"),
            to: rel("sub/renomeada.md"),
        }]
    );
    assert_eq!(
        f.svc.reload_note(id).unwrap().path,
        rel("sub/renomeada.md"),
        "the tab follows the file rather than resetting"
    );
}

/// "ambíguo gera id novo". Two files with the same content, one vanished note:
/// the hash cannot say which, and §9 says a new identity is cheaper than the
/// wrong one.
#[test]
fn an_ambiguous_external_rename_yields_a_new_identity() {
    let mut f = setup();
    write(&f, "nota.md", "identico\n");
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;

    // Two candidates with the same bytes appear, and the original goes.
    std::fs::write(f.work.path().join("sub/a.md"), b"identico\n").unwrap();
    std::fs::write(f.work.path().join("sub/b.md"), b"identico\n").unwrap();
    std::fs::remove_file(f.work.path().join("nota.md")).unwrap();

    let events = scan(&mut f, &[]);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, CoreEvent::NoteMoved { .. })),
        "ambiguity must not guess: {events:?}"
    );
    assert!(events.iter().any(|e| matches!(
        e,
        CoreEvent::FsChanged {
            kind: ChangeKind::Removed,
            note_id: Some(gone),
            ..
        } if *gone == id
    )));
    // Opening one of them gives it an identity of its own.
    assert_ne!(f.svc.open_note(&rel("sub/a.md")).unwrap().note_id, id);
}

/// A zero-byte file is never correlated by hash: every empty file has the same
/// digest, so the signal is no signal at all (§9).
#[test]
fn an_empty_file_is_never_correlated_by_content() {
    let mut f = setup();
    std::fs::write(f.work.path().join("vazia.md"), b"").unwrap();
    let id = f.svc.open_note(&rel("vazia.md")).unwrap().note_id;
    // Keep both files alive until the replacement exists. Otherwise a filesystem
    // may reuse the inode, exercising native identity instead of content matching.
    std::fs::write(f.work.path().join("sub/outra-vazia.md"), b"").unwrap();
    std::fs::remove_file(f.work.path().join("vazia.md")).unwrap();

    let events = scan(&mut f, &[]);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, CoreEvent::NoteMoved { .. })),
        "two empty files are not the same note: {events:?}"
    );
    assert_ne!(
        f.svc.open_note(&rel("sub/outra-vazia.md")).unwrap().note_id,
        id
    );
}

/// A rename **the application performs** never enters correlation at all: the
/// registry was updated directly, so there is nothing vanished to reconnect.
#[test]
fn a_rename_the_app_performed_produces_no_correlation_work() {
    let mut f = setup();
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;
    f.svc.rename_entry(&rel("nota.md"), "renomeada.md").unwrap();

    let events = scan(&mut f, &[]);
    assert!(
        !events.iter().any(|e| matches!(
            e,
            CoreEvent::NoteMoved { .. }
                | CoreEvent::FsChanged {
                    kind: ChangeKind::Removed,
                    ..
                }
        )),
        "{events:?}"
    );
    assert_eq!(f.svc.reload_note(id).unwrap().path, rel("renomeada.md"));
}

// ---------------------------------------------------------------------------
// The rest of §8
// ---------------------------------------------------------------------------

/// A file the watcher just saw appear is news, and it arrives **without an
/// identity**: listing never assigns one (`DECISIONS-0.1a.md` D-09), opening
/// does.
#[test]
fn a_hinted_new_file_is_reported_without_being_given_an_identity() {
    let mut f = setup();
    write(&f, "nova.md", "# nova\n");

    let events = hint(&mut f, &["nova.md"], &[]);
    assert_eq!(
        events,
        vec![CoreEvent::FsChanged {
            path: rel("nova.md"),
            kind: ChangeKind::Created,
            note_id: None,
        }]
    );
    assert!(f.svc.open_note(&rel("nova.md")).is_ok());
}

/// A **full scan** says nothing about files nobody has opened, and that is the
/// point: the registry is lazy, so on a scan every unopened note is "unknown".
/// Announcing them would report the whole workspace as created on every window
/// focus.
#[test]
fn a_full_scan_does_not_announce_every_note_nobody_has_opened() {
    let mut f = setup();
    write(&f, "nova.md", "# nova\n");
    assert!(scan(&mut f, &[]).is_empty());
}

/// A touch that does not change the content updates the registry and says
/// nothing: `touch nota.md` is not an edit.
#[test]
fn a_touch_that_changes_no_bytes_is_not_a_change() {
    let mut f = setup();
    f.svc.open_note(&rel("nota.md")).unwrap();
    write(&f, "nota.md", "# nota\n"); // identical bytes, new mtime

    assert!(hint(&mut f, &["nota.md"], &[]).is_empty());
}

/// An unreachable root is one event, not a thousand removals. Scope §12:
/// *"Raiz/arquivo inacessível → estado indisponível; não infere exclusão."*
#[test]
fn an_unreachable_root_is_reported_as_unavailable_and_deletes_nothing() {
    let mut f = setup();
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;
    std::fs::remove_dir_all(f.work.path()).unwrap();

    let events = scan(&mut f, &[]);
    assert_eq!(events.len(), 1, "{events:?}");
    assert!(matches!(events[0], CoreEvent::WorkspaceUnavailable { .. }));
    // The registry is untouched: the notes are not gone, the folder is.
    assert!(f.svc.reload_note(id).is_err());
}

/// Ignored entries never reach reconciliation, so a `.git/` full of churn does
/// not wake the application up.
#[test]
fn changes_inside_an_ignored_directory_are_not_reported() {
    let mut f = setup();
    std::fs::create_dir(f.work.path().join(".git")).unwrap();
    std::fs::write(f.work.path().join(".git/index"), b"x").unwrap();
    std::fs::write(f.work.path().join(".git/nota.md"), b"# nao conta\n").unwrap();

    let events = scan(&mut f, &[]);
    assert!(events.is_empty(), "{events:?}");
}

/// The budget is a promise that a tick ends. Past it, work is queued rather
/// than done, and the caller is told there is more.
#[test]
fn the_hash_budget_defers_work_instead_of_blocking() {
    let mut f = setup();
    let mut ids = Vec::new();
    for i in 0..60 {
        let name = format!("n{i:03}.md");
        std::fs::write(f.work.path().join(&name), format!("# {i}\n")).unwrap();
        ids.push(f.svc.open_note(&rel(&name)).unwrap().note_id);
    }
    std::thread::sleep(Duration::from_millis(5));
    for i in 0..60 {
        std::fs::write(
            f.work.path().join(format!("n{i:03}.md")),
            format!("# mudou {i}\n"),
        )
        .unwrap();
    }

    let first = f.svc.reconcile(&BTreeSet::new(), &[]).unwrap();
    assert!(first.queued > 0, "the rest must be queued, not dropped");
    assert!(first.events.len() <= 50, "{}", first.events.len());

    // The next tick picks up what was deferred, and eventually there is nothing
    // left — a queue that never drains is a queue that lost the work.
    let mut seen = first.events.len();
    for _ in 0..5 {
        let next = f.svc.reconcile(&BTreeSet::new(), &[]).unwrap();
        seen += next.events.len();
        if next.queued == 0 {
            break;
        }
    }
    assert_eq!(seen, 60, "every change must be reported exactly once");
}

// ---------------------------------------------------------------------------
// The watcher itself
// ---------------------------------------------------------------------------

/// The 0.1b acceptance criterion — *"editar no VS Code com o app aberto atualiza
/// a aba em <1s"* — as far as it can be asserted without a window: a real
/// change to a real file reaches the reconciler well inside the second.
#[test]
fn a_real_watcher_reports_a_change_within_the_debounce() {
    let mut f = setup();
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;
    match f.svc.start_watch().unwrap() {
        None => {}
        Some(reason) => {
            // A machine with no inotify budget is a machine fact. Skipping is
            // honest; asserting anyway would pass for the wrong reason.
            eprintln!("skipped: this machine cannot watch — {reason}");
            return;
        }
    }

    let started = std::time::Instant::now();
    write(&f, "nota.md", "# outro editor\n");

    let paths = f.svc.wait_for_change(Duration::from_millis(900));
    assert!(
        paths.contains(&rel("nota.md")),
        "the watcher must report the file within a second: {paths:?}"
    );

    let events = f.svc.reconcile(&paths, &[]).unwrap().events;
    assert_eq!(
        events,
        vec![CoreEvent::FsChanged {
            path: rel("nota.md"),
            kind: ChangeKind::Modified,
            note_id: Some(id),
        }]
    );
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "took {:?}",
        started.elapsed()
    );
}

/// Our own temporary files never reach the reconciler, so an autosave does not
/// look like two changes.
#[test]
fn the_watcher_never_reports_our_own_temporary_files() {
    let mut f = setup();
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();
    if f.svc.start_watch().unwrap().is_some() {
        eprintln!("skipped: this machine cannot watch");
        return;
    }
    std::thread::sleep(Duration::from_millis(5));
    f.svc
        .save_note(opened.note_id, "# eu escrevi\n", 1, &opened.base_rev)
        .unwrap();

    let paths = f.svc.wait_for_change(Duration::from_millis(900));
    assert!(
        paths.iter().all(|p| !p.file_name().ends_with(".tmp")),
        "{paths:?}"
    );
}
