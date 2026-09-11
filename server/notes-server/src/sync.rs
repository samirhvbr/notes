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
    Ok(())
}
fn transaction<T>(
    root: &Path,
    c: &Credential,
    change: impl FnOnce(&mut Vault) -> Result<(T, bool)>,
) -> Result<T> {
    admin::workspace(root, &c.workspace).map_err(|_| Error::Storage)?;
    let base = root.join("sync");
    admin::private_dir(&base).map_err(|_| Error::Storage)?;
    let dir = base.join(&c.workspace);
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
            return if *old == p {
                authorize(v, c, &p)?;
                Ok((p.revision.id, false))
            } else {
                Err(Error::Stale)
            };
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
                    workspace: p.workspace,
                    expected: None,
                    revision: branch.revision.clone(),
                    content_base64: branch.content_base64.clone(),
                    branches: vec![],
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
