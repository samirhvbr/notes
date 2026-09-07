//! The 0.1a listing criterion, measured.
//!
//! Ignored by default: it needs `fixtures/large`, which is generated rather
//! than committed. Run it with
//!
//!     tools/gen-large.sh && cargo test -p notes-core --test performance -- --ignored --nocapture
//!
//! The threshold is deliberately generous against the criterion's one second:
//! a timing assertion on shared CI hardware is a flake generator, and the
//! property being defended — that listing never reads a file's contents — is
//! asserted structurally in `notes-fs`'s `listing_is_one_level_and_reads_no_content`.

use notes_core::WorkspaceService;
use notes_model::RelPath;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn large() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/large")
}

#[test]
#[ignore = "needs fixtures/large — run tools/gen-large.sh first"]
fn a_ten_thousand_note_workspace_opens_and_lists_in_under_a_second() {
    let root = large();
    assert!(root.is_dir(), "run tools/gen-large.sh first");

    let notes = walk_count(&root);
    let data = tempfile::tempdir().unwrap();
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();

    let t0 = Instant::now();
    svc.open_workspace(&root).unwrap();
    let opened = t0.elapsed();

    let t1 = Instant::now();
    let top = svc.list_dir(&RelPath::root()).unwrap();
    let listed_root = t1.elapsed();

    // Every top-level directory, which is what the tree does as the user
    // expands it — and the honest measure of "listing the tree".
    let t2 = Instant::now();
    let mut total = top.len();
    for e in &top {
        if e.kind == notes_model::EntryKind::Dir {
            for sub in svc.list_dir(&e.path).unwrap() {
                total += 1;
                if sub.kind == notes_model::EntryKind::Dir {
                    total += svc.list_dir(&sub.path).unwrap().len();
                }
            }
        }
    }
    let listed_all = t2.elapsed();

    println!("notes on disk:        {notes}");
    println!("entries listed:       {total}");
    println!("open_workspace:       {opened:?}");
    println!("list root:            {listed_root:?}");
    println!("list whole tree:      {listed_all:?}");
    println!("open + whole tree:    {:?}", opened + listed_all);

    assert!(
        (opened + listed_all).as_secs_f64() < 1.0,
        "open + full tree walk took {:?}, over the one-second criterion",
        opened + listed_all
    );

    // The registry must still be empty: listing assigns no identity (D-09).
    // If it did, this would have hashed 197 MiB.
    assert!(svc.list_drafts().unwrap().is_empty());
}

fn walk_count(dir: &Path) -> usize {
    let mut n = 0;
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        if e.file_type().unwrap().is_dir() {
            n += walk_count(&e.path());
        } else {
            n += 1;
        }
    }
    n
}
