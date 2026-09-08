//! The three ways out of a conflict, and the rule they all share.
//!
//! Scope §12 lists four resolutions; `ARCHITECTURE.md` §17.1 settled that
//! *comparar* is a screen rather than a command. The three that touch the disk
//! are here, and every one of them is asserted against the same requirement:
//! **the version the user did not choose still exists afterwards.**

use notes_core::{ConflictChoice, SaveResult, Side, WorkspaceService};
use notes_model::{BaseRev, CoreError, Eol, NoteId, ReadOnlyReason, RelPath};

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
    std::fs::write(work.path().join("nota.md"), b"# original\n").unwrap();
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(work.path()).unwrap();
    Fixture {
        _data: data,
        work,
        svc,
    }
}

/// Open the note, type into it, have someone else write the file, and let the
/// save discover the divergence. Returns the note id and the base revision the
/// buffer was opened against — the state a real conflict starts from.
fn conflicted(f: &mut Fixture, mine: &str, theirs: &str) -> (NoteId, BaseRev) {
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();
    let id = opened.note_id;
    let base = opened.base_rev.clone();

    std::fs::write(f.work.path().join("nota.md"), theirs.as_bytes()).unwrap();
    let r = f.svc.save_note(id, mine, 1, &base).unwrap();
    assert!(
        matches!(r, SaveResult::Conflict { .. }),
        "the fixture must actually conflict, got {r:?}"
    );
    assert!(f.svc.is_suspended(id), "a conflict suspends autosave");
    (id, base)
}

fn disk(f: &Fixture) -> String {
    std::fs::read_to_string(f.work.path().join("nota.md")).unwrap()
}

fn snapshots(f: &Fixture) -> Vec<notes_core::ConflictSnapshot> {
    f.svc.list_conflicts().unwrap().snapshots
}

fn snapshot_text(s: &notes_core::ConflictSnapshot) -> String {
    std::fs::read_to_string(&s.file).unwrap()
}

// ---------------------------------------------------------------------------

#[test]
fn keep_local_writes_the_buffer_and_keeps_the_disk_version() {
    let mut f = setup();
    let (id, base) = conflicted(&mut f, "# meu\n", "# deles\n");

    let reopened = f
        .svc
        .resolve_conflict(id, "# meu\n", &base, ConflictChoice::KeepLocal)
        .unwrap();

    assert_eq!(disk(&f), "# meu\n", "the buffer must have won");
    assert_eq!(reopened.text, "# meu\n");
    assert_eq!(reopened.note_id, id, "the note keeps its identity");
    assert!(!f.svc.is_suspended(id), "resolving lifts the suspension");
    assert!(reopened.draft.is_none(), "the draft has served its purpose");

    let kept = snapshots(&f);
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(kept[0].side, Side::Disk);
    assert_eq!(
        snapshot_text(&kept[0]),
        "# deles\n",
        "the version that lost must still exist"
    );
}

#[test]
fn use_disk_throws_the_buffer_away_into_conflicts_rather_than_away() {
    let mut f = setup();
    let (id, base) = conflicted(&mut f, "# meu\n", "# deles\n");

    let reopened = f
        .svc
        .resolve_conflict(id, "# meu\n", &base, ConflictChoice::UseDisk)
        .unwrap();

    assert_eq!(disk(&f), "# deles\n", "the disk must be untouched");
    assert_eq!(reopened.text, "# deles\n");
    assert!(!f.svc.is_suspended(id));

    let kept = snapshots(&f);
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(kept[0].side, Side::Local);
    assert_eq!(
        snapshot_text(&kept[0]),
        "# meu\n",
        "a buffer is never discarded without a copy"
    );
}

#[test]
fn save_as_copy_leaves_both_versions_on_disk_where_the_user_can_see_them() {
    let mut f = setup();
    let (id, base) = conflicted(&mut f, "# meu\n", "# deles\n");

    let copy = f
        .svc
        .resolve_conflict(id, "# meu\n", &base, ConflictChoice::SaveAsCopy)
        .unwrap();

    assert_eq!(disk(&f), "# deles\n", "the note itself is left alone");
    assert_eq!(
        copy.path,
        rel("nota (local).md"),
        "scope §12 names the file"
    );
    assert_eq!(copy.text, "# meu\n");
    assert_ne!(copy.note_id, id, "a new file is a new note");
    assert_eq!(
        std::fs::read_to_string(f.work.path().join("nota (local).md")).unwrap(),
        "# meu\n"
    );
    assert!(!f.svc.is_suspended(id));
}

/// A second conflict on the same note has somewhere to go. `create_new` never
/// overwrites, so without a numbered fallback the resolution would fail with
/// `AlreadyExists` and the user would be stuck holding a buffer.
#[test]
fn a_second_copy_is_numbered_rather_than_refused() {
    let mut f = setup();
    let (id, base) = conflicted(&mut f, "# um\n", "# deles\n");
    f.svc
        .resolve_conflict(id, "# um\n", &base, ConflictChoice::SaveAsCopy)
        .unwrap();

    let (id, base) = conflicted(&mut f, "# dois\n", "# outros\n");
    let copy = f
        .svc
        .resolve_conflict(id, "# dois\n", &base, ConflictChoice::SaveAsCopy)
        .unwrap();
    assert_eq!(copy.path, rel("nota (local 2).md"));
    assert_eq!(copy.text, "# dois\n");
}

/// The removal case of scope §12: the file is gone and the buffer is not.
/// `KeepLocal` is the user asking for it back — the only circumstance in which
/// this application recreates a path it did not create.
#[test]
fn keep_local_recreates_a_note_that_was_removed_externally() {
    let mut f = setup();
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();
    let (id, base) = (opened.note_id, opened.base_rev.clone());
    std::fs::remove_file(f.work.path().join("nota.md")).unwrap();

    let reopened = f
        .svc
        .resolve_conflict(id, "# de volta\n", &base, ConflictChoice::KeepLocal)
        .unwrap();
    assert_eq!(disk(&f), "# de volta\n");
    assert_eq!(reopened.text, "# de volta\n");
}

