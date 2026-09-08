//! `Stat::native_id` — the filesystem's own answer to "is this the same file?".
//!
//! Identity correlation after an external rename (`ARCHITECTURE.md` §9) asks
//! exactly that, and a content hash cannot answer it: two notes with the same
//! text hash the same, and one file renamed hashes the same as before. The
//! native id distinguishes both cases, and its absence is the reason correlation
//! degrades to minting a new `NoteId`.
//!
//! **These tests are the Windows job's, as much as this machine's.** `native_id`
//! returned `None` on Windows from 0.1a because the standard library's
//! accessors are unstable (`docs/DECISIONS-0.1a.md` D-24); the implementation
//! that closed it — `GetFileInformationByHandle` through `windows-sys` — cannot
//! be exercised here at all, so every assertion below is written to hold on
//! both platforms and to be *checked* on the one it was written for.

use notes_fs::{FileSystem, LocalFs};
use notes_model::{NativeId, RelPath};

fn ws() -> (tempfile::TempDir, LocalFs) {
    let d = tempfile::tempdir().unwrap();
    let fs = LocalFs::open(d.path()).unwrap();
    (d, fs)
}

fn p(s: &str) -> RelPath {
    RelPath::parse(s).unwrap()
}

/// The capability and the value must agree. A `Caps` that promises an id the
/// backend does not produce is worse than one that promises nothing: the core
/// reads `caps.native_id` to decide whether to *try* correlating.
#[test]
fn the_capability_says_what_the_backend_actually_returns() {
    let (d, fs) = ws();
    std::fs::write(d.path().join("a.md"), b"# a\n").unwrap();
    let stat = fs.stat(&p("a.md")).unwrap();

    assert_eq!(
        fs.caps().native_id,
        stat.native_id.is_some(),
        "caps.native_id promised {} and stat gave {:?}",
        fs.caps().native_id,
        stat.native_id
    );
}

/// On both platforms this project ships to. The shape differs — `dev`/`ino`
/// against a volume serial and a file index — and both are read here so that a
/// Windows regression is a failing test rather than a silent `None`.
#[test]
fn a_local_file_has_an_id_of_the_shape_its_platform_uses() {
    let (d, fs) = ws();
    std::fs::write(d.path().join("a.md"), b"# a\n").unwrap();
    let id = fs.stat(&p("a.md")).unwrap().native_id;

    match id {
        #[cfg(unix)]
        Some(NativeId::Unix { ino, .. }) => assert_ne!(ino, 0, "an inode of zero is not an inode"),
        #[cfg(windows)]
        Some(NativeId::Windows { volume, index }) => {
            assert_ne!(volume, 0, "NTFS reports a volume serial");
            assert_ne!(index, 0, "and a file index");
        }
        other => panic!("unexpected native id on this platform: {other:?}"),
    }
}

/// The property the whole thing exists for: a rename is the same file.
#[test]
fn a_renamed_file_keeps_its_identity() {
    let (d, fs) = ws();
    std::fs::write(d.path().join("antes.md"), b"# nota\n").unwrap();
    let before = fs.stat(&p("antes.md")).unwrap().native_id;

    // Renamed **outside** the application, which is the case correlation is for.
    std::fs::rename(d.path().join("antes.md"), d.path().join("depois.md")).unwrap();
    let after = fs.stat(&p("depois.md")).unwrap().native_id;

    assert!(before.is_some(), "nothing to correlate with: {before:?}");
    assert_eq!(before, after, "a rename does not make a different file");
}

/// And the other half: identical bytes are not identical files. This is the
/// case a content hash gets wrong, and the reason the id is worth an opened
/// handle on Windows.
#[test]
fn two_files_with_the_same_bytes_have_different_identities() {
    let (d, fs) = ws();
    std::fs::write(d.path().join("uma.md"), b"# igual\n").unwrap();
    std::fs::write(d.path().join("outra.md"), b"# igual\n").unwrap();

    let a = fs.stat(&p("uma.md")).unwrap().native_id;
    let b = fs.stat(&p("outra.md")).unwrap().native_id;

    assert!(a.is_some() && b.is_some());
    assert_ne!(a, b, "same content, different file");
}

/// A directory has one too. The tree's own root is stat'd as a directory when a
/// workspace is opened, and on Windows a handle to a directory only opens with
/// `FILE_FLAG_BACKUP_SEMANTICS` — which is the flag this asserts is set.
#[test]
fn a_directory_has_an_identity_too() {
    let (d, fs) = ws();
    std::fs::create_dir(d.path().join("pasta")).unwrap();
    let id = fs.stat(&p("pasta")).unwrap().native_id;
    assert!(id.is_some(), "a directory reports an id: {id:?}");
}

/// A rewrite in place is the same file; the atomic replace is not.
///
/// This is not a defect, it is what `stat_at` reports after the write and what
/// `ARCHITECTURE.md` §5 already implies: the temp file is *renamed over* the
/// target, so the note's identity is the new file's. Correlation never sees it
/// — the write arms a self-write expectation — but the value is asserted here so
/// that it is a recorded property rather than a surprise.
#[test]
fn an_atomic_replace_produces_a_new_identity_and_the_write_knows_it() {
    use notes_fs::WriteOutcome;
    let (d, fs) = ws();
    std::fs::write(d.path().join("a.md"), b"# um\n").unwrap();
    let before = fs.stat(&p("a.md")).unwrap();

    let outcome = fs.write_atomic(&p("a.md"), b"# dois\n", None).unwrap();
    let WriteOutcome::Written(after) = outcome else {
        panic!("expected a write, got {outcome:?}");
    };

    assert!(before.native_id.is_some() && after.native_id.is_some());
    assert_ne!(
        before.native_id, after.native_id,
        "the replace put a different file at the path, and the returned Stat says so"
    );
}

/// A file that is not there has no id and is not a panic.
#[test]
fn a_missing_file_is_an_error_not_an_identity() {
    let (_d, fs) = ws();
    assert!(fs.stat(&p("nao-existe.md")).is_err());
}
