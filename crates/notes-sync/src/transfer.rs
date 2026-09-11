//! Shared immutable publication contract; no HTTP client dependency.
use crate::{Error, Result, Revision};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub const MAX_CONTENT: usize = 8 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Publication {
    pub workspace: Uuid,
    pub expected: Option<Uuid>,
    pub revision: Revision,
    /// Canonical standard base64; None only for a tombstone.
    pub content_base64: Option<String>,
    /// Original divergent revisions, in parent-before-child order. They never
    /// become visible heads on their own; the enclosing resolution consumes them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branches: Vec<Branch>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Branch {
    pub revision: Revision,
    pub content_base64: Option<String>,
}
pub fn content(p: &Publication) -> Result<Vec<u8>> {
    decode(&p.revision, &p.content_base64)
}
fn decode(revision: &Revision, encoded: &Option<String>) -> Result<Vec<u8>> {
    match (&revision.content, encoded) {
        (None, None) => Ok(vec![]),
        (Some(hash), Some(encoded)) if encoded.len() <= MAX_CONTENT.div_ceil(3) * 4 => {
            let bytes = STANDARD.decode(encoded).map_err(|_| Error::InvalidState)?;
            if bytes.len() > MAX_CONTENT {
                return Err(Error::Limit);
            }
            if STANDARD.encode(&bytes) != *encoded
                || blake3::hash(&bytes).as_bytes() != hash.as_bytes()
            {
                return Err(Error::InvalidState);
            }
            Ok(bytes)
        }
        _ => Err(Error::InvalidState),
    }
}

/// A device reports a durable source-application receipt, never mere storage.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ApplicationAcknowledgment {
    pub workspace: Uuid,
    pub device: Uuid,
    pub revision: Uuid,
}

/// Decoded capacity includes retained branches, not just the chosen result.
pub fn payload_size(p: &Publication) -> Result<usize> {
    if p.branches.len() > 20 {
        return Err(Error::Limit);
    }
    let mut size = content(p)?.len();
    for b in &p.branches {
        size += decode(&b.revision, &b.content_base64)?.len();
    }
    if size > MAX_CONTENT {
        return Err(Error::Limit);
    }
    Ok(size)
}
/// Replay a publication into a disposable journal. Callers validate the complete
/// graph once after replay, and publish the journal only if that succeeds.
pub fn append(graph: &mut crate::Journal, p: &Publication) -> Result<()> {
    payload_size(p)?;
    let r = &p.revision;
    if p.workspace != graph.workspace || graph.heads.get(&r.note).copied() != p.expected {
        return Err(Error::Stale);
    }
    for b in &p.branches {
        let v = &b.revision;
        if v.note != r.note || v.parents.is_empty() {
            return Err(Error::InvalidGraph);
        }
        insert(graph, v)?;
    }
    if !p.branches.is_empty() {
        let expected = p.expected.ok_or(Error::InvalidGraph)?;
        if r.parents.len() != 2 || !r.parents.contains(&expected) {
            return Err(Error::InvalidGraph);
        }
        let other = *r
            .parents
            .iter()
            .find(|id| **id != expected)
            .ok_or(Error::InvalidGraph)?;
        if graph.is_ancestor(expected, other)
            || graph.is_ancestor(other, expected)
            || p.branches
                .iter()
                .any(|b| !graph.is_ancestor(b.revision.id, other))
        {
            return Err(Error::InvalidGraph);
        }
    }
    if p.expected.is_some_and(|id| !r.parents.contains(&id))
        || (p.expected.is_none() && !r.parents.is_empty())
    {
        return Err(Error::InvalidGraph);
    }
    insert(graph, r)?;
    graph.heads.insert(r.note, r.id);
    Ok(())
}
fn insert(graph: &mut crate::Journal, r: &Revision) -> Result<()> {
    if !r.path.is_note()
        || r.parents.len() > 2
        || graph.revisions.contains_key(&r.id)
        || r.parents
            .iter()
            .any(|id| graph.revisions.get(id).is_none_or(|p| p.note != r.note))
    {
        return Err(Error::InvalidGraph);
    }
    graph.revisions.insert(r.id, r.clone());
    Ok(())
}