/// …and `UseDisk` on a removed file is the user accepting the deletion. The
/// buffer is kept anyway, in `conflicts/`, and the tab is told the note is gone.
#[test]
fn use_disk_on_a_removed_note_accepts_the_deletion_and_still_keeps_the_buffer() {
    let mut f = setup();
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();
    let (id, base) = (opened.note_id, opened.base_rev.clone());
    std::fs::remove_file(f.work.path().join("nota.md")).unwrap();

    match f
        .svc
        .resolve_conflict(id, "# meu texto\n", &base, ConflictChoice::UseDisk)
    {
        Err(CoreError::NotFound { .. }) => {}
        other => panic!("expected NotFound so the tab closes, got {other:?}"),
    }
    let kept = snapshots(&f);
    assert_eq!(kept.len(), 1);
    assert_eq!(snapshot_text(&kept[0]), "# meu texto\n");
}

#[test]
fn a_conflict_snapshot_is_never_pruned_while_it_is_young() {
    let mut f = setup();
    let (id, base) = conflicted(&mut f, "# meu\n", "# deles\n");
    f.svc
        .resolve_conflict(id, "# meu\n", &base, ConflictChoice::KeepLocal)
        .unwrap();

    let root = f.work.path().to_path_buf();
    let data = f.svc.data_dir().to_path_buf();
    let mut reopened = WorkspaceService::with_data_dir(&data).unwrap();
    reopened.open_workspace(&root).unwrap();
    assert_eq!(
        reopened.list_conflicts().unwrap().snapshots.len(),
        1,
        "a fresh snapshot survives the prune that runs on open"
    );
}

/// `0` is "keep them", not "delete them all". A user typing a zero into a
/// retention field is turning the feature off.
#[test]
fn a_retention_of_zero_prunes_nothing() {
    let mut f = setup();
    let (id, base) = conflicted(&mut f, "# meu\n", "# deles\n");
    f.svc
        .resolve_conflict(id, "# meu\n", &base, ConflictChoice::KeepLocal)
        .unwrap();

    let dir = notes_core::paths::workspace_dir(f.svc.data_dir(), f.svc.workspace_id().unwrap());
    assert_eq!(notes_core::conflicts::prune(&dir, 0), 0);
    assert_eq!(snapshots(&f).len(), 1);
}

// ---------------------------------------------------------------------------
// Conversion — the one command that rewrites a file the user did not edit
// ---------------------------------------------------------------------------

#[test]
fn converting_line_endings_makes_a_mixed_note_editable_and_keeps_the_old_bytes() {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let mixed = b"uma\r\nduas\ntres\r\n";
    std::fs::write(work.path().join("misto.md"), mixed).unwrap();
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(work.path()).unwrap();

    let opened = svc.open_note(&rel("misto.md")).unwrap();
    assert_eq!(
        opened.read_only,
        Some(ReadOnlyReason::MixedEol),
        "a mixed note opens read-only, which is what conversion exists for"
    );

    let converted = svc.convert_eol(opened.note_id, Eol::Lf).unwrap();
    assert_eq!(converted.read_only, None, "and is editable afterwards");
    assert_eq!(
        std::fs::read(work.path().join("misto.md")).unwrap(),
        b"uma\nduas\ntres\n"
    );

    // The bytes it had are recoverable, like any other resolution.
    let kept = svc.list_conflicts().unwrap().snapshots;
    assert_eq!(kept.len(), 1);
    assert_eq!(std::fs::read(&kept[0].file).unwrap(), mixed);
}

#[test]
fn converting_to_mixed_is_refused_because_that_is_what_it_removes() {
    let mut f = setup();
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;
    assert!(matches!(
        f.svc.convert_eol(id, Eol::Mixed),
        Err(CoreError::InvalidPath { .. })
    ));
}

#[test]
fn converting_a_note_that_is_not_utf8_is_refused_rather_than_guessed_at() {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("bin.md"), [0xFF, 0xFE, 0x00, 0x41]).unwrap();
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(work.path()).unwrap();
    let id = svc.open_note(&rel("bin.md")).unwrap().note_id;

    assert!(matches!(
        svc.convert_eol(id, Eol::Lf),
        Err(CoreError::ReadOnly {
            reason: ReadOnlyReason::NotUtf8,
            ..
        })
    ));
    assert_eq!(
        std::fs::read(work.path().join("bin.md")).unwrap(),
        [0xFF, 0xFE, 0x00, 0x41],
        "and the bytes are untouched"
    );
}

// ---------------------------------------------------------------------------

#[test]
fn reloading_returns_what_is_on_disk_now() {
    let mut f = setup();
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;
    std::fs::write(f.work.path().join("nota.md"), b"# outro\n").unwrap();
    assert_eq!(f.svc.reload_note(id).unwrap().text, "# outro\n");
}

/// Closing a tab lifts the suspension and **leaves the draft alone**: a draft
/// outlives the tab, and is removed only by a confirmed write or by the user.
#[test]
fn closing_a_note_lifts_the_suspension_and_keeps_the_draft() {
    let mut f = setup();
    let (id, _) = conflicted(&mut f, "# meu\n", "# deles\n");
    assert_eq!(f.svc.list_drafts().unwrap().len(), 1);

    f.svc.close_note(id).unwrap();
    assert!(!f.svc.is_suspended(id));
    assert_eq!(
        f.svc.list_drafts().unwrap().len(),
        1,
        "the draft must survive the tab"
    );
}
