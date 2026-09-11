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
    inventory_using(root, data, false)
}
/// Correlate identities while holding exclusive ownership of a draft-free folder.
pub fn closed_inventory(root: &Path, data: &Path) -> Result<Vec<File>> {
    inventory_using(root, data, true)
}
fn inventory_using(root: &Path, data: &Path, exclusive: bool) -> Result<Vec<File>> {
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
    if exclusive {
        service.open_sync_workspace(&root)?;
        match std::fs::read_dir(crate::paths::drafts_dir(&service.open()?.dir)) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    return Err(CoreError::Unsupported {
                        cap: "workspace has pending drafts".into(),
                    });
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(CoreError::io("sync_drafts", "state", &e)),
        }
    } else {
        service.open_workspace(&root)?;
    }
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
    correlate_inventory_cycles(&mut service, &paths)?;
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

// Only a complete permutation with unique native identities AND unchanged bytes
// can override path identity. Atomic saves and ambiguous hard links do not qualify.
fn correlate_inventory_cycles(service: &mut WorkspaceService, paths: &[RelPath]) -> Result<()> {
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    let records = service.open()?.registry.notes.clone();
    let mut identities = HashMap::new();
    for (id, record) in &records {
        if let Some(native) = &record.native_id {
            identities
                .entry(native.clone())
                .or_insert_with(Vec::new)
                .push(*id);
        }
    }
    let mut destinations = BTreeMap::new();
    let mut seen = HashMap::new();
    for path in paths {
        let stat = service.open()?.fs.stat(path)?;
        if let Some(native) = stat.native_id {
            *seen.entry(native.clone()).or_insert(0usize) += 1;
            if let Some(ids) = identities.get(&native) {
                if ids.len() == 1 && records[&ids[0]].path != *path {
                    let bytes = service.open()?.fs.read(path)?;
                    if notes_fs::hash(&bytes) == records[&ids[0]].hash {
                        destinations.insert(ids[0], path.clone());
                    }
                }
            }
        }
    }
    destinations.retain(|id, _| {
        records[id]
            .native_id
            .as_ref()
            .is_some_and(|n| seen.get(n) == Some(&1))
    });
    let mut accepted = BTreeSet::new();
    for start in destinations.keys() {
        let mut cycle = BTreeSet::new();
        let mut current = *start;
        while cycle.insert(current) {
            let Some(target) = destinations.get(&current) else {
                break;
            };
            let Some((next, _)) = records.iter().find(|(_, r)| r.path == *target) else {
                break;
            };
            current = *next;
            if current == *start {
                accepted.extend(cycle);
                break;
            }
        }
    }
    for id in accepted {
        service
            .open_mut()?
            .registry
            .notes
            .get_mut(&id)
            .unwrap()
            .path = destinations[&id].clone();
    }
    Ok(())
}

/// Original bytes captured against the inventory hash. A concurrent edit aborts
/// capture instead of enqueuing a revision with a hash from different bytes.
pub fn capture(root: &Path, data: &Path) -> Result<Vec<(File, Vec<u8>)>> {
    let files = inventory(root, data)?;
    let mut service = WorkspaceService::with_data_dir(data)?;
    service.record_visits = false;
    service.open_workspace(root)?;
    let mut total = 0usize;
    let mut result = Vec::new();
    for file in files {
        if service.open()?.fs.stat(&file.path)?.size > 8 * 1024 * 1024 {
            return Err(CoreError::Unsupported {
                cap: "sync capture exceeds its limits".into(),
            });
        }
        let bytes = service.open()?.fs.read(&file.path)?;
        total = total.saturating_add(bytes.len());
        if total > 32 * 1024 * 1024 || notes_fs::hash(&bytes) != file.content {
            return Err(CoreError::Unsupported {
                cap: "sync capture changed or exceeds its limits".into(),
            });
        }
        result.push((file, bytes));
    }
    Ok(result)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Applied {
    pub note_id: notes_model::NoteId,
    pub base_rev: notes_model::BaseRev,
}
/// Offline application: all cooperating clients must share the app data path.
/// `prepare` durably records intent after precondition checks, before any write.
/// Retry is authorized only by that saved intent, never by matching bytes alone.
pub fn apply_received(
    root: &Path,
    data: &Path,
    path: &RelPath,
    bytes: &[u8],
    expected: Option<&Applied>,
    retry: bool,
    prepare: impl FnOnce() -> Result<()>,
) -> Result<Applied> {
    validate_state_location(&[root], data)?;
    let mut service = WorkspaceService::with_data_dir(data)?;
    service.record_visits = false;
    service.open_sync_workspace(root)?;
    apply_in_workspace(&mut service, path, bytes, expected, retry, &[], prepare)
}

/// The host must freeze editing and provide every live buffer, including
/// inactive panes, for the duration of this call and the subsequent reload.
/// Core does not own frontend buffers and cannot infer omitted buffer state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct BufferSnapshot {
    pub note_id: notes_model::NoteId,
    pub base_rev: notes_model::BaseRev,
    #[ts(type = "number")]
    pub buffer_version: u64,
    #[ts(type = "number")]
    pub saved_version: u64,
}

