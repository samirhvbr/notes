//! Rename, move, duplicate and delete — the 0.1b criteria of scope §17 that
//! can be asserted without a window.
//!
//! Three of the five acceptance criteria live here:
//!
//! - *rename via app não reseta aba/cursor/id* — the identity half;
//! - *criar/duplicar nunca sobrescreve destino existente; move com colisão pede
//!   resolução*;
//! - *delete reporta `Trashed` ou `Permanent`; nunca apaga sem dizer qual*.

use notes_core::{DeleteKind, WorkspaceService};
use notes_model::{CoreError, EntryKind, RelPath};

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
    std::fs::create_dir_all(work.path().join("pasta/dentro")).unwrap();
    std::fs::write(work.path().join("pasta/a.md"), b"# a\n").unwrap();
    std::fs::write(work.path().join("pasta/dentro/b.md"), b"# b\n").unwrap();
    std::fs::create_dir(work.path().join("destino")).unwrap();

    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(work.path()).unwrap();
    Fixture {
        _data: data,
        work,
        svc,
    }
}

// ---------------------------------------------------------------------------
// "rename via app não reseta aba/cursor/id"
// ---------------------------------------------------------------------------

#[test]
fn renaming_a_note_keeps_its_identity() {
    let mut f = setup();
    let before = f.svc.open_note(&rel("nota.md")).unwrap().note_id;

    let entry = f.svc.rename_entry(&rel("nota.md"), "renomeada.md").unwrap();
    assert_eq!(entry.path, rel("renomeada.md"));
    assert!(entry.is_note);

    let after = f.svc.open_note(&rel("renomeada.md")).unwrap().note_id;
    assert_eq!(
        after, before,
        "a rename the app performs never enters identity correlation"
    );
    assert!(!f.work.path().join("nota.md").exists());
}

/// Renaming a folder carries every note beneath it. Giving them new ids would
/// lose their history for a reason invisible to the person who did it.
#[test]
fn renaming_a_folder_carries_the_notes_inside_it() {
    let mut f = setup();
    let a = f.svc.open_note(&rel("pasta/a.md")).unwrap().note_id;
    let b = f.svc.open_note(&rel("pasta/dentro/b.md")).unwrap().note_id;

    f.svc.rename_entry(&rel("pasta"), "renomeada").unwrap();

    assert_eq!(f.svc.open_note(&rel("renomeada/a.md")).unwrap().note_id, a);
    assert_eq!(
        f.svc
            .open_note(&rel("renomeada/dentro/b.md"))
            .unwrap()
            .note_id,
        b
    );
}

/// A sibling whose name merely *starts with* the renamed folder's name is not
/// inside it. A naive prefix test moves it and silently corrupts the registry.
#[test]
fn a_sibling_with_a_similar_name_is_not_dragged_along() {
    let mut f = setup();
    std::fs::create_dir(f.work.path().join("pasta2")).unwrap();
    std::fs::write(f.work.path().join("pasta2/c.md"), b"# c\n").unwrap();
    let c = f.svc.open_note(&rel("pasta2/c.md")).unwrap().note_id;

    f.svc.rename_entry(&rel("pasta"), "outra-pasta").unwrap();

    assert_eq!(f.svc.open_note(&rel("pasta2/c.md")).unwrap().note_id, c);
    assert!(f.work.path().join("pasta2/c.md").exists());
}

#[test]
fn a_rename_onto_an_existing_name_is_refused_before_anything_moves() {
    let mut f = setup();
    match f.svc.rename_entry(&rel("nota.md"), "outra.md") {
        Err(CoreError::AlreadyExists { .. }) => {}
        other => panic!("expected AlreadyExists, got {other:?}"),
    }
    assert!(f.work.path().join("nota.md").exists());
    assert_eq!(
        std::fs::read_to_string(f.work.path().join("outra.md")).unwrap(),
        "# outra\n",
        "the file that was in the way must be untouched"
    );
}

#[test]
fn a_rename_to_a_name_no_filesystem_can_hold_is_refused() {
    let mut f = setup();
    // Each is legal on ext4 and refused because it is not portable: a
    // separator, a name Windows has reserved since DOS, a trailing space, a
    // trailing dot, and a character NTFS forbids (scope §7.6).
    for bad in [
        "com/barra.md",
        "CON.md",
        "acaba com espaco ",
        "acaba com ponto.",
        "dois:pontos.md",
        "",
    ] {
        assert!(
            f.svc.rename_entry(&rel("nota.md"), bad).is_err(),
            "{bad} must be refused"
        );
    }
    assert!(f.work.path().join("nota.md").exists());
}

