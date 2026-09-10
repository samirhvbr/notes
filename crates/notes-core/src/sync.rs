//! Explicit sync inventories. Normal folder listing stays lazy and never hashes.
use crate::{Result, WorkspaceService};
use notes_fs::FileSystem;
use notes_model::{CoreError, EntryKind, RelPath};
use notes_sync::File;
use std::path::Path;

/// Resolve the nearest existing ancestor before creating any state directory.
/// A rejected location must not leave new files inside a source workspace.
pub fn validate_state_location(roots: &[&Path], data: &Path) -> Result<()> {
    if !data.is_absolute()
        || data
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(CoreError::Unsupported {
            cap: "state location must be absolute without traversal".into(),
        });
    }
    let mut ancestor = data;
    let mut tail = vec![];
    while !ancestor.exists() {
        tail.push(ancestor.file_name().ok_or_else(|| CoreError::Unsupported {
            cap: "invalid state location".into(),
        })?);
        ancestor = ancestor.parent().ok_or_else(|| CoreError::Unsupported {
            cap: "invalid state location".into(),
        })?;
    }
    let mut resolved =
        std::fs::canonicalize(ancestor).map_err(|e| CoreError::io("sync_state", "state", &e))?;
    for component in tail.into_iter().rev() {
        resolved.push(component);
    }
    for root in roots {
        let root =
            std::fs::canonicalize(root).map_err(|e| CoreError::io("sync_root", "workspace", &e))?;
        if resolved.starts_with(root) {
            return Err(CoreError::Unsupported {
                cap: "sync state must be outside source folders".into(),
            });
        }
    }
    Ok(())
}

/// Read source bytes through the core and retain the existing note identities.
/// This operation is explicit and bounded; no source file is created or saved.
/// The operational directory must be separate from the user workspace.
pub fn inventory(root: &Path, data: &Path) -> Result<Vec<File>> {
    let root =
        std::fs::canonicalize(root).map_err(|e| CoreError::io("sync_root", "workspace", &e))?;
    validate_state_location(&[&root], data)?;
    let mut service = WorkspaceService::with_data_dir(data)?;
    let canonical_data =
        std::fs::canonicalize(data).map_err(|e| CoreError::io("sync_state", "state", &e))?;
    if canonical_data.starts_with(&root) {
        return Err(CoreError::Unsupported {
            cap: "sync state must be outside the workspace".into(),
        });
    }
    service.record_visits = false;
    service.open_workspace(&root)?;
    let mut pending = vec![RelPath::root()];
    let mut paths = vec![];
    while let Some(dir) = pending.pop() {
        for entry in service.open()?.fs.list(&dir)? {
            if crate::ignore::is_hidden_name(&entry.name)
                || service
                    .open()?
                    .extra_ignore
                    .iter()
                    .any(|s| s == &entry.name || s == entry.path.as_str())
            {
                continue;
            }
            match entry.kind {
                EntryKind::Dir => pending.push(entry.path),
                EntryKind::File if entry.is_note => {
                    if paths.len() >= 10_000
                        || entry.size.is_some_and(|size| size > 8 * 1024 * 1024)
                    {
                        return Err(CoreError::Unsupported {
                            cap: "sync inventory exceeds its limits".into(),
                        });
                    }
                    paths.push(entry.path);
                }
                _ => {}
            }
        }
    }
    paths.sort();
    // Drain the core's bounded correlation queue before opening renamed notes.
    // Otherwise a rename beyond the first hash budget could receive a new ID.
    let mut settled = false;
    for _ in 0..=200 {
        if service.reconcile_all(&[])?.queued == 0 {
            settled = true;
            break;
        }
    }
    if !settled {
        return Err(CoreError::LockTimeout);
    }
    paths
        .into_iter()
        .map(|path| {
            let note = service.open_note(&path)?;
            Ok(File {
                note: note.note_id,
                path,
                content: note.base_rev.hash,
            })
        })
        .collect()
}
