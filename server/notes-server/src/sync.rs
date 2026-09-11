//! Immutable replication inbox, separate from live workspace files.
//! One atomic document commits content and heads together: no dangling blobs.
use crate::admin::{self, Credential};
use notes_core::agent::Permission;
use notes_model::{NoteId, RelPath};
use notes_sync::{Journal, Revision};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};
use uuid::Uuid;

pub use notes_sync::transfer::MAX_CONTENT;
const MAX_TOTAL: usize = 32 * 1024 * 1024;
const MAX_STATE: u64 = 64 * 1024 * 1024;
const MAX_REVISIONS: usize = 10_000;

#[derive(Debug)]
pub enum Error {
    Forbidden,
    Invalid,
    Missing,
    Stale,
    Limit,
    Busy,
    Storage,
}
pub type Result<T> = std::result::Result<T, Error>;
pub use notes_sync::transfer::Publication;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Vault {
    schema: u32,
    journal: Journal,
    // In insertion order, making an integer cursor stable as revisions append.
    publications: Vec<Publication>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    device_owners: BTreeMap<Uuid, Uuid>,
}
impl Vault {
    fn new() -> Self {
        Self {
            schema: 1,
            journal: Journal::new(Uuid::new_v4()),
            publications: vec![],
            device_owners: BTreeMap::new(),
        }
    }
    fn validate(&self) -> Result<()> {
        if self.schema != 1
            || self.publications.len() > MAX_REVISIONS
            || self.journal.revisions.len() > MAX_REVISIONS
        {
            return Err(Error::Storage);
        }
        let mut rebuilt = Journal::new(self.journal.workspace);
        let mut total = 0usize;
        for p in &self.publications {
            if p.workspace != rebuilt.workspace {
                return Err(Error::Storage);
            }
            total = total
                .checked_add(notes_sync::transfer::payload_size(p).map_err(|_| Error::Invalid)?)
                .ok_or(Error::Limit)?;
            if total > MAX_TOTAL {
                return Err(Error::Limit);
            }
            notes_sync::transfer::append(&mut rebuilt, p).map_err(|_| Error::Storage)?;
        }
        if self
            .device_owners
            .keys()
            .ne(self.journal.acknowledgments.keys())
        {
            return Err(Error::Storage);
        }
        rebuilt.acknowledgments = self.journal.acknowledgments.clone();
        rebuilt.validate().map_err(|_| Error::Storage)?;
        if rebuilt != self.journal {
            return Err(Error::Storage);
        }
        Ok(())
    }
    fn visible(&self, note: NoteId, credential: &Credential) -> bool {
        self.journal
            .revisions
            .values()
            .filter(|r| r.note == note)
            .all(|r| allowed_path(&r.path, credential, false))
            && self
                .publications
                .iter()
                .filter(|p| p.revision.note == note)
                .all(|p| {
                    p.attachments
                        .iter()
                        .chain(p.branches.iter().flat_map(|b| &b.attachments))
                        .all(|a| allowed_attachment(&a.path, credential, false))
                })
    }
}
fn sync_error(e: notes_sync::Error) -> Error {
    match e {
        notes_sync::Error::Stale | notes_sync::Error::Collision => Error::Stale,
        notes_sync::Error::Limit => Error::Limit,
        _ => Error::Invalid,
    }
}
fn within(path: &RelPath, scope: &RelPath) -> bool {
    scope.is_root() || path == scope || path.as_str().starts_with(&format!("{scope}/"))
}
fn allowed_path(path: &RelPath, c: &Credential, write: bool) -> bool {
    path.as_str().len() <= 4096
        && path.is_note()
        && !path.as_str().split('/').any(|s| s.starts_with('.'))
        && within(path, &c.scope)
        && (!write
            || !c.review
            || c.scope
                .join("proposals")
                .is_ok_and(|scope| within(path, &scope)))
}
fn require(c: &Credential, permission: Permission) -> Result<()> {
    if c.permissions.contains(&permission) {
        Ok(())
    } else {
        Err(Error::Forbidden)
    }
}
fn authorize(v: &Vault, c: &Credential, p: &Publication) -> Result<()> {
    require(c, Permission::Read)?;
    if !allowed_path(&p.revision.path, c, true) || !v.visible(p.revision.note, c) {
        return Err(Error::Forbidden);
    }
    if p.revision.parents.is_empty() {
        require(c, Permission::Create)?;
    }
    for id in &p.revision.parents {
        let parent = v.journal.revisions.get(id).ok_or(Error::Invalid)?;
        if parent.note != p.revision.note {
            return Err(Error::Invalid);
        }
        if !allowed_path(&parent.path, c, true) {
            return Err(Error::Forbidden);
        }
        if parent.path != p.revision.path {
            require(c, Permission::Move)?;
        }
        if p.revision.content.is_some() && parent.content != p.revision.content {
            require(
                c,
                if parent.content.is_none() {
                    Permission::Create
                } else {
                    Permission::Update
                },
            )?;
        }
    }
    if p.revision.content.is_none() {
        require(c, Permission::Delete)?;
    }
    // A revision that changes no bytes still changes history.
    if !p.revision.parents.is_empty() && p.revision.content.is_some() {
        require(c, Permission::Update)?;
    }
    if !p.attachments.is_empty() {
        require(c, Permission::Create)?;
        require(c, Permission::Update)?;
    }
    if p.attachments
        .iter()
        .any(|a| !allowed_attachment(&a.path, c, true))
    {
        return Err(Error::Forbidden);
    }
    Ok(())
}
fn allowed_attachment(path: &RelPath, c: &Credential, write: bool) -> bool {
    path.as_str().len() <= 4096
        && !path.is_root()
        && !path.is_note()
        && !path.as_str().split('/').any(|s| s.starts_with('.'))
        && within(path, &c.scope)
        && (!write
            || !c.review
            || c.scope
                .join("proposals")
                .is_ok_and(|scope| within(path, &scope)))
}
fn transaction_workspace<T>(
    root: &Path,
    workspace: &str,
    change: impl FnOnce(&mut Vault) -> Result<(T, bool)>,
) -> Result<T> {
    admin::workspace(root, workspace).map_err(|_| Error::Storage)?;
    let base = root.join("sync");
    admin::private_dir(&base).map_err(|_| Error::Storage)?;
    let dir = base.join(workspace);
    admin::private_dir(&dir).map_err(|_| Error::Storage)?;
    let mut lock = fd_lock::RwLock::new(
        admin::private_file(&dir.join("vault.lock"), false).map_err(|_| Error::Storage)?,
    );
    let _guard = lock.try_write().map_err(|_| Error::Busy)?;
    let target = dir.join("vault.json");
    let exists = target.try_exists().map_err(|_| Error::Storage)?;
    let mut vault = if exists {
        let meta = fs::symlink_metadata(&target).map_err(|_| Error::Storage)?;
        if !meta.is_file() || meta.len() > MAX_STATE {
            return Err(Error::Storage);
        }
        let mut bytes = vec![];
        File::open(&target)
            .map_err(|_| Error::Storage)?
            .take(MAX_STATE + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Storage)?;
        if bytes.len() as u64 > MAX_STATE {
            return Err(Error::Storage);
        }
        let v: Vault = serde_json::from_slice(&bytes).map_err(|_| Error::Storage)?;
        v.validate().map_err(|_| Error::Storage)?;
        v
    } else {
        Vault::new()
    };
    let (output, changed) = change(&mut vault)?;
    if changed || !exists {
        let bytes = serde_json::to_vec(&vault).map_err(|_| Error::Storage)?;
        if bytes.len() as u64 > MAX_STATE {
            return Err(Error::Limit);
        }
        let mut temp = tempfile::NamedTempFile::new_in(&dir).map_err(|_| Error::Storage)?;
        temp.write_all(&bytes).map_err(|_| Error::Storage)?;
        temp.as_file().sync_all().map_err(|_| Error::Storage)?;
        temp.persist(&target).map_err(|_| Error::Storage)?;
        #[cfg(unix)]
        File::open(&dir)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Storage)?;
    }
    Ok(output)
}
fn transaction<T>(
    root: &Path,
    c: &Credential,
    change: impl FnOnce(&mut Vault) -> Result<(T, bool)>,
) -> Result<T> {
    transaction_workspace(root, &c.workspace, change)
}