// ---------------------------------------------------------------------------
// Move
// ---------------------------------------------------------------------------

#[test]
fn moving_a_note_keeps_its_identity_and_its_name() {
    let mut f = setup();
    let before = f.svc.open_note(&rel("nota.md")).unwrap().note_id;

    let entry = f.svc.move_entry(&rel("nota.md"), &rel("destino")).unwrap();
    assert_eq!(entry.path, rel("destino/nota.md"));
    assert_eq!(
        f.svc.open_note(&rel("destino/nota.md")).unwrap().note_id,
        before
    );
}

/// "move com colisão pede resolução": the core refuses and names what is in the
/// way, which is what lets the interface ask rather than guess.
#[test]
fn a_move_onto_an_existing_name_is_refused_and_says_so() {
    let mut f = setup();
    std::fs::write(f.work.path().join("destino/nota.md"), b"# ja existe\n").unwrap();

    match f.svc.move_entry(&rel("nota.md"), &rel("destino")) {
        Err(CoreError::AlreadyExists { path }) => assert_eq!(path, "destino/nota.md"),
        other => panic!("expected AlreadyExists, got {other:?}"),
    }
    assert_eq!(
        std::fs::read_to_string(f.work.path().join("destino/nota.md")).unwrap(),
        "# ja existe\n"
    );
}

#[test]
fn a_folder_cannot_be_moved_inside_itself() {
    let mut f = setup();
    match f.svc.move_entry(&rel("pasta"), &rel("pasta/dentro")) {
        Err(CoreError::InvalidPath { .. }) => {}
        other => panic!("expected InvalidPath, got {other:?}"),
    }
    assert!(f.work.path().join("pasta/dentro/b.md").exists());
}

// ---------------------------------------------------------------------------
// "criar/duplicar nunca sobrescreve destino existente"
// ---------------------------------------------------------------------------

#[test]
fn duplicating_a_note_never_overwrites_and_gets_a_new_identity() {
    let mut f = setup();
    let original = f.svc.open_note(&rel("nota.md")).unwrap().note_id;

    let copy = f.svc.duplicate_entry(&rel("nota.md")).unwrap();
    assert_eq!(copy.path, rel("nota (copy).md"));
    assert_eq!(
        std::fs::read_to_string(f.work.path().join("nota (copy).md")).unwrap(),
        "# nota\n"
    );
    assert_ne!(
        f.svc.open_note(&rel("nota (copy).md")).unwrap().note_id,
        original,
        "a new file is a new note; it has none of the original's history"
    );

    // …and again, without clobbering the first copy.
    let second = f.svc.duplicate_entry(&rel("nota.md")).unwrap();
    assert_eq!(second.path, rel("nota (copy 2).md"));
    assert_eq!(
        std::fs::read_to_string(f.work.path().join("nota (copy).md")).unwrap(),
        "# nota\n",
        "the first copy must be untouched"
    );
}

#[test]
fn duplicating_a_folder_copies_the_tree() {
    let mut f = setup();
    let copy = f.svc.duplicate_entry(&rel("pasta")).unwrap();
    assert_eq!(copy.path, rel("pasta (copy)"));
    assert_eq!(copy.kind, EntryKind::Dir);
    assert_eq!(
        std::fs::read_to_string(f.work.path().join("pasta (copy)/dentro/b.md")).unwrap(),
        "# b\n"
    );
    // The original is still there, in one piece.
    assert_eq!(
        std::fs::read_to_string(f.work.path().join("pasta/dentro/b.md")).unwrap(),
        "# b\n"
    );
}

/// The name is ASCII and locale-independent on purpose: a filename that
/// depended on the interface language would give the same folder different
/// names on two machines.
#[test]
fn the_copy_suffix_does_not_depend_on_the_interface_language() {
    let mut f = setup();
    let copy = f.svc.duplicate_entry(&rel("nota.md")).unwrap();
    assert!(copy.path.as_str().is_ascii(), "{}", copy.path);
}

// ---------------------------------------------------------------------------
// "delete reporta Trashed ou Permanent; nunca apaga sem dizer qual"
// ---------------------------------------------------------------------------