/// Apply one received revision in an opt-in exclusive session. Dirty or stale
/// buffers are refused, never flushed or discarded. After success the host must
/// reload affected clean buffers before unfreezing editing. The returned receipt
/// still needs durable client persistence; retry follows the original intent.
#[allow(clippy::too_many_arguments)]
pub fn apply_in_workspace(
    service: &mut WorkspaceService,
    path: &RelPath,
    bytes: &[u8],
    expected: Option<&Applied>,
    retry: bool,
    buffers: &[BufferSnapshot],
    prepare: impl FnOnce() -> Result<()>,
) -> Result<Applied> {
    use notes_model::{BaseRev, IoKind};
    if !path.is_note()
        || bytes.len() > 8 * 1024 * 1024
        || path.as_str().split('/').any(crate::ignore::is_hidden_name)
    {
        return Err(CoreError::Unsupported {
            cap: "invalid sync application".into(),
        });
    }
    for name in path.as_str().split('/') {
        notes_model::portable_name(name).map_err(|e| CoreError::InvalidPath {
            path: path.to_string(),
            reason: e.to_string(),
        })?;
    }
    let open = service.open()?;
    if !open.sync_exclusive || open.read_only || !open.fs.caps().atomic_replace {
        return Err(CoreError::Unsupported {
            cap: "workspace cannot apply atomic sync writes".into(),
        });
    }
    if !open.suspended.is_empty() || buffers.iter().any(|b| b.buffer_version != b.saved_version) {
        return Err(CoreError::Unsupported {
            cap: "sync application has dirty or suspended buffers".into(),
        });
    }
    let dir = open.dir.clone();
    let mut guard = crate::lock::acquire(&crate::paths::lock_file(&dir))?;
    guard.with(|| {
        let mut seen = std::collections::BTreeSet::new();
        for buffer in buffers {
            if !seen.insert(buffer.note_id) {
                return Err(CoreError::Unsupported {
                    cap: "duplicate sync buffer snapshot".into(),
                });
            }
            let open = service.open()?;
            let record =
                open.registry
                    .record(buffer.note_id)
                    .ok_or_else(|| CoreError::NotFound {
                        path: buffer.note_id.to_string(),
                    })?;
            let stat = open.fs.stat(&record.path)?;
            if stat.size > 8 * 1024 * 1024
                || stat.size != buffer.base_rev.size
                || stat.mtime_ns != buffer.base_rev.mtime_ns
                || notes_fs::hash(&open.fs.read(&record.path)?) != buffer.base_rev.hash
            {
                return Err(CoreError::Unsupported {
                    cap: "sync application has a stale buffer".into(),
                });
            }
        }
        let drafts = crate::paths::drafts_dir(&dir);
        match std::fs::read_dir(&drafts) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    return Err(CoreError::Unsupported {
                        cap: "workspace has pending drafts".into(),
                    });
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(CoreError::io("sync_drafts", "state", &e)),
        }
        let current = match service.open()?.fs.stat(path) {
            Ok(stat) => {
                if stat.size > 8 * 1024 * 1024 {
                    return Err(CoreError::Unsupported {
                        cap: "sync target exceeds limit".into(),
                    });
                }
                let raw = service.open()?.fs.read(path)?;
                Some(BaseRev {
                    size: stat.size,
                    mtime_ns: stat.mtime_ns,
                    hash: notes_fs::hash(&raw),
                })
            }
            Err(CoreError::Io {
                kind: IoKind::NotFound,
                ..
            })
            | Err(CoreError::NotFound { .. }) => None,
            Err(e) => return Err(e),
        };
        let hash = notes_fs::hash(bytes);
        let already = retry && current.as_ref().is_some_and(|r| r.hash == hash);
        if !already {
            match (expected, &current) {
                (None, None) => {
                    let mut cursor = RelPath::root();
                    for name in path.as_str().split('/') {
                        let next = cursor.join(name)?;
                        match service.open()?.fs.stat(&next) {
                            Ok(stat) if stat.kind == EntryKind::Dir => {}
                            Ok(_) => {
                                return Err(CoreError::AlreadyExists {
                                    path: next.to_string(),
                                })
                            }
                            Err(CoreError::Io {
                                kind: IoKind::NotFound,
                                ..
                            })
                            | Err(CoreError::NotFound { .. }) => {
                                service.check_name(name, &next)?;
                                break;
                            }
                            Err(e) => return Err(e),
                        }
                        cursor = next;
                    }
                }
                (Some(before), Some(now))
                    if before.base_rev == *now
                        && service
                            .open()?
                            .registry
                            .record(before.note_id)
                            .is_some_and(|r| r.path == *path) => {}
                _ => {
                    return Err(CoreError::Unsupported {
                        cap: "sync target changed".into(),
                    })
                }
            }
        }
        prepare()?;
        if !already {
            if let Some(before) = expected {
                match service
                    .open()?
                    .fs
                    .write_atomic(path, bytes, Some(&before.base_rev))?
                {
                    notes_fs::WriteOutcome::Written(_) => {}
                    notes_fs::WriteOutcome::Diverged(_) => {
                        return Err(CoreError::Unsupported {
                            cap: "sync target changed during write".into(),
                        })
                    }
                }
            } else {
                // Create parent folders through the same jail; no source note is
                // overwritten even if a third-party editor wins the create race.
                let mut parent = RelPath::root();
                let names: Vec<_> = path.as_str().split('/').collect();
                for name in &names[..names.len() - 1] {
                    let next = parent.join(name)?;
                    match service.open()?.fs.stat(&next) {
                        Ok(_) => {}
                        Err(CoreError::Io {
                            kind: IoKind::NotFound,
                            ..
                        })
                        | Err(CoreError::NotFound { .. }) => {
                            service.check_name(name, &next)?;
                            service.open()?.fs.create_dir(&next)?;
                        }
                        Err(e) => return Err(e),
                    }
                    parent = next;
                }
                service.open()?.fs.create_new(path, bytes)?;
            }
        }
        let stat = service.open()?.fs.stat(path)?;
        let actual = service.open()?.fs.read(path)?;
        if notes_fs::hash(&actual) != hash {
            return Err(CoreError::Unsupported {
                cap: "sync target changed after write".into(),
            });
        }
        let id = service
            .open_mut()?
            .registry
            .observe(path, &stat, hash.clone());
        crate::state::store(
            &crate::paths::registry_file(&dir),
            &service.open()?.registry,
        )?;
        service.invalidate_paths();
        Ok(Applied {
            note_id: id,
            base_rev: BaseRev {
                size: stat.size,
                mtime_ns: stat.mtime_ns,
                hash,
            },
        })
    })?
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct SyncApplyResult {
    pub applied: Option<u32>,
    pub error: Option<CoreError>,
    pub refreshed: Vec<crate::OpenedNote>,
    pub reload_failed: bool,
}

