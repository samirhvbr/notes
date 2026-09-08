//! Changing workspace without going through the Welcome screen.
//!
//! Milestone 0.1d's first item is the interface for this
//! (`.continue/0.1d-interface.md` §4.2), and the interface is a menu over two
//! commands that already existed. **These tests exist because nothing had ever
//! exercised the path between them.** Every test in the suite opens one
//! workspace; switching from one to another — which is what the menu does, and
//! what scope §9 governs with *"trocar de workspace resolve pendências
//! antes"* — was reachable only from the Welcome screen, which disappears
//! after the first open.
//!
//! They pin behaviour rather than a fix. Identity surviving a switch holds
//! today because `open_note` writes the registry on every observe; if the
//! two-second debounce `ARCHITECTURE.md` §4.1 describes is ever actually
//! implemented, `open_workspace` becomes a path that drops the unwritten part
//! (`DECISIONS-0.1d.md` D-01). That is the day this file earns its keep.

use notes_core::WorkspaceService;
use notes_model::{NoteId, RelPath};

fn rel(s: &str) -> RelPath {
    RelPath::parse(s).unwrap()
}

struct Two {
    data: tempfile::TempDir,
    a: tempfile::TempDir,
    b: tempfile::TempDir,
}

fn two() -> Two {
    let t = Two {
        data: tempfile::tempdir().unwrap(),
        a: tempfile::tempdir().unwrap(),
        b: tempfile::tempdir().unwrap(),
    };
    std::fs::write(t.a.path().join("nota-a.md"), b"# a\n\ncorpo\n").unwrap();
    std::fs::write(t.b.path().join("nota-b.md"), b"# b\n\ncorpo\n").unwrap();
    t
}

fn service(t: &Two) -> WorkspaceService {
    WorkspaceService::with_data_dir(t.data.path()).unwrap()
}

/// The identity a workspace minted survives being switched away from.
///
/// A `NoteId` is minted when a note is first **opened** (§9's `observe`). Open
/// one, switch immediately, come back: the id has to be the same, or the tab
/// that was pointing at it is pointing at nothing.
#[test]
fn switching_workspace_persists_the_identities_the_old_one_minted() {
    let t = two();
    let mut svc = service(&t);

    svc.open_workspace(t.a.path()).unwrap();
    let first = svc.open_note(&rel("nota-a.md")).unwrap().note_id;

    // Straight to the other workspace, with no close and no pause.
    svc.open_workspace(t.b.path()).unwrap();
    svc.open_note(&rel("nota-b.md")).unwrap();

    svc.open_workspace(t.a.path()).unwrap();
    let again = svc.open_note(&rel("nota-a.md")).unwrap().note_id;

    assert_eq!(
        first, again,
        "the note kept its identity across the switch — a NoteId that changes is \
         a tab that cannot find its note (ADR-015)"
    );
}

/// And across a restart of the process, which is the same fact seen from the
/// other side: a service built fresh over the same data directory finds the id
/// rather than minting a new one.
#[test]
fn the_identity_is_on_disk_after_the_switch_not_only_in_memory() {
    let t = two();
    let first = {
        let mut svc = service(&t);
        svc.open_workspace(t.a.path()).unwrap();
        let id = svc.open_note(&rel("nota-a.md")).unwrap().note_id;
        svc.open_workspace(t.b.path()).unwrap();
        // Dropped without a close: a crash, a kill, a window closed by the WM.
        id
    };

    let mut svc = service(&t);
    svc.open_workspace(t.a.path()).unwrap();
    let again = svc.open_note(&rel("nota-a.md")).unwrap().note_id;
    assert_eq!(
        first, again,
        "the identity was on disk before the switch, not only in memory"
    );
}

/// The guard the menu relies on, asserted where the menu will call it.
///
/// `.continue/0.1d-interface.md` §4.2 puts the guard on the switch path:
/// *"Trocar de workspace pelo menu é `close` + `open`, na ordem, com a mesma
/// guarda."* The refusal names the notes, so the interface can offer to flush
/// them rather than only saying no.
#[test]
fn closing_with_a_dirty_buffer_is_refused_and_names_the_notes() {
    let t = two();
    let mut svc = service(&t);
    svc.open_workspace(t.a.path()).unwrap();
    let id = svc.open_note(&rel("nota-a.md")).unwrap().note_id;

    let err = svc.close_workspace(&[id]).unwrap_err();
    match err {
        notes_model::CoreError::DirtyBuffers { note_ids, count } => {
            assert_eq!(count, 1);
            assert_eq!(note_ids, vec![id], "named, so the UI can offer to save");
        }
        other => panic!("expected DirtyBuffers, got {other:?}"),
    }

    // And the workspace is still the one that was open — a refused close must
    // not half-close.
    assert!(
        svc.list_dir(&RelPath::root()).is_ok(),
        "the workspace stayed open after the refusal"
    );
}

/// With nothing dirty, close leaves no workspace open — which is what sends the
/// interface back to Welcome.
#[test]
fn closing_clean_leaves_no_workspace_open() {
    let t = two();
    let mut svc = service(&t);
    svc.open_workspace(t.a.path()).unwrap();

    svc.close_workspace(&[]).unwrap();
    assert!(matches!(
        svc.list_dir(&RelPath::root()),
        Err(notes_model::CoreError::NoWorkspace)
    ));
}

/// The menu's recents come from the same list the Welcome screen shows, and a
/// workspace reaches it by being opened — so the workspace being left is in it
/// the moment the next one is picked.
#[test]
fn every_workspace_opened_is_offered_again_in_recents() {
    let t = two();
    let mut svc = service(&t);
    svc.open_workspace(t.a.path()).unwrap();
    svc.open_workspace(t.b.path()).unwrap();

    let recent = svc.recent_workspaces().unwrap();
    let roots: Vec<_> = recent.iter().map(|w| w.root.as_str()).collect();
    for want in [t.a.path(), t.b.path()] {
        let want = want.canonicalize().unwrap().display().to_string();
        assert!(
            roots.iter().any(|r| *r == want),
            "{want} is offered again: {roots:?}"
        );
    }
}

/// A `NoteId` the caller invented has never been observed, so it cannot be a
/// dirty buffer of this workspace — but the refusal is still the honest answer.
/// The core does not hold buffers and cannot check the claim; what it must not
/// do is quietly close over a caller that believes work is pending.
#[test]
fn a_claim_of_dirtiness_is_believed_rather_than_second_guessed() {
    let t = two();
    let mut svc = service(&t);
    svc.open_workspace(t.a.path()).unwrap();

    assert!(svc.close_workspace(&[NoteId::new()]).is_err());
}