#[derive(Debug, Serialize)]
pub struct PruneReport {
    pub pruned_resolutions: usize,
    pub pruned_payload_bytes: usize,
    pub retained_revisions: usize,
    pub known_devices: usize,
}

#[derive(Debug, Serialize)]
pub struct RetireDeviceReport {
    pub retired_device: Uuid,
    pub removed_receipts: usize,
    pub retained_devices: usize,
}

#[derive(Debug, Serialize)]
pub struct DeviceStatus {
    pub device: Uuid,
    pub credential: Uuid,
    pub credential_revoked: bool,
    pub receipts: usize,
}

pub fn devices(root: &Path, workspace: &str) -> Result<Vec<DeviceStatus>> {
    let credentials = admin::load(root).map_err(|_| Error::Storage)?;
    transaction_workspace(root, workspace, |v| {
        let rows = v
            .device_owners
            .iter()
            .map(|(device, credential)| DeviceStatus {
                device: *device,
                credential: *credential,
                credential_revoked: credentials
                    .credentials
                    .iter()
                    .find(|candidate| candidate.id == *credential)
                    .is_some_and(|candidate| candidate.revoked),
                receipts: v
                    .journal
                    .acknowledgments
                    .get(device)
                    .map_or(0, BTreeMap::len),
            })
            .collect();
        Ok((rows, false))
    })
}