/// Capture one already-applied note in a closed workspace. The receiver maps
/// its remote note to this local identity; paths alone never authorize capture.
pub fn capture_conflict(
    root: &Path,
    data: &Path,
    path: &RelPath,
    expected: &Applied,
) -> Result<(Applied, Vec<u8>)> {
    capture_saved(root, data, path, Some(expected.note_id))
}
/// Read a saved note with exclusive ownership; new notes have no prior receipt.
pub fn capture_saved(
    root: &Path,
    data: &Path,
    path: &RelPath,
    expected: Option<notes_model::NoteId>,
) -> Result<(Applied, Vec<u8>)> {
    validate_state_location(&[root], data)?;
    let mut service = WorkspaceService::with_data_dir(data)?;
    service.record_visits = false;
    service.open_sync_workspace(root)?;
    let dir = service.open()?.dir.clone();
    match std::fs::read_dir(crate::paths::drafts_dir(&dir)) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(CoreError::Unsupported {
                    cap: "workspace has pending drafts".into(),
                });
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(CoreError::io("sync_drafts", "state", &e)),
    }
    if service.open()?.fs.stat(path)?.size > 8 * 1024 * 1024 {
        return Err(CoreError::Unsupported {
            cap: "sync capture exceeds its limit".into(),
        });
    }
    let note = service.open_note(path)?;
    if expected.is_some_and(|id| note.note_id != id) || note.draft.is_some() {
        return Err(CoreError::Unsupported {
            cap: "sync capture identity changed".into(),
        });
    }
    let stat = service.open()?.fs.stat(path)?;
    if stat.size > 8 * 1024 * 1024 {
        return Err(CoreError::Unsupported {
            cap: "sync capture exceeds its limit".into(),
        });
    }
    let bytes = service.open()?.fs.read(path)?;
    let after = service.open()?.fs.stat(path)?;
    if stat.size != after.size
        || stat.mtime_ns != after.mtime_ns
        || stat.size != note.base_rev.size
        || stat.mtime_ns != note.base_rev.mtime_ns
        || notes_fs::hash(&bytes) != note.base_rev.hash
    {
        return Err(CoreError::Unsupported {
            cap: "sync capture changed during read".into(),
        });
    }
    Ok((
        Applied {
            note_id: note.note_id,
            base_rev: note.base_rev,
        },
        bytes,
    ))
}

