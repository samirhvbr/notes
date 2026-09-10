use crate::{BaseRev, NoteId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The kind of an I/O failure, as a **typed code**.
///
/// Scope §17 requires "disco cheio / permissão negada → erro visível", and the
/// frontend switches on the code and never reads a message (`ARCHITECTURE.md`
/// §7.3). A stringly-typed `kind` would have put the only distinguishing
/// information in the one field that is not the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum IoKind {
    DiskFull,
    PermissionDenied,
    ReadOnlyFilesystem,
    NotFound,
    IsADirectory,
    Busy,
    NameTooLong,
    Interrupted,
    Other,
}

impl IoKind {
    /// Classify a `std::io::Error`.
    ///
    /// `ErrorKind` alone does not carry "disk full" on stable Rust for every
    /// platform, so the errno is consulted first: ENOSPC (28) and EDQUOT
    /// (122 on Linux, 69 on macOS/BSD) are both "there is no room", and telling
    /// a user their disk is full when they hit a quota is closer to true than
    /// "unknown I/O error".
    pub fn classify(e: &std::io::Error) -> Self {
        if let Some(code) = e.raw_os_error() {
            match code {
                28 => return IoKind::DiskFull,       // ENOSPC
                122 | 69 => return IoKind::DiskFull, // EDQUOT
                112 | 39 => return IoKind::DiskFull, // Windows ERROR_DISK_FULL / ERROR_HANDLE_DISK_FULL
                _ => {}
            }
        }
        match e.kind() {
            std::io::ErrorKind::PermissionDenied => IoKind::PermissionDenied,
            std::io::ErrorKind::NotFound => IoKind::NotFound,
            std::io::ErrorKind::IsADirectory => IoKind::IsADirectory,
            std::io::ErrorKind::ResourceBusy | std::io::ErrorKind::WouldBlock => IoKind::Busy,
            std::io::ErrorKind::InvalidFilename => IoKind::NameTooLong,
            std::io::ErrorKind::Interrupted => IoKind::Interrupted,
            std::io::ErrorKind::ReadOnlyFilesystem => IoKind::ReadOnlyFilesystem,
            _ => IoKind::Other,
        }
    }

    /// Whether retrying the same operation could plausibly succeed. A full disk
    /// is not retried in a loop; a busy file is.
    pub fn is_transient(self) -> bool {
        matches!(self, IoKind::Busy | IoKind::Interrupted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ReadOnlyReason {
    NotUtf8,
    MixedEol,
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum UnavailableReason {
    RootMissing,
    PermissionRevoked,
    NotADirectory,
}

/// Every command returns `Result<T, CoreError>`.
///
/// `code` is the contract; the frontend maps it to an i18n key and never
/// inspects a message (`ARCHITECTURE.md` §7.3). `Internal` is a bug rather than
/// a state, and a test that expects one fails CI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, thiserror::Error)]
#[serde(tag = "code", rename_all = "snake_case")]
#[ts(export)]
pub enum CoreError {
    #[error("invalid path {path}: {reason}")]
    InvalidPath { path: String, reason: String },
    #[error("path {path} resolves outside the workspace root")]
    OutsideRoot { path: String },
    #[error("path {path} is a symlink and symlinks are not followed")]
    SymlinkNotFollowed { path: String },
    #[error("{path} not found")]
    NotFound { path: String },
    #[error("{path} already exists")]
    AlreadyExists { path: String },
    #[error("{count} buffers still have unsaved changes")]
    DirtyBuffers { note_ids: Vec<NoteId>, count: usize },
    #[error("{note_id} changed on disk since it was opened")]
    Conflict { note_id: NoteId, disk_rev: BaseRev },
    #[error("{note_id} is read-only")]
    ReadOnly {
        note_id: NoteId,
        reason: ReadOnlyReason,
    },
    #[error("workspace {root} is unavailable")]
    Unavailable {
        root: String,
        reason: UnavailableReason,
    },
    #[error("no workspace is open")]
    NoWorkspace,
    #[error("timed out waiting for the workspace write lock")]
    LockTimeout,
    #[error("this storage does not support {cap}")]
    Unsupported { cap: String },
    #[error("{op} failed on {path}")]
    Io {
        op: String,
        path: String,
        kind: IoKind,
    },
    #[error("{store} schema {found} is newer than supported schema {supported}")]
    SchemaAhead {
        store: String,
        found: u32,
        supported: u32,
    },
    #[error("internal error: {message}")]
    Internal { message: String },
}

impl CoreError {
    pub fn io(op: &str, path: impl std::fmt::Display, e: &std::io::Error) -> Self {
        CoreError::Io {
            op: op.to_string(),
            path: path.to_string(),
            kind: IoKind::classify(e),
        }
    }
    /// The stable code a frontend switches on, matching the serde tag.
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::InvalidPath { .. } => "invalid_path",
            CoreError::OutsideRoot { .. } => "outside_root",
            CoreError::SymlinkNotFollowed { .. } => "symlink_not_followed",
            CoreError::NotFound { .. } => "not_found",
            CoreError::AlreadyExists { .. } => "already_exists",
            CoreError::DirtyBuffers { .. } => "dirty_buffers",
            CoreError::Conflict { .. } => "conflict",
            CoreError::ReadOnly { .. } => "read_only",
            CoreError::Unavailable { .. } => "unavailable",
            CoreError::NoWorkspace => "no_workspace",
            CoreError::LockTimeout => "lock_timeout",
            CoreError::Unsupported { .. } => "unsupported",
            CoreError::Io { .. } => "io",
            CoreError::SchemaAhead { .. } => "schema_ahead",
            CoreError::Internal { .. } => "internal",
        }
    }
}

impl From<crate::PathError> for CoreError {
    fn from(e: crate::PathError) -> Self {
        CoreError::InvalidPath {
            path: String::new(),
            reason: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_full_is_distinguishable_from_permission_denied() {
        let full = std::io::Error::from_raw_os_error(28);
        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert_eq!(IoKind::classify(&full), IoKind::DiskFull);
        assert_eq!(IoKind::classify(&denied), IoKind::PermissionDenied);
        assert_ne!(IoKind::classify(&full), IoKind::classify(&denied));
    }

    #[test]
    fn quota_reads_as_disk_full() {
        assert_eq!(
            IoKind::classify(&std::io::Error::from_raw_os_error(122)),
            IoKind::DiskFull
        );
    }

    #[test]
    fn only_transient_kinds_are_worth_retrying() {
        assert!(IoKind::Busy.is_transient());
        assert!(!IoKind::DiskFull.is_transient());
        assert!(!IoKind::PermissionDenied.is_transient());
    }

    #[test]
    fn the_serde_tag_matches_the_code_accessor() {
        let e = CoreError::LockTimeout;
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
        assert_eq!(v["code"], e.code());
    }
}
