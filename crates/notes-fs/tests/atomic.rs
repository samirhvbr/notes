//! The atomic write.
//!
//! `ARCHITECTURE.md` §19 makes one property mandatory:
//! `read(write_atomic(x)) == x` for arbitrary bytes.

use notes_fs::{FileSystem, LocalFs, WriteOutcome};
use notes_model::{BaseRev, RelPath};

fn ws() -> (tempfile::TempDir, LocalFs) {
    let d = tempfile::tempdir().unwrap();
    let fs = LocalFs::open(d.path()).unwrap();
    (d, fs)
}

/// Deterministic byte shapes, standing in for a property-test generator.
///
/// A real generator is `proptest`, and pulling it in is a dependency decision
/// rather than a design one (`ARCHITECTURE.md` §2 pins the list). What is
/// covered here is what actually breaks a writer: empty, NUL, invalid UTF-8,
/// every line ending, a BOM, and sizes either side of a page and a buffer.
fn payloads() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"\n".to_vec(),
        b"\r\n".to_vec(),
        b"\r".to_vec(),
        b"\0\0\0".to_vec(),
        b"\xff\xfe invalid utf-8".to_vec(),
        b"\xef\xbb\xbfbom".to_vec(),
        "acentuação e emoji \u{1f331}".as_bytes().to_vec(),
        b"mixed\nline\r\nendings\rhere".to_vec(),
    ];
    for size in [1usize, 255, 4095, 4096, 4097, 65535, 65536, 1_000_003] {
        let mut b = Vec::with_capacity(size);
        let mut x: u8 = 7;
        for _ in 0..size {
            x = x.wrapping_mul(31).wrapping_add(17);
            b.push(x);
        }
        v.push(b);
    }
    v
}

#[test]
fn read_of_write_atomic_is_the_identity() {
    let (_d, fs) = ws();
    let p = RelPath::parse("n.md").unwrap();
    for payload in payloads() {
        fs.write_atomic(&p, &payload, None).unwrap();
        assert_eq!(fs.read(&p).unwrap(), payload, "lost bytes at len {}", payload.len());
    }
}

#[test]
fn a_write_leaves_no_temporary_behind() {
    let (d, fs) = ws();
    let p = RelPath::parse("n.md").unwrap();
    for payload in payloads() {
        fs.write_atomic(&p, &payload, None).unwrap();
    }
    let leftovers: Vec<_> = std::fs::read_dir(d.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.contains(".tmp-"))
        .collect();
    assert!(leftovers.is_empty(), "temp files left behind: {leftovers:?}");
}

#[test]
fn expect_matching_lets_the_write_through() {
    let (_d, fs) = ws();
    let p = RelPath::parse("n.md").unwrap();
    fs.write_atomic(&p, b"one", None).unwrap();
    let s = fs.stat(&p).unwrap();
    let base = BaseRev { size: s.size, mtime_ns: s.mtime_ns, hash: notes_fs::hash(b"one") };
    assert!(matches!(fs.write_atomic(&p, b"two", Some(&base)), Ok(WriteOutcome::Written(_))));
    assert_eq!(fs.read(&p).unwrap(), b"two");
}

#[test]
fn expect_mismatching_writes_nothing_and_reports_divergence() {
    let (_d, fs) = ws();
    let p = RelPath::parse("n.md").unwrap();
    fs.write_atomic(&p, b"one", None).unwrap();
    let s = fs.stat(&p).unwrap();
    let base = BaseRev { size: s.size, mtime_ns: s.mtime_ns, hash: notes_fs::hash(b"one") };

    // Somebody else writes. mtime resolution can be coarse, so change the size
    // too — the cheap check is size+mtime and the test is about divergence, not
    // about clock granularity.
    std::thread::sleep(std::time::Duration::from_millis(10));
    std::fs::write(fs.root().join("n.md"), b"external change").unwrap();

    let out = fs.write_atomic(&p, b"mine", Some(&base)).unwrap();
    assert!(matches!(out, WriteOutcome::Diverged(_)), "got {out:?}");
    assert_eq!(
        fs.read(&p).unwrap(),
        b"external change",
        "a divergence must never be overwritten"
    );
}

#[test]
fn a_touch_only_change_is_not_a_divergence() {
    let (_d, fs) = ws();
    let p = RelPath::parse("n.md").unwrap();
    fs.write_atomic(&p, b"same", None).unwrap();
    let s = fs.stat(&p).unwrap();
    let base = BaseRev { size: s.size, mtime_ns: s.mtime_ns, hash: notes_fs::hash(b"same") };

    // Rewritten with identical content: mtime moves, the hash does not.
    std::thread::sleep(std::time::Duration::from_millis(10));
    std::fs::write(fs.root().join("n.md"), b"same").unwrap();

    let out = fs.write_atomic(&p, b"mine", Some(&base)).unwrap();
    assert!(matches!(out, WriteOutcome::Written(_)), "hash decides, not mtime: {out:?}");
}

#[test]
fn create_new_never_overwrites() {
    let (_d, fs) = ws();
    let p = RelPath::parse("n.md").unwrap();
    fs.create_new(&p, b"first").unwrap();
    let err = fs.create_new(&p, b"second").unwrap_err();
    assert!(matches!(err, notes_model::CoreError::AlreadyExists { .. }));
    assert_eq!(fs.read(&p).unwrap(), b"first");
}

#[test]
fn rename_refuses_to_clobber_an_existing_name() {
    let (_d, fs) = ws();
    let a = RelPath::parse("a.md").unwrap();
    let b = RelPath::parse("b.md").unwrap();
    fs.create_new(&a, b"A").unwrap();
    fs.create_new(&b, b"B").unwrap();
    assert!(matches!(fs.rename(&a, &b), Err(notes_model::CoreError::AlreadyExists { .. })));
    assert_eq!(fs.read(&b).unwrap(), b"B");
}

#[test]
fn writing_into_a_missing_directory_is_an_io_error_not_a_panic() {
    let (_d, fs) = ws();
    let p = RelPath::parse("nope/n.md").unwrap();
    let err = fs.write_atomic(&p, b"x", None).unwrap_err();
    assert!(matches!(err, notes_model::CoreError::Io { .. }), "got {err:?}");
}