/// Permanently remove one device's application receipts after the credential
/// that owned it has been revoked. The offline operator lock prevents a live
/// server from racing the retirement or allowing that credential to re-enroll.
pub fn retire_device(root: &Path, workspace: &str, device: Uuid) -> Result<RetireDeviceReport> {
    if device.is_nil() {
        return Err(Error::Invalid);
    }
    let credentials = admin::load(root).map_err(|_| Error::Storage)?;
    transaction_workspace(root, workspace, |v| {
        let owner = *v.device_owners.get(&device).ok_or(Error::Missing)?;
        if !credentials
            .credentials
            .iter()
            .any(|credential| credential.id == owner && credential.revoked)
        {
            return Err(Error::Forbidden);
        }
        let removed_receipts = v
            .journal
            .acknowledgments
            .remove(&device)
            .ok_or(Error::Storage)?
            .len();
        v.device_owners.remove(&device).ok_or(Error::Storage)?;
        v.validate()?;
        Ok((
            RetireDeviceReport {
                retired_device: device,
                removed_receipts,
                retained_devices: v.device_owners.len(),
            },
            true,
        ))
    })
}

/// Drop only imported branch payloads whose enclosing resolution has been
/// acknowledged by every known device. Revision metadata and tombstones stay
/// in the graph, so ancestry and future compare-and-set checks remain intact.
pub fn prune_resolved(root: &Path, workspace: &str) -> Result<PruneReport> {
    transaction_workspace(root, workspace, |v| {
        let known_devices = v.device_owners.len();
        let mut pruned_resolutions = 0usize;
        let mut pruned_payload_bytes = 0usize;
        if known_devices > 0 {
            let children = revision_children(&v.journal);
            let candidates: Vec<_> = v
                .publications
                .iter()
                .map(|publication| {
                    (
                        !publication.branches.is_empty()
                            && acknowledged_by_every_device(v, publication.revision.id),
                        linear_payload_is_prunable(v, &children, publication),
                    )
                })
                .collect();
            for (publication, (resolved, linear)) in v.publications.iter_mut().zip(candidates) {
                if publication.branches.is_empty() || !resolved {
                    if !linear {
                        continue;
                    }
                    pruned_payload_bytes += notes_sync::transfer::prune_linear_payload(publication)
                        .map_err(|_| Error::Storage)?;
                    pruned_resolutions += 1;
                    continue;
                }
                pruned_payload_bytes += notes_sync::transfer::prune_resolved_payloads(publication)
                    .map_err(|_| Error::Storage)?;
                pruned_resolutions += 1;
            }
        }
        v.validate()?;
        Ok((
            PruneReport {
                pruned_resolutions,
                pruned_payload_bytes,
                retained_revisions: v.journal.revisions.len(),
                known_devices,
            },
            pruned_resolutions > 0,
        ))
    })
}

