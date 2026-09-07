use std::path::{Path, PathBuf};

use notes_model::{BaseRev, CoreError, NoteId, RelPath};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Why a buffer ended up in app data instead of in its note.
///
/// Four causes, not the scope's two: a superset cannot weaken the guarantee, and
/// `stale`/`exit` are the ones that save work nobody asked to lose
/// (`ARCHITECTURE.md` §17.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DraftReason {
    Conflict,
    WriteFailed,
    Stale,
    Exit,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DraftInfo {
    pub schema: u32,
    pub note_id: NoteId,
    pub path: RelPath,
    #[ts(type = "number")]
    pub buffer_version: u64,
    pub base_rev: BaseRev,
    pub reason: DraftReason,
    pub written_at: String,
}

/// A draft on disk: one file, one atomic write, bytes preserved exactly.
///
/// Header line of JSON, `\n`, then the buffer verbatim. Two files (`meta.json`
/// plus `buffer`) would need the pair to land atomically, and a half-written
/// pair is precisely the state a draft exists to prevent.
pub struct Draft {
    pub info: DraftInfo,
    pub bytes: Vec<u8>,
}

pub fn path_for(dir: &Path, id: NoteId) -> PathBuf {
    dir.join(format!("{id}.draft"))
}

pub fn write(dir: &Path, draft: &Draft) -> Result<(), CoreError> {
    std::fs::create_dir_all(dir).map_err(|e| CoreError::io("mkdir", dir.display(), &e))?;
    let mut header = serde_json::to_vec(&draft.info).map_err(|e| CoreError::Internal {
        message: format!("draft header: {e}"),
    })?;
    header.push(b'\n');
    header.extend_from_slice(&draft.bytes);
    crate::state::write_atomic(&path_for(dir, draft.info.note_id), &header)
}

pub fn read(dir: &Path, id: NoteId) -> Result<Option<Draft>, CoreError> {
    let p = path_for(dir, id);
    let raw = match std::fs::read(&p) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(CoreError::io("read_draft", p.display(), &e)),
    };
    let Some(nl) = raw.iter().position(|b| *b == b'\n') else {
        return Ok(None);
    };
    let Ok(info) = serde_json::from_slice::<DraftInfo>(&raw[..nl]) else {
        return Ok(None);
    };
    Ok(Some(Draft {
        info,
        bytes: raw[nl + 1..].to_vec(),
    }))
}

/// Remove a draft — **only** once a write of at least its `buffer_version` has
/// been confirmed. It is the only copy of something the user typed, so it never
/// expires and no cleanup touches it.
pub fn discard(dir: &Path, id: NoteId) -> Result<(), CoreError> {
    match std::fs::remove_file(path_for(dir, id)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(CoreError::io(
            "remove_draft",
            path_for(dir, id).display(),
            &e,
        )),
    }
}

pub fn list(dir: &Path) -> Vec<DraftInfo> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    rd.flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let id: NoteId = name.strip_suffix(".draft")?.parse().ok()?;
            read(dir, id).ok().flatten().map(|d| d.info)
        })
        .collect()
}
