//! Types shared by every crate in the workspace.
//!
//! **No I/O, and no dependency that does any.** Everything here is a value:
//! `notes-fs` produces these, `notes-core` decides with them, `src-tauri` and
//! `notes-mcp` serialise them. The rule is what makes the write protocol
//! testable against a fake filesystem (`ARCHITECTURE.md` §14).

mod error;
mod ids;
mod path;
mod text;

pub use error::{CoreError, IoKind, ReadOnlyReason, UnavailableReason};
pub use ids::{ContentHash, NoteId, WorkspaceId};
pub use path::{CompareKey, PathError, RelPath};
pub use text::{Encoding, Eol, TextProfile};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Filesystem identity, when the backend offers one. A **strong signal, never a
/// proof**: it is reused after deletion on most filesystems, and SAF document
/// ids change when a document moves (`ARCHITECTURE.md` §11).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum NativeId {
    Unix { dev: u64, ino: u64 },
    Windows { volume: u64, index: u64 },
    /// SAF document id, iOS bookmark id. `[0.4]`
    Provider(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum EntryKind {
    File,
    Dir,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Stat {
    pub size: u64,
    /// Nanoseconds since the Unix epoch. `i128` because a filesystem is free to
    /// report a timestamp before 1970 or far past 2262, and neither should be a
    /// panic in a note-taking app.
    #[ts(type = "string")]
    pub mtime_ns: i128,
    pub native_id: Option<NativeId>,
    pub kind: EntryKind,
}

/// What an open buffer was read against.
///
/// `size` and `mtime_ns` are the cheap check; **`hash` is the decision**. Scope
/// §12: size and mtime alone never authorise overwriting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BaseRev {
    pub size: u64,
    #[ts(type = "string")]
    pub mtime_ns: i128,
    pub hash: ContentHash,
}

impl BaseRev {
    /// True when size and mtime match — the cheap half of the comparison.
    /// A `false` here means *read and hash*, never *overwrite*.
    pub fn cheap_match(&self, s: &Stat) -> bool {
        self.size == s.size && self.mtime_ns == s.mtime_ns
    }
}

/// What a backend guarantees. Declared per root; the core adapts and the UI
/// states the limitation (`ARCHITECTURE.md` §11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Caps {
    pub atomic_replace: bool,
    pub rename: bool,
    pub trash: bool,
    pub watch: bool,
    pub native_id: bool,
    pub preserve_mode: bool,
    pub create_new: bool,
    pub same_volume_move: bool,
}

impl Caps {
    /// What an unknown backend gets. Every guarantee off: costs performance and
    /// honesty, never correctness.
    pub const CONSERVATIVE: Caps = Caps {
        atomic_replace: false,
        rename: true,
        trash: false,
        watch: false,
        native_id: false,
        preserve_mode: false,
        create_new: true,
        same_volume_move: true,
    };

    /// A local POSIX or NTFS filesystem.
    pub const LOCAL: Caps = Caps {
        atomic_replace: true,
        rename: true,
        trash: true,
        watch: true,
        native_id: true,
        preserve_mode: cfg!(unix),
        create_new: true,
        same_volume_move: true,
    };
}

/// One entry of a directory listing. `is_note` is the core's judgement, not the
/// filesystem's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Entry {
    pub path: RelPath,
    pub name: String,
    pub kind: EntryKind,
    pub size: Option<u64>,
    pub is_note: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cheap_match_needs_both_halves() {
        let h = ContentHash::from_bytes([7u8; 32]);
        let base = BaseRev { size: 10, mtime_ns: 5, hash: h.clone() };
        let same = Stat { size: 10, mtime_ns: 5, native_id: None, kind: EntryKind::File };
        let size_moved = Stat { size: 11, ..same.clone() };
        let mtime_moved = Stat { mtime_ns: 6, ..same.clone() };
        assert!(base.cheap_match(&same));
        assert!(!base.cheap_match(&size_moved));
        assert!(!base.cheap_match(&mtime_moved));
    }

    /// The unknown-backend profile must promise nothing it cannot keep. Written
    /// as a data comparison rather than three `assert!`s on constants, which
    /// clippy correctly points out are evaluated at compile time.
    #[test]
    fn conservative_caps_promise_nothing_dangerous() {
        let dangerous_to_assume = [
            ("atomic_replace", Caps::CONSERVATIVE.atomic_replace),
            ("trash", Caps::CONSERVATIVE.trash),
            ("native_id", Caps::CONSERVATIVE.native_id),
            ("watch", Caps::CONSERVATIVE.watch),
            ("preserve_mode", Caps::CONSERVATIVE.preserve_mode),
        ];
        let claimed: Vec<_> = dangerous_to_assume.iter().filter(|(_, v)| *v).map(|(n, _)| *n).collect();
        assert!(claimed.is_empty(), "conservative caps claim: {claimed:?}");
    }
}