fn acknowledged_by_every_device(vault: &Vault, revision: Uuid) -> bool {
    vault.journal.acknowledgments.keys().all(|device| {
        vault
            .journal
            .acknowledgments
            .get(device)
            .and_then(|notes| notes.get(&vault.journal.revisions[&revision].note))
            .is_some_and(|receipt| vault.journal.is_ancestor(revision, *receipt))
    })
}

fn revision_children(journal: &Journal) -> BTreeMap<Uuid, Vec<Uuid>> {
    let mut children = BTreeMap::new();
    for revision in journal.revisions.values() {
        for parent in &revision.parents {
            children
                .entry(*parent)
                .or_insert_with(Vec::new)
                .push(revision.id);
        }
    }
    children
}

/// A compacted linear entry stays in its original append-log slot. Its current
/// live head must retain bytes, and every edge to that head must be one-parent,
/// one-child; a merge or divergence therefore never loses a payload here.
fn linear_payload_is_prunable(
    vault: &Vault,
    children: &BTreeMap<Uuid, Vec<Uuid>>,
    publication: &Publication,
) -> bool {
    if publication.payload_pruned
        || publication.revision.content.is_none()
        || !publication.branches.is_empty()
        || !publication.history.is_empty()
        || !acknowledged_by_every_device(vault, publication.revision.id)
    {
        return false;
    }
    let Some(mut next) = vault.journal.heads.get(&publication.revision.note).copied() else {
        return false;
    };
    if next == publication.revision.id
        || vault
            .journal
            .revisions
            .get(&next)
            .is_none_or(|head| head.content.is_none())
    {
        return false;
    }
    let mut backward = Vec::new();
    while next != publication.revision.id {
        let revision = &vault.journal.revisions[&next];
        if revision.parents.len() != 1 {
            return false;
        }
        backward.push(next);
        next = *revision.parents.iter().next().unwrap();
    }
    backward.into_iter().all(|id| {
        let parent = vault.journal.revisions[&id].parents.iter().next().copied();
        parent.is_some_and(|parent| children.get(&parent).is_some_and(|rows| rows.len() == 1))
    })
}
#[derive(Serialize)]
pub struct Page {
    pub workspace: Uuid,
    pub revisions: Vec<Revision>,
    pub heads: BTreeMap<NoteId, Uuid>,
    pub next_cursor: usize,
    pub has_more: bool,
}
/// Cursor is a consumed append-log position, never a clock. Empty filtered
/// pages still advance. Head values are a current view, not a frozen snapshot.
pub fn page(root: &Path, c: &Credential, cursor: usize, limit: usize) -> Result<Page> {
    require(c, Permission::Read)?;
    if !(1..=200).contains(&limit) {
        return Err(Error::Invalid);
    }
    transaction(root, c, |v| {
        if cursor > v.publications.len() {
            return Err(Error::Invalid);
        }
        let end = (cursor + limit).min(v.publications.len());
        let revisions: Vec<_> = v.publications[cursor..end]
            .iter()
            .filter(|p| v.visible(p.revision.note, c))
            .map(|p| p.revision.clone())
            .collect();
        let heads = revisions
            .iter()
            .filter_map(|r| v.journal.heads.get(&r.note).map(|id| (r.note, *id)))
            .collect();
        Ok((
            Page {
                workspace: v.journal.workspace,
                revisions,
                heads,
                next_cursor: end,
                has_more: end < v.publications.len(),
            },
            false,
        ))
    })
}
pub fn fetch(root: &Path, c: &Credential, id: Uuid) -> Result<Publication> {
    require(c, Permission::Read)?;
    transaction(root, c, |v| {
        let p = v
            .publications
            .iter()
            .find(|p| p.revision.id == id)
            .ok_or(Error::Missing)?;
        if !v.visible(p.revision.note, c) {
            return Err(Error::Missing);
        }
        Ok((p.clone(), false))
    })
}
pub fn publish(root: &Path, c: &Credential, p: Publication) -> Result<Uuid> {
    require(c, Permission::Read)?;
    // Metadata-only publications are produced exclusively by the offline
    // operator prune. A credential can retry its original envelope, never
    // manufacture or replay the compacted form.
    if p.payload_pruned {
        return Err(Error::Invalid);
    }
    let size = notes_sync::transfer::payload_size(&p).map_err(sync_error)?;
    transaction(root, c, |v| {
        if p.workspace != v.journal.workspace {
            return Err(Error::Stale);
        }
        if let Some(old) = v
            .publications
            .iter()
            .find(|old| old.revision.id == p.revision.id)
        {
            let mut compacted_retry = p.clone();
            let is_pre_prune_retry = p.history.is_empty()
                && !p.branches.is_empty()
                && notes_sync::transfer::prune_resolved_payloads(&mut compacted_retry).is_ok()
                && *old == compacted_retry;
            let mut linear_retry = p.clone();
            let is_pre_linear_prune_retry = p.branches.is_empty()
                && p.history.is_empty()
                && notes_sync::transfer::prune_linear_payload(&mut linear_retry).is_ok()
                && *old == linear_retry;
            return if *old == p || is_pre_prune_retry || is_pre_linear_prune_retry {
                if is_pre_prune_retry {
                    for branch in &p.branches {
                        authorize(
                            v,
                            c,
                            &Publication {
                                attachments: branch.attachments.clone(),
                                workspace: p.workspace,
                                expected: None,
                                revision: branch.revision.clone(),
                                content_base64: branch.content_base64.clone(),
                                branches: vec![],
                                history: vec![],
                                payload_pruned: false,
                            },
                        )?;
                    }
                }
                authorize(v, c, &p)?;
                Ok((p.revision.id, false))
            } else {
                Err(Error::Stale)
            };
        }
        // Metadata-only history is produced only by the local operator prune.
        // Remote publishers must always provide original branch payloads.
        if !p.history.is_empty() {
            return Err(Error::Invalid);
        }
        let total = v
            .publications
            .iter()
            .try_fold(size, |sum, old| -> Result<usize> {
                Ok(sum + notes_sync::transfer::payload_size(old).map_err(|_| Error::Invalid)?)
            })?;
        if total > MAX_TOTAL || v.publications.len() >= MAX_REVISIONS {
            return Err(Error::Limit);
        }
        // Work only on the transaction's disposable state. Validate and authorize
        // each imported edge before authorizing the merge against both parents.
        let mut next = v.journal.clone();
        notes_sync::transfer::append(&mut next, &p).map_err(sync_error)?;
        if next.revisions.len() > MAX_REVISIONS {
            return Err(Error::Limit);
        }
        next.validate().map_err(sync_error)?;
        for branch in &p.branches {
            authorize(
                v,
                c,
                &Publication {
                    attachments: branch.attachments.clone(),
                    workspace: p.workspace,
                    expected: None,
                    revision: branch.revision.clone(),
                    content_base64: branch.content_base64.clone(),
                    branches: vec![],
                    history: vec![],
                    payload_pruned: false,
                },
            )?;
            v.journal
                .revisions
                .insert(branch.revision.id, branch.revision.clone());
        }
        authorize(v, c, &p)?;
        v.journal = next;
        let id = p.revision.id;
        v.publications.push(p);
        Ok((id, true))
    })
}

