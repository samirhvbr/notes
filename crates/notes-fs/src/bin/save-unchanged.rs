//! Open every file under a directory, save it back unedited, and report.
//!
//! This is the 0.1a byte-preservation criterion in its literal form: run it
//! over the committed corpus in the working tree, then `git status --porcelain`
//! must be empty. `tools/byte-preservation.sh` does both halves.
//!
//! The hermetic version of the same assertion lives in
//! `crates/notes-fs/tests/fixtures.rs`, which works on a copy — this one exists
//! because "git sees no change" is what the criterion actually says, and git is
//! the only thing that can answer it.
//!
//!     save-unchanged <dir>

use notes_fs::{FileSystem, LocalFs};
use notes_model::{RelPath, TextProfile};
use std::path::Path;

fn main() {
    let dir = std::env::args()
        .nth(1)
        .expect("usage: save-unchanged <dir>");
    let fs = LocalFs::open(&dir).expect("open workspace");
    let root = fs.root().to_path_buf();

    let mut paths = Vec::new();
    walk(&root, &root, &mut paths);
    paths.sort();

    let (mut saved, mut read_only, mut skipped) = (0, 0, 0);
    for rel in &paths {
        let bytes = match fs.read(rel) {
            Ok(b) => b,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let (profile, text) = TextProfile::detect(&bytes);
        if profile.read_only_reason().is_some() {
            read_only += 1;
            continue;
        }
        let Some(text) = text else {
            read_only += 1;
            continue;
        };
        let encoded = profile.encode(&text);
        if encoded != bytes {
            eprintln!("MISMATCH before writing: {rel} — profile {profile:?}");
            std::process::exit(1);
        }
        fs.write_atomic(rel, &encoded, None).expect("write");
        saved += 1;
    }
    println!("saved unchanged: {saved} · read-only: {read_only} · skipped: {skipped}");
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<RelPath>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if e.file_type().unwrap().is_dir() {
            walk(root, &p, out);
        } else {
            let rel = p
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if let Ok(r) = RelPath::parse(&rel) {
                out.push(r);
            }
        }
    }
}