/// Closed-workspace move/delete resolution. The caller retains captured bytes
/// and persists intent before any filesystem effect. A move creates its chosen
/// destination without replacement before removing the guarded source.
#[allow(clippy::too_many_arguments)]
pub fn apply_resolution_effect(
    root: &Path,
    data: &Path,
    from: &RelPath,
    to: &RelPath,
    bytes: Option<&[u8]>,
    expected: &Applied,
    retry: bool,
    prepare: impl FnOnce() -> Result<()>,
) -> Result<Option<Applied>> {
    if let (true, Some(raw)) = (from == to, bytes) {
        return apply_received(root, data, to, raw, Some(expected), retry, prepare).map(Some);
    }
    validate_state_location(&[root], data)?;
    let mut service = WorkspaceService::with_data_dir(data)?;
    service.record_visits = false;
    service.open_sync_workspace(root)?;
    let blocked = || CoreError::Unsupported {
        cap: "sync move/delete precondition failed".into(),
    };
    for path in [from, to] {
        if !path.is_note() || path.as_str().split('/').any(crate::ignore::is_hidden_name) {
            return Err(blocked());
        }
        for name in path.as_str().split('/') {
            notes_model::portable_name(name).map_err(|_| blocked())?;
        }
    }
    if bytes.is_some_and(|b| b.len() > 8 * 1024 * 1024) {
        return Err(blocked());
    }
    let dir = service.open()?.dir.clone();
    let mut lock = crate::lock::acquire(&crate::paths::lock_file(&dir))?;
    lock.with(|| {
        let open = service.open()?;
        if open.read_only || !open.sync_exclusive || !open.fs.caps().atomic_replace {
            return Err(blocked());
        }
        match std::fs::read_dir(crate::paths::drafts_dir(&dir)) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    return Err(blocked());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(CoreError::io("sync_drafts", "state", &e)),
        }
        let read = |path: &RelPath| -> Result<Option<notes_model::BaseRev>> {
            match service.open()?.fs.stat(path) {
                Ok(stat) => {
                    if stat.kind != EntryKind::File || stat.size > 8 * 1024 * 1024 {
                        return Err(blocked());
                    }
                    let raw = service.open()?.fs.read(path)?;
                    Ok(Some(notes_model::BaseRev {
                        size: stat.size,
                        mtime_ns: stat.mtime_ns,
                        hash: notes_fs::hash(&raw),
                    }))
                }
                Err(CoreError::NotFound { .. })
                | Err(CoreError::Io {
                    kind: notes_model::IoKind::NotFound,
                    ..
                }) => Ok(None),
                Err(e) => Err(e),
            }
        };
        let source = read(from)?;
        if let Some(current) = &source {
            if *current != expected.base_rev
                || service
                    .open()?
                    .registry
                    .record(expected.note_id)
                    .is_none_or(|r| r.path != *from)
            {
                return Err(blocked());
            }
        } else if !retry {
            return Err(blocked());
        }
        let destination = if bytes.is_some() { read(to)? } else { None };
        if let Some(raw) = bytes {
            if destination
                .as_ref()
                .is_some_and(|r| !retry || r.hash != notes_fs::hash(raw))
            {
                return Err(blocked());
            }
            if source.is_none() && destination.is_none() {
                return Err(blocked());
            }
            if destination.is_none() {
                // Existing parent folders only: no partial directory topology.
                let parent = to.parent().ok_or_else(blocked)?;
                if service.open()?.fs.stat(&parent)?.kind != EntryKind::Dir {
                    return Err(blocked());
                }
                service.check_name(to.file_name(), to)?;
            }
        }
        prepare()?;
        if let Some(raw) = bytes {
            if destination.is_none() {
                service.open()?.fs.create_new(to, raw)?;
            }
        }
        // Recheck after destination creation; a third-party edit stays intact.
        if source.is_some() {
            let stat = service.open()?.fs.stat(from)?;
            if stat.size != expected.base_rev.size
                || stat.mtime_ns != expected.base_rev.mtime_ns
                || notes_fs::hash(&service.open()?.fs.read(from)?) != expected.base_rev.hash
            {
                return Err(blocked());
            }
            service.open()?.fs.delete(from)?;
        }
        let result = if let Some(raw) = bytes {
            let stat = service.open()?.fs.stat(to)?;
            let hash = notes_fs::hash(&service.open()?.fs.read(to)?);
            if hash != notes_fs::hash(raw) {
                return Err(blocked());
            }
            let registry = &mut service.open_mut()?.registry;
            if registry
                .record(expected.note_id)
                .is_none_or(|r| r.path != *to)
            {
                registry.forget(to);
                registry.repath(from, to);
            }
            let id = registry.observe(to, &stat, hash.clone());
            if id != expected.note_id {
                return Err(blocked());
            }
            Some(Applied {
                note_id: id,
                base_rev: notes_model::BaseRev {
                    size: stat.size,
                    mtime_ns: stat.mtime_ns,
                    hash,
                },
            })
        } else {
            service.open_mut()?.registry.forget(from);
            None
        };
        crate::state::store(
            &crate::paths::registry_file(&dir),
            &service.open()?.registry,
        )?;
        service.invalidate_paths();
        Ok(result)
    })?
}

