//! Conflict snapshots, and the four ways out of a conflict.
//!
//! Scope §12 names the resolutions — *comparar · manter o meu · usar o do disco
//! · salvar como `nome (local).md`* — and `ARCHITECTURE.md` §17.1 already
//! settled that **"compare" is not one of them**: it changes nothing on disk, it
//! is UI over data the frontend is already holding, and a command that reads two
//! strings the caller has would be a command that exists to be called by a
//! screen rather than to do anything.
//!
//! The other three are here, and each one **keeps the version it is not
//! choosing**. That is the rule the whole module exists for: a resolution is the
//! one moment a user can lose work by answering a dialog quickly, so the answer
//! they did not give is written to `conflicts/` first (`ARCHITECTURE.md` §4.3).

use std::path::{Path, PathBuf};

use notes_model::{BaseRev, CoreError, NoteId, RelPath};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What the user chose. Named exactly as `ARCHITECTURE.md` §7.1 names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ConflictChoice {
    /// Overwrite the disk with the buffer. The disk version is snapshotted
    /// first.
    KeepLocal,
    /// Throw the buffer away and take what is on disk. The buffer is
    /// snapshotted first.
    UseDisk,
    /// Write the buffer to `nome (local).md` and leave the note alone. Both
    /// versions end up on disk, where the user can see them.
    SaveAsCopy,
}

/// Which side of a conflict a snapshot holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Side {
    /// What the user had typed.
    Local,
    /// What was on disk when they were asked.
    Disk,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ConflictSnapshot {
    pub schema: u32,
    pub note_id: NoteId,
    pub path: RelPath,
    pub side: Side,
    pub base_rev: BaseRev,
    pub resolved_as: ConflictChoice,
    pub written_at: String,
    /// The file holding the bytes, as an absolute path in app data. The UI
    /// shows it so a user can find the version they did not choose without
    /// being told where the application keeps things.
    pub file: String,
    #[ts(type = "number")]
    pub size: u64,
}

/// The whole of `conflicts/` for one workspace, and what it costs.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Conflicts {
    pub snapshots: Vec<ConflictSnapshot>,
    #[ts(type = "number")]
    pub bytes: u64,
    /// True past the 200 MB-per-workspace mark of `ARCHITECTURE.md` §4.3. The
    /// application **says so and deletes nothing**: making room by throwing away
    /// the only copy of something a user wrote is the failure this directory
    /// exists to prevent.
    pub over_budget: bool,
}

/// `ARCHITECTURE.md` §4.3.
pub const BUDGET_BYTES: u64 = 200 * 1024 * 1024;

fn note_dir(dir: &Path, id: NoteId) -> PathBuf {
    crate::paths::conflicts_dir(dir).join(id.to_string())
}

/// Write one side of a conflict, and return the sidecar that describes it.
pub fn snapshot(
    dir: &Path,
    note_id: NoteId,
    path: &RelPath,
    side: Side,
    base_rev: &BaseRev,
    resolved_as: ConflictChoice,
    bytes: &[u8],
) -> Result<ConflictSnapshot, CoreError> {
    let d = note_dir(dir, note_id);
    std::fs::create_dir_all(&d).map_err(|e| CoreError::io("mkdir", d.display(), &e))?;

    // `<iso-ts>-<local|disk>.md`, with the colons of an ISO timestamp removed:
    // they are legal on ext4 and illegal on NTFS, and app data is as portable
    // as a workspace is.
    let stamp = crate::now().replace(':', "");
    let name = match side {
        Side::Local => format!("{stamp}-local.md"),
        Side::Disk => format!("{stamp}-disk.md"),
    };
    let file = d.join(&name);
    crate::state::write_atomic(&file, bytes)?;

    let info = ConflictSnapshot {
        schema: 1,
        note_id,
        path: path.clone(),
        side,
        base_rev: base_rev.clone(),
        resolved_as,
        written_at: crate::now(),
        file: file.display().to_string(),
        size: bytes.len() as u64,
    };
    let sidecar = file.with_extension("md.json");
    let json = serde_json::to_vec_pretty(&info).map_err(|e| CoreError::Internal {
        message: format!("conflict sidecar: {e}"),
    })?;
    crate::state::write_atomic(&sidecar, &json)?;
    Ok(info)
}

/// Everything under `conflicts/`, newest first.
pub fn list(dir: &Path) -> Conflicts {
    let root = crate::paths::conflicts_dir(dir);
    let mut snapshots = Vec::new();
    let mut bytes = 0u64;

    if let Ok(per_note) = std::fs::read_dir(&root) {
        for note in per_note.flatten() {
            let Ok(files) = std::fs::read_dir(note.path()) else {
                continue;
            };
            for f in files.flatten() {
                let p = f.path();
                if p.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let Ok(raw) = std::fs::read(&p) else { continue };
                let Ok(info) = serde_json::from_slice::<ConflictSnapshot>(&raw) else {
                    continue;
                };
                bytes += info.size + raw.len() as u64;
                snapshots.push(info);
            }
        }
    }
    snapshots.sort_by(|a, b| b.written_at.cmp(&a.written_at));
    Conflicts {
        snapshots,
        bytes,
        over_budget: bytes > BUDGET_BYTES,
    }
}

/// Delete snapshots older than `days`.
///
/// **Everything under `conflicts/` is by definition a *resolved* conflict** —
/// an unresolved one is a draft, in `drafts/`, and nothing prunes those
/// (`ARCHITECTURE.md` §4.2, §4.3). So retention here needs no state beyond the
/// timestamp in the name.
///
/// `days == 0` disables pruning rather than deleting everything, which is the
/// reading a user typing a zero into a settings field means.
pub fn prune(dir: &Path, days: u32) -> u64 {
    if days == 0 {
        return 0;
    }
    let cutoff =
        std::time::SystemTime::now() - std::time::Duration::from_secs(days as u64 * 86_400);
    let root = crate::paths::conflicts_dir(dir);
    let mut removed = 0u64;

    let Ok(per_note) = std::fs::read_dir(&root) else {
        return 0;
    };
    for note in per_note.flatten() {
        let Ok(files) = std::fs::read_dir(note.path()) else {
            continue;
        };
        for f in files.flatten() {
            let older = f
                .metadata()
                .and_then(|m| m.modified())
                .map(|m| m < cutoff)
                .unwrap_or(false);
            if older && std::fs::remove_file(f.path()).is_ok() {
                removed += 1;
            }
        }
        // An empty per-note directory is noise; a non-empty one is left alone.
        let _ = std::fs::remove_dir(note.path());
    }
    removed
}
