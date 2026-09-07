//! Writes a note in a tight loop until it is killed. Driven by
//! `tools/crash-save-loop.sh`.
//!
//! Every payload is **self-describing** — `LEN=<n>` then `n` bytes then `END` —
//! so the checker can tell a complete note from a truncated one without knowing
//! which iteration the kill landed on. That is the whole design: after a
//! `SIGKILL` the file must be some *complete* payload, old or new, and never a
//! prefix of one.
//!
//!     crash-writer <workspace-dir> [note-name]

use notes_fs::{FileSystem, LocalFs};
use notes_model::RelPath;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: crash-writer <dir> [note]");
    let name = args.next().unwrap_or_else(|| "crash.md".to_string());

    let fs = LocalFs::open(&dir).expect("open workspace");
    let path = RelPath::parse(&name).expect("note name");

    let mut n: usize = 1;
    loop {
        // Vary the length across page and buffer boundaries, where a partial
        // write is most likely to be observable.
        let len = [1usize, 100, 4095, 4096, 4097, 65_536, 200_000][n % 7];
        let body = "x".repeat(len);
        let payload = format!("LEN={len}\n{body}\nEND\n");
        if fs.write_atomic(&path, payload.as_bytes(), None).is_err() {
            std::process::exit(2);
        }
        n = n.wrapping_add(1);
    }
}
