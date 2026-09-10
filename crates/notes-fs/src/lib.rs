//! The filesystem seam.
//!
//! This crate knows about bytes, paths and capabilities. It does **not** know
//! what a note is, and never touches the registry (`ARCHITECTURE.md` §2) — the
//! separation is what lets `notes-core`'s write protocol be tested against a
//! fake implementation with no disk at all.
//!
//! The trait exists at 0.1a with exactly one implementation behind it. That is
//! deliberate: "a folder the user chose" is a desktop concept, and iOS has no
//! equivalent (ADR-008). Discovering that after the core has been written
//! against `std::fs` is a rewrite of the core.

mod local;
mod probe;
pub mod watch;

pub use local::LocalFs;
pub use probe::probe_case_insensitive;
pub use watch::{Degraded, Watch};

use notes_model::{BaseRev, Caps, ContentHash, CoreError, Entry, RelPath, Stat};

pub type Result<T> = std::result::Result<T, CoreError>;

/// What a write asked for, and what it got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteOutcome {
    Written(Stat),
    /// `expect` was supplied and the file on disk no longer matches it. The
    /// caller decides; this crate never overwrites a divergence.
    Diverged(Stat),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteOutcome {
    Trashed,
    Permanent,
}

/// Everything the core is allowed to do to a filesystem.
///
/// Every path is a [`RelPath`]; resolving it against the root and refusing an
/// escape is the implementation's job, **on every call** — a root validated once
/// at open time says nothing about the path being used now (scope §7.6).
pub trait FileSystem: Send + Sync {
    fn caps(&self) -> Caps;

    /// One level. Never reads file contents — listing a 10 000-note workspace
    /// is an acceptance criterion of 0.1a and reading would blow it.
    fn list(&self, dir: &RelPath) -> Result<Vec<Entry>>;

    fn read(&self, path: &RelPath) -> Result<Vec<u8>>;

    /// Write via a temporary file and a rename (`ARCHITECTURE.md` §5.2).
    ///
    /// When `expect` is `Some`, the file is re-read and hashed immediately before the
    /// rename and a mismatch returns [`WriteOutcome::Diverged`] with nothing
    /// written — the narrow window between the core's own check and this one.
    fn write_atomic(
        &self,
        path: &RelPath,
        bytes: &[u8],
        expect: Option<&BaseRev>,
    ) -> Result<WriteOutcome>;

    /// Create a new file, failing if anything is already there. Never
    /// overwrites (scope §7.1).
    fn create_new(&self, path: &RelPath, bytes: &[u8]) -> Result<Stat>;

    fn create_dir(&self, path: &RelPath) -> Result<()>;
    fn rename(&self, from: &RelPath, to: &RelPath) -> Result<()>;
    fn delete(&self, path: &RelPath) -> Result<DeleteOutcome>;
    fn stat(&self, path: &RelPath) -> Result<Stat>;

    /// Start watching the root, recursively.
    ///
    /// **Not being able to watch is a state of the workspace, not a failure of
    /// this call**, which is why the returned [`Watch`] carries a `degraded`
    /// reason instead of this returning `Err`: a network mount, a SAF tree
    /// `[0.4]` and a kernel out of inotify watches all mean *poll instead and
    /// say why*, and none of them should stop a workspace opening.
    fn watch(&self) -> Watch {
        Watch::none(Degraded::Unsupported("this backend has no watch".into()))
    }
}

/// blake3 of a byte slice.
pub fn hash(bytes: &[u8]) -> ContentHash {
    ContentHash::from_bytes(*blake3::hash(bytes).as_bytes())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_hard_coded_empty_hash_matches_the_real_hasher() {
        // notes-model carries the digest of `b""` as a constant so that it can
        // stay free of a hash dependency. This is the assertion that keeps the
        // constant honest.
        assert_eq!(super::hash(b""), notes_model::ContentHash::of_empty());
    }
}