/// Capture only locally referenced non-note files, through the workspace jail.
pub fn capture_attachments(
    root: &Path,
    data: &Path,
    path: &RelPath,
    bytes: &[u8],
) -> Result<Vec<notes_sync::transfer::Attachment>> {
    validate_state_location(&[root], data)?;
    let mut service = WorkspaceService::with_data_dir(data)?;
    service.record_visits = false;
    service.open_workspace(root)?;
    let paths = notes_sync::transfer::attachment_paths(path, bytes);
    let blocked = || CoreError::Unsupported {
        cap: "attachment capture is missing, changed or exceeds limits".into(),
    };
    if paths.len() > 32 {
        return Err(blocked());
    }
    let mut total = bytes.len();
    let mut result = vec![];
    for path in paths {
        let stat = service.open()?.fs.stat(&path)?;
        if stat.kind != EntryKind::File || stat.size > 8 * 1024 * 1024 {
            return Err(blocked());
        }
        let raw = service.open()?.fs.read(&path)?;
        let after = service.open()?.fs.stat(&path)?;
        total += raw.len();
        if total > 8 * 1024 * 1024 || stat.size != after.size || stat.mtime_ns != after.mtime_ns {
            return Err(blocked());
        }
        result.push(notes_sync::transfer::Attachment::new(path, &raw));
    }
    Ok(result)
}

