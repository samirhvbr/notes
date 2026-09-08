//! The full-disk half of acceptance criterion 5, against a **real** ENOSPC.
//!
//! `IoKind::classify` is unit-tested for errno 28 and for EDQUOT, but a
//! classifier proves nothing about the path from a filesystem that is actually
//! out of room to a visible error and a recoverable buffer — the two steps in
//! between are `write_atomic` returning `Err` and `settle` writing the draft,
//! and neither is exercised by constructing an `io::Error`. That gap is why the
//! criterion read *partly met* at 0.1a.
//!
//! **Not root, and not a loopback mount.** The workspace lives on a size-capped
//! `tmpfs` inside an unprivileged user namespace, which the kernel allows any
//! user to create and which returns ENOSPC on overflow like any other
//! filesystem. `tools/enospc.sh` builds the namespace and runs this test inside
//! it; the test itself only needs the directory, named by `NOTES_TINY_DIR`.
//!
//! Ignored by default because without that directory there is nothing to
//! assert, and a test that silently passes when its precondition is missing is
//! the failure mode `docs/ACCEPTANCE-0.1a.md` exists to prevent.

#![cfg(unix)]

use notes_core::{SaveResult, WorkspaceService};
use notes_model::{IoKind, RelPath};

const TINY: &str = "NOTES_TINY_DIR";

#[test]
#[ignore = "needs a size-capped filesystem: run tools/enospc.sh"]
fn a_full_disk_is_reported_and_leaves_the_buffer_recoverable() {
    let tiny = std::env::var(TINY).unwrap_or_else(|_| {
        panic!("{TINY} is not set — run this through tools/enospc.sh, which provides it")
    });
    let tiny = std::path::PathBuf::from(tiny);

    // The app data directory stays on the ordinary filesystem. That is the
    // point of the criterion rather than an artefact of the test: the draft is
    // what makes the buffer recoverable, and a draft written beside the note
    // would fail for the same reason the note did.
    let data = tempfile::tempdir().unwrap();

    let root = tiny.join("workspace");
    std::fs::create_dir_all(&root).unwrap();
    let note = root.join("nota.md");
    let original: &[u8] = b"# nota\n";
    std::fs::write(&note, original).unwrap();

    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(&root).unwrap();
    let ws_id = svc.workspace_id().unwrap();
    let opened = svc.open_note(&RelPath::parse("nota.md").unwrap()).unwrap();

    // Comfortably larger than the filesystem, and self-describing so a partial
    // write could not be mistaken for the whole thing.
    let typed = format!(
        "# nota\n\n{}\nFIM\n",
        "conteúdo que não cabe. ".repeat(400_000)
    );
    assert!(
        typed.len() > 8 * 1024 * 1024,
        "the buffer must exceed the filesystem"
    );

    let result = svc
        .save_note(opened.note_id, &typed, 7, &opened.base_rev)
        .expect("a full disk is a result, not an Err");

    match result {
        SaveResult::WriteFailed {
            kind,
            buffer_version,
        } => {
            assert_eq!(
                kind,
                IoKind::DiskFull,
                "a full disk must not read as {kind:?}"
            );
            assert_eq!(buffer_version, 7, "the failure names the buffer it refused");
        }
        other => panic!("expected WriteFailed on a full filesystem, got {other:?}"),
    }

    // Nothing partial reached the note: the temporary file never became it.
    assert_eq!(
        std::fs::read(&note).unwrap(),
        original,
        "the note on disk must be byte-identical after a failed write"
    );

    // …and the temporary file did not survive to litter the user's folder.
    let leftovers: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".tmp"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "temporary files left behind: {leftovers:?}"
    );

    // The buffer is recoverable: the draft holds it verbatim, on the filesystem
    // that still has room.
    let dir =
        notes_core::paths::drafts_dir(&notes_core::paths::workspace_dir(svc.data_dir(), ws_id));
    let draft = notes_core::drafts::read(&dir, opened.note_id)
        .unwrap()
        .expect("a failed write must leave a draft");
    assert_eq!(
        draft.bytes,
        typed.as_bytes(),
        "the draft must hold the buffer verbatim"
    );
    assert_eq!(draft.info.reason, notes_core::DraftReason::WriteFailed);

    // And reopening the note offers it, which is the user-visible half.
    let reopened = svc.open_note(&RelPath::parse("nota.md").unwrap()).unwrap();
    assert_eq!(reopened.text, "# nota\n", "the note itself is unchanged");
    assert!(
        reopened.draft.is_some(),
        "reopening a note whose write failed must offer the draft back"
    );
}
