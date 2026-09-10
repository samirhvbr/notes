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
pub use path::{portable_name, CompareKey, NameRule, PathError, RelPath};
pub use text::{Encoding, Eol, TextProfile};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Serialise a nanosecond timestamp as a **string**.
///
/// `mtime_ns` is around 1.7e18 today, and `Number.MAX_SAFE_INTEGER` is 9.0e15.
/// Sent as a JSON number it loses precision crossing the IPC, and it does not
/// merely display wrong — `BaseRev` travels back to the core on every save, so
/// a rounded `mtime_ns` would make the cheap check disagree with the disk and
/// send every save down the hashing path. A string is exact and costs nothing.
pub mod ns_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &i128, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<i128, D::Error> {
        // Accept a number too: state written before this rule, and any
        // hand-edited file, still load.
        match serde_json::Value::deserialize(d)? {
            serde_json::Value::String(s) => s.parse().map_err(serde::de::Error::custom),
            serde_json::Value::Number(n) => n
                .as_i64()
                .map(i128::from)
                .ok_or_else(|| serde::de::Error::custom("mtime out of range")),
            _ => Err(serde::de::Error::custom(
                "mtime must be a string or a number",
            )),
        }
    }
}

/// Filesystem identity, when the backend offers one. A **strong signal, never a
/// proof**: it is reused after deletion on most filesystems, and SAF document
/// ids change when a document moves (`ARCHITECTURE.md` §11).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum NativeId {
    Unix {
        dev: u64,
        ino: u64,
    },
    Windows {
        volume: u64,
        index: u64,
    },
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
    #[ts(type = "number")]
    pub size: u64,
    /// Nanoseconds since the Unix epoch. `i128` because a filesystem is free to
    /// report a timestamp before 1970 or far past 2262, and neither should be a
    /// panic in a note-taking app. Crosses the wire as a string — see
    /// [`ns_string`].
    #[serde(with = "ns_string")]
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
    #[ts(type = "number")]
    pub size: u64,
    #[serde(with = "ns_string")]
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
    ///
    /// `native_id` holds on both: `dev`/`ino` on Unix, and on Windows the
    /// volume serial and file index read through `GetFileInformationByHandle`
    /// (`DECISIONS-0.1a.md` D-24, closed at 0.11.1). `preserve_mode` stays
    /// Unix-only — NTFS ACLs are not a mode, and `ARCHITECTURE.md` §11 does not
    /// promise them.
    ///
    /// `trash` is **desktop-only, and that is a platform fact rather than a
    /// choice** \[0.4\]: iOS and Android have no user-visible bin for an app's
    /// own container, so there is nothing for `trash::delete` to move a file
    /// into and the crate does not build for either target. A local filesystem
    /// on a phone is still a local filesystem in every other respect — atomic
    /// replace, rename and `create_new` all hold — which is why this is one
    /// field and not a second constant. `DeleteOutcome::Permanent` is then the
    /// only answer mobile can give, and §7.7 is satisfied by saying so rather
    /// than by pretending otherwise.
    pub const LOCAL: Caps = Caps {
        atomic_replace: true,
        rename: true,
        trash: cfg!(not(any(target_os = "ios", target_os = "android"))),
        watch: true,
        native_id: cfg!(any(unix, windows)),
        preserve_mode: cfg!(unix),
        create_new: true,
        same_volume_move: true,
    };
}

/// The seven states scope §9 requires the status bar to distinguish.
///
/// Named in the model rather than assembled in the UI for one reason: **`Saved`
/// may only appear after the backend confirms**, which is a guarantee of the
/// core and not a convention for whoever writes the component. The frontend
/// computes the value from the last `SaveResult`, the open note's read-only
/// reason and the workspace's availability; it never invents a seventh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DocStatus {
    Saved,
    Pending,
    Writing,
    Conflict,
    ReadOnly,
    Error,
    Unavailable,
}

/// One entry of a directory listing. `is_note` is the core's judgement, not the
/// filesystem's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Entry {
    pub path: RelPath,
    pub name: String,
    pub kind: EntryKind,
    #[ts(type = "number | null")]
    pub size: Option<u64>,
    pub is_note: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cheap_match_needs_both_halves() {
        let h = ContentHash::from_bytes([7u8; 32]);
        let base = BaseRev {
            size: 10,
            mtime_ns: 5,
            hash: h.clone(),
        };
        let same = Stat {
            size: 10,
            mtime_ns: 5,
            native_id: None,
            kind: EntryKind::File,
        };
        let size_moved = Stat {
            size: 11,
            ..same.clone()
        };
        let mtime_moved = Stat {
            mtime_ns: 6,
            ..same.clone()
        };
        assert!(base.cheap_match(&same));
        assert!(!base.cheap_match(&size_moved));
        assert!(!base.cheap_match(&mtime_moved));
    }

    /// The unknown-backend profile must promise nothing it cannot keep. Written
    /// as a data comparison rather than three `assert!`s on constants, which
    /// clippy correctly points out are evaluated at compile time.
    /// A nanosecond timestamp must survive the wire exactly.
    ///
    /// The failure this guards is quiet: sent as a JSON number, `mtime_ns` is
    /// rounded by JavaScript, comes back in `BaseRev` on the next save, and the
    /// cheap check then disagrees with the disk on every write.
    #[test]
    fn a_nanosecond_timestamp_survives_json_exactly() {
        let precise: i128 = 1_757_268_123_456_789_321;
        assert!(
            precise > 9_007_199_254_740_991,
            "the value must exceed MAX_SAFE_INTEGER"
        );
        let base = BaseRev {
            size: 1,
            mtime_ns: precise,
            hash: ContentHash::of_empty(),
        };
        let json = serde_json::to_string(&base).unwrap();
        assert!(
            json.contains("\"1757268123456789321\""),
            "must be a string: {json}"
        );
        let back: BaseRev = serde_json::from_str(&json).unwrap();
        assert_eq!(back.mtime_ns, precise);
    }

    #[test]
    fn a_timestamp_written_as_a_number_still_loads() {
        let json = r#"{"size":1,"mtime_ns":1757268123,"hash":"b3:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"}"#;
        let back: BaseRev = serde_json::from_str(json).unwrap();
        assert_eq!(back.mtime_ns, 1_757_268_123);
    }

    #[test]
    fn conservative_caps_promise_nothing_dangerous() {
        let dangerous_to_assume = [
            ("atomic_replace", Caps::CONSERVATIVE.atomic_replace),
            ("trash", Caps::CONSERVATIVE.trash),
            ("native_id", Caps::CONSERVATIVE.native_id),
            ("watch", Caps::CONSERVATIVE.watch),
            ("preserve_mode", Caps::CONSERVATIVE.preserve_mode),
        ];
        let claimed: Vec<_> = dangerous_to_assume
            .iter()
            .filter(|(_, v)| *v)
            .map(|(n, _)| *n)
            .collect();
        assert!(claimed.is_empty(), "conservative caps claim: {claimed:?}");
    }
}