/// Restore one referenced binary. Identical existing bytes may be shared;
/// differing bytes require the last durable attachment revision.
pub fn apply_attachment(
    root: &Path,
    data: &Path,
    asset: &notes_sync::transfer::Attachment,
    expected: Option<&notes_model::BaseRev>,
    retry: bool,
    prepare: impl FnOnce() -> Result<()>,
) -> Result<notes_model::BaseRev> {
    restore_attachment(root, data, asset, expected, retry, true, prepare)
}
pub fn confirm_attachment(
    root: &Path,
    data: &Path,
    asset: &notes_sync::transfer::Attachment,
) -> Result<notes_model::BaseRev> {
    restore_attachment(root, data, asset, None, false, false, || Ok(()))
}
fn restore_attachment(
    root: &Path,
    data: &Path,
    asset: &notes_sync::transfer::Attachment,
    expected: Option<&notes_model::BaseRev>,
    retry: bool,
    write: bool,
    prepare: impl FnOnce() -> Result<()>,
) -> Result<notes_model::BaseRev> {
    validate_state_location(&[root], data)?;
    let mut service = WorkspaceService::with_data_dir(data)?;
    service.record_visits = false;
    service.open_sync_workspace(root)?;
    let path = &asset.path;
    let blocked = || CoreError::Unsupported {
        cap: "attachment application precondition failed".into(),
    };
    let bytes = asset.bytes().map_err(|_| blocked())?;
    if path.is_root()
        || path.is_note()
        || path.as_str().split('/').any(crate::ignore::is_hidden_name)
    {
        return Err(blocked());
    }
    for name in path.as_str().split('/') {
        notes_model::portable_name(name).map_err(|_| blocked())?;
    }
    let dir = service.open()?.dir.clone();
    let mut lock = crate::lock::acquire(&crate::paths::lock_file(&dir))?;
    lock.with(|| {
        if service.open()?.read_only || !service.open()?.fs.caps().atomic_replace {
            return Err(blocked());
        }
        match std::fs::read_dir(crate::paths::drafts_dir(&dir)) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    return Err(blocked());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(CoreError::io("sync_drafts", "state", &e)),
        }
        let current = match service.open()?.fs.stat(path) {
            Ok(stat) => {
                if stat.kind != EntryKind::File || stat.size > 8 * 1024 * 1024 {
                    return Err(blocked());
                }
                Some(notes_model::BaseRev {
                    size: stat.size,
                    mtime_ns: stat.mtime_ns,
                    hash: notes_fs::hash(&service.open()?.fs.read(path)?),
                })
            }
            Err(CoreError::NotFound { .. })
            | Err(CoreError::Io {
                kind: notes_model::IoKind::NotFound,
                ..
            }) => None,
            Err(e) => return Err(e),
        };
        let identical = current.as_ref().is_some_and(|r| r.hash == asset.hash);
        if !identical && (!write || current.as_ref() != expected) {
            return Err(blocked());
        }
        // Retry is proof of an interrupted write; identical bytes do not need
        // replacement regardless of whether another note already stored them.
        let _ = retry;
        prepare()?;
        if !identical {
            if let Some(before) = expected {
                if !matches!(
                    service
                        .open()?
                        .fs
                        .write_atomic(path, &bytes, Some(before))?,
                    notes_fs::WriteOutcome::Written(_)
                ) {
                    return Err(blocked());
                }
            } else {
                let mut parent = RelPath::root();
                let names: Vec<_> = path.as_str().split('/').collect();
                for name in &names[..names.len() - 1] {
                    let next = parent.join(name)?;
                    match service.open()?.fs.stat(&next) {
                        Ok(s) if s.kind == EntryKind::Dir => {}
                        Ok(_) => return Err(blocked()),
                        Err(CoreError::NotFound { .. })
                        | Err(CoreError::Io {
                            kind: notes_model::IoKind::NotFound,
                            ..
                        }) => {
                            service.check_name(name, &next)?;
                            service.open()?.fs.create_dir(&next)?;
                        }
                        Err(e) => return Err(e),
                    }
                    parent = next;
                }
                service.check_name(path.file_name(), path)?;
                service.open()?.fs.create_new(path, &bytes)?;
            }
        }
        let stat = service.open()?.fs.stat(path)?;
        let hash = notes_fs::hash(&service.open()?.fs.read(path)?);
        if hash != asset.hash {
            return Err(blocked());
        }
        Ok(notes_model::BaseRev {
            size: stat.size,
            mtime_ns: stat.mtime_ns,
            hash,
        })
    })?
}
