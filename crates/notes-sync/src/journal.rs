use notes_model::{ContentHash, NoteId, RelPath};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const SCHEMA: u32 = 1;
pub const MAX_REVISIONS: usize = 100_000;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("unsupported sync schema")]
    Schema,
    #[error("invalid revision graph")]
    InvalidGraph,
    #[error("revision no longer matches the observed head")]
    Stale,
    #[error("destination belongs to another note")]
    Collision,
    #[error("pairing mode does not match the two folders")]
    Pairing,
    #[error("sync storage limit reached")]
    Limit,
    #[error("invalid sync state")]
    InvalidState,
    #[error("sync state could not be read or persisted")]
    Storage,
    #[error("sync state is locked by another process")]
    Busy,
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub id: Uuid,
    pub note: NoteId,
    pub parents: BTreeSet<Uuid>,
    pub device: Uuid,
    pub path: RelPath,
    /// None is a tombstone. Empty files still have a real content hash.
    pub content: Option<ContentHash>,
}
impl Revision {
    pub fn new(
        note: NoteId,
        parents: BTreeSet<Uuid>,
        device: Uuid,
        path: RelPath,
        content: Option<ContentHash>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            note,
            parents,
            device,
            path,
            content,
        }
    }
    pub fn same_value(&self, other: &Self) -> bool {
        self.path == other.path && self.content == other.content
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Journal {
    pub schema: u32,
    pub workspace: Uuid,
    pub revisions: BTreeMap<Uuid, Revision>,
    pub heads: BTreeMap<NoteId, Uuid>,
    /// A receipt names the exact heads this device applied, not a wall clock.
    pub acknowledgments: BTreeMap<Uuid, BTreeMap<NoteId, Uuid>>,
}
impl Journal {
    pub fn new(workspace: Uuid) -> Self {
        Self {
            schema: SCHEMA,
            workspace,
            revisions: BTreeMap::new(),
            heads: BTreeMap::new(),
            acknowledgments: BTreeMap::new(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema != SCHEMA {
            return Err(Error::Schema);
        }
        if self.revisions.len() > MAX_REVISIONS || self.acknowledgments.len() > 1024 {
            return Err(Error::Limit);
        }
        let mut roots = BTreeSet::new();
        for (id, revision) in &self.revisions {
            if *id != revision.id || !revision.path.is_note() || revision.parents.len() > 2 {
                return Err(Error::InvalidGraph);
            }
            if revision.parents.is_empty()
                && (revision.content.is_none() || !roots.insert(revision.note))
            {
                return Err(Error::InvalidGraph);
            }
            for parent in &revision.parents {
                if self
                    .revisions
                    .get(parent)
                    .is_none_or(|p| p.note != revision.note)
                {
                    return Err(Error::InvalidGraph);
                }
            }
        }
        // Iterative topological elimination rejects cycles without recursion on
        // attacker-provided history. Each edge is processed once.
        let mut pending: BTreeMap<_, _> = self
            .revisions
            .iter()
            .map(|(id, r)| (*id, r.parents.len()))
            .collect();
        let mut children: BTreeMap<Uuid, Vec<Uuid>> = BTreeMap::new();
        for (id, r) in &self.revisions {
            for parent in &r.parents {
                children.entry(*parent).or_default().push(*id);
            }
        }
        let mut ready: Vec<_> = pending
            .iter()
            .filter(|(_, n)| **n == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut visited = 0;
        while let Some(id) = ready.pop() {
            visited += 1;
            for child in children.get(&id).into_iter().flatten() {
                let remaining = pending.get_mut(child).ok_or(Error::InvalidGraph)?;
                *remaining -= 1;
                if *remaining == 0 {
                    ready.push(*child);
                }
            }
        }
        if visited != self.revisions.len() {
            return Err(Error::InvalidGraph);
        }
        for (note, head) in &self.heads {
            if self.revisions.get(head).is_none_or(|r| r.note != *note) {
                return Err(Error::InvalidGraph);
            }
        }
        for receipt in self.acknowledgments.values() {
            for (note, head) in receipt {
                if self.revisions.get(head).is_none_or(|r| r.note != *note) {
                    return Err(Error::InvalidGraph);
                }
            }
        }
        self.check_collisions(None)?;
        Ok(())
    }
    fn check_collisions(&self, replacement: Option<&Revision>) -> Result<()> {
        let mut paths = BTreeMap::new();
        for (note, head) in &self.heads {
            let value = if replacement.is_some_and(|r| r.note == *note) {
                replacement.unwrap()
            } else {
                self.revisions.get(head).ok_or(Error::InvalidGraph)?
            };
            if value.content.is_some() && paths.insert(value.path.clone(), value.note).is_some() {
                return Err(Error::Collision);
            }
        }
        if let Some(r) =
            replacement.filter(|r| !self.heads.contains_key(&r.note) && r.content.is_some())
        {
            if paths.contains_key(&r.path) {
                return Err(Error::Collision);
            }
        }
        Ok(())
    }
    pub fn head(&self, note: NoteId) -> Option<&Revision> {
        self.heads.get(&note).and_then(|id| self.revisions.get(id))
    }
    pub fn is_ancestor(&self, ancestor: Uuid, descendant: Uuid) -> bool {
        let mut pending = vec![descendant];
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            if id == ancestor {
                return self.revisions.contains_key(&ancestor);
            }
            if let Some(r) = self.revisions.get(&id) {
                pending.extend(r.parents.iter().copied());
            }
        }
        false
    }
    /// Integrate immutable history without choosing a current head. Two peers
    /// reusing one revision UUID for different facts are rejected atomically.
    pub fn import(&mut self, revisions: impl IntoIterator<Item = Revision>) -> Result<()> {
        let mut next = self.clone();
        for revision in revisions {
            if let Some(existing) = next.revisions.get(&revision.id) {
                if existing != &revision {
                    return Err(Error::InvalidGraph);
                }
            } else {
                next.revisions.insert(revision.id, revision);
            }
        }
        next.validate()?;
        *self = next;
        Ok(())
    }
    /// Compare-and-set is separate from ancestry: nobody can consume a head
    /// another process has already replaced, even when the bytes are identical.
    pub fn commit(&mut self, revision: Revision, expected: Option<Uuid>) -> Result<()> {
        if self.heads.get(&revision.note).copied() != expected {
            return Err(Error::Stale);
        }
        if self.revisions.contains_key(&revision.id) {
            return Err(Error::InvalidGraph);
        }
        if let Some(parent) = expected {
            if !revision.parents.contains(&parent) {
                return Err(Error::InvalidGraph);
            }
        } else if !revision.parents.is_empty() {
            return Err(Error::InvalidGraph);
        }
        self.check_collisions(Some(&revision))?;
        let mut next = self.clone();
        next.heads.insert(revision.note, revision.id);
        next.revisions.insert(revision.id, revision);
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn advance(&mut self, note: NoteId, expected: Option<Uuid>, next: Uuid) -> Result<()> {
        if self.heads.get(&note).copied() != expected {
            return Err(Error::Stale);
        }
        let value = self.revisions.get(&next).ok_or(Error::InvalidGraph)?;
        if value.note != note || expected.is_some_and(|old| !self.is_ancestor(old, next)) {
            return Err(Error::InvalidGraph);
        }
        self.check_collisions(Some(value))?;
        self.heads.insert(note, next);
        Ok(())
    }
    pub fn acknowledge(&mut self, device: Uuid, heads: BTreeMap<NoteId, Uuid>) -> Result<()> {
        if !self.acknowledgments.contains_key(&device) && self.acknowledgments.len() >= 1024 {
            return Err(Error::Limit);
        }
        for (note, id) in &heads {
            if self.revisions.get(id).is_none_or(|r| r.note != *note) {
                return Err(Error::InvalidGraph);
            }
            if let Some(previous) = self.acknowledgments.get(&device).and_then(|h| h.get(note)) {
                if !self.is_ancestor(*previous, *id) {
                    return Err(Error::Stale);
                }
            }
        }
        self.acknowledgments
            .entry(device)
            .or_default()
            .extend(heads);
        Ok(())
    }
}