/// Authenticated historical assertion; never authorizes pruning or source writes.
pub fn acknowledge(
    root: &Path,
    c: &Credential,
    receipt: &notes_sync::transfer::ApplicationAcknowledgment,
) -> Result<()> {
    require(c, Permission::Read)?;
    if receipt.device.is_nil() {
        return Err(Error::Invalid);
    }
    transaction(root, c, |v| {
        if receipt.workspace != v.journal.workspace {
            return Err(Error::Stale);
        }
        if v.device_owners
            .get(&receipt.device)
            .is_some_and(|id| *id != c.id)
        {
            return Err(Error::Forbidden);
        }
        let r = v
            .journal
            .revisions
            .get(&receipt.revision)
            .ok_or(Error::Missing)?;
        if !v.visible(r.note, c) {
            return Err(Error::Missing);
        }
        let note = r.note;
        let old = v
            .journal
            .acknowledgments
            .get(&receipt.device)
            .and_then(|h| h.get(&note))
            .copied();
        v.journal
            .acknowledge(receipt.device, BTreeMap::from([(note, receipt.revision)]))
            .map_err(|e| match e {
                notes_sync::Error::Stale => Error::Stale,
                notes_sync::Error::Limit => Error::Limit,
                _ => Error::Invalid,
            })?;
        v.device_owners.insert(receipt.device, c.id);
        Ok(((), old != Some(receipt.revision)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use notes_model::{ContentHash, RelPath};
    use std::collections::BTreeSet;

    fn publication(
        workspace: Uuid,
        expected: Option<Uuid>,
        revision: Revision,
        bytes: &[u8],
    ) -> Publication {
        Publication {
            attachments: vec![],
            workspace,
            expected,
            revision,
            content_base64: Some(STANDARD.encode(bytes)),
            branches: vec![],
            history: vec![],
            payload_pruned: false,
        }
    }

    #[test]
    fn linear_retention_keeps_the_live_head_as_the_baseline() {
        let workspace = Uuid::new_v4();
        let note = NoteId::default();
        let device = Uuid::new_v4();
        let path = RelPath::parse("note.md").unwrap();
        let revision = |parent: Option<Uuid>, bytes: &[u8]| {
            Revision::new(
                note,
                parent.into_iter().collect::<BTreeSet<_>>(),
                Uuid::new_v4(),
                path.clone(),
                Some(ContentHash::from_bytes(*blake3::hash(bytes).as_bytes())),
            )
        };
        let first = revision(None, b"one");
        let second = revision(Some(first.id), b"two");
        let head = revision(Some(second.id), b"three");
        let mut journal = Journal::new(workspace);
        for (expected, value) in [
            (None, first.clone()),
            (Some(first.id), second.clone()),
            (Some(second.id), head.clone()),
        ] {
            journal.commit(value, expected).unwrap();
        }
        journal
            .acknowledge(device, BTreeMap::from([(note, head.id)]))
            .unwrap();
        let vault = Vault {
            schema: 1,
            journal,
            publications: vec![
                publication(workspace, None, first.clone(), b"one"),
                publication(workspace, Some(first.id), second.clone(), b"two"),
                publication(workspace, Some(second.id), head.clone(), b"three"),
            ],
            device_owners: BTreeMap::from([(device, Uuid::new_v4())]),
        };
        let children = revision_children(&vault.journal);
        assert!(linear_payload_is_prunable(
            &vault,
            &children,
            &vault.publications[0]
        ));
        assert!(linear_payload_is_prunable(
            &vault,
            &children,
            &vault.publications[1]
        ));
        assert!(!linear_payload_is_prunable(
            &vault,
            &children,
            &vault.publications[2]
        ));
    }
}