#[test]
fn deleting_says_which_of_the_two_happened_and_forgets_the_note() {
    let mut f = setup();
    let id = f.svc.open_note(&rel("nota.md")).unwrap().note_id;

    let deleted = f.svc.delete_entry(&rel("nota.md")).unwrap();
    assert_eq!(deleted.path, rel("nota.md"));
    assert!(
        matches!(deleted.outcome, DeleteKind::Trashed | DeleteKind::Permanent),
        "the outcome is always one of the two, and always reported"
    );
    assert_eq!(
        deleted.note_ids,
        vec![id],
        "the tab holding it has to be told"
    );
    assert!(!f.work.path().join("nota.md").exists());

    // The identity is gone with the file; reopening the path is not possible.
    assert!(f.svc.open_note(&rel("nota.md")).is_err());
}

#[test]
fn deleting_a_folder_forgets_every_note_beneath_it() {
    let mut f = setup();
    let a = f.svc.open_note(&rel("pasta/a.md")).unwrap().note_id;
    let b = f.svc.open_note(&rel("pasta/dentro/b.md")).unwrap().note_id;
    let other = f.svc.open_note(&rel("nota.md")).unwrap().note_id;

    let deleted = f.svc.delete_entry(&rel("pasta")).unwrap();
    let mut ids = deleted.note_ids.clone();
    ids.sort();
    let mut want = vec![a, b];
    want.sort();
    assert_eq!(ids, want);
    assert!(
        !deleted.note_ids.contains(&other),
        "a note outside the folder is not touched"
    );
    assert!(f.work.path().join("nota.md").exists());
}

/// A note deleted while it held unsaved edits is precisely the case where the
/// draft is the only copy of them. Deleting the file must not take it.
#[test]
fn deleting_a_note_leaves_its_draft_alone() {
    let mut f = setup();
    let opened = f.svc.open_note(&rel("nota.md")).unwrap();
    f.svc
        .write_draft(
            opened.note_id,
            "coisas que eu digitei",
            3,
            &opened.base_rev,
            notes_core::DraftReason::Stale,
        )
        .unwrap();

    f.svc.delete_entry(&rel("nota.md")).unwrap();
    let drafts = f.svc.list_drafts().unwrap();
    assert_eq!(drafts.len(), 1, "the draft is the only copy of that text");
    assert_eq!(drafts[0].note_id, opened.note_id);
}

/// `RelPath` cannot express an escape, so the shape a real attempt takes is a
/// symlink — a well-formed relative path that resolves elsewhere. Every entry
/// operation has to refuse one, not just `note_open`.
///
/// Unix-only because that is where a symlink can be created without a
/// privilege; the Windows leg of CI runs the rest of this file.
#[test]
#[cfg(unix)]
fn no_entry_operation_follows_a_symlink_out_of_the_root() {
    let mut f = setup();
    let outside = f.work.path().parent().unwrap().join("fora.md");
    std::fs::write(&outside, b"# fora\n").unwrap();
    std::os::unix::fs::symlink(&outside, f.work.path().join("atalho.md")).unwrap();

    for r in [
        f.svc.rename_entry(&rel("atalho.md"), "novo.md").err(),
        f.svc.move_entry(&rel("atalho.md"), &rel("destino")).err(),
        f.svc.duplicate_entry(&rel("atalho.md")).err(),
        f.svc.delete_entry(&rel("atalho.md")).err(),
    ] {
        assert!(
            matches!(r, Some(CoreError::SymlinkNotFollowed { .. })),
            "a symlink must be refused, not followed: {r:?}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(&outside).unwrap(),
        "# fora\n",
        "the file outside the root must be byte-identical afterwards"
    );
}

/// The string half of the same jail, on every platform: a destination that
/// climbs out cannot even be spelled.
#[test]
fn a_destination_outside_the_root_cannot_be_expressed() {
    let mut f = setup();
    assert!(RelPath::parse("../fora.md").is_err());
    assert!(RelPath::parse("/etc/passwd").is_err());
    // …and the one shape that *is* expressible — a name with a separator in it —
    // is refused by the portability check before anything moves.
    assert!(f.svc.rename_entry(&rel("nota.md"), "../fora.md").is_err());
    assert!(f.work.path().join("nota.md").exists());
}
