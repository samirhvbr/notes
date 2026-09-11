//! Shared immutable publication contract; no HTTP client dependency.
use crate::{Error, Result, Revision};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub const MAX_CONTENT: usize = 8 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Publication {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,
    pub workspace: Uuid,
    pub expected: Option<Uuid>,
    pub revision: Revision,
    /// Canonical standard base64; None only for a tombstone.
    pub content_base64: Option<String>,
    /// Original divergent revisions, in parent-before-child order. They never
    /// become visible heads on their own; the enclosing resolution consumes them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branches: Vec<Branch>,
    /// Metadata retained after acknowledged divergent branch payloads are
    /// pruned. These revisions preserve ancestry but are never independently
    /// applicable because their original bytes are no longer present.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<Revision>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Branch {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,
    pub revision: Revision,
    pub content_base64: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Attachment {
    pub path: notes_model::RelPath,
    pub hash: notes_model::ContentHash,
    pub content_base64: String,
}
impl Attachment {
    pub fn new(path: notes_model::RelPath, bytes: &[u8]) -> Self {
        Self {
            path,
            hash: notes_model::ContentHash::from_bytes(*blake3::hash(bytes).as_bytes()),
            content_base64: STANDARD.encode(bytes),
        }
    }
    pub fn bytes(&self) -> Result<Vec<u8>> {
        if self.content_base64.len() > MAX_CONTENT.div_ceil(3) * 4 {
            return Err(Error::Limit);
        }
        let bytes = STANDARD
            .decode(&self.content_base64)
            .map_err(|_| Error::InvalidState)?;
        if bytes.len() > MAX_CONTENT
            || STANDARD.encode(&bytes) != self.content_base64
            || blake3::hash(&bytes).as_bytes() != self.hash.as_bytes()
        {
            return Err(Error::InvalidState);
        }
        Ok(bytes)
    }
}
pub fn attachment_paths(
    path: &notes_model::RelPath,
    bytes: &[u8],
) -> std::collections::BTreeSet<notes_model::RelPath> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Default::default();
    };
    let base = path.parent().unwrap_or_else(notes_model::RelPath::root);
    notes_markdown::parse(text)
        .links
        .into_iter()
        .filter(|l| {
            !l.in_code
                && !l.target.contains(':')
                && !matches!(
                    l.kind,
                    notes_markdown::LinkKind::Url
                        | notes_markdown::LinkKind::Anchor
                        | notes_markdown::LinkKind::Wiki
                )
        })
        .filter_map(|l| {
            notes_markdown::url::resolve_relative(
                &base,
                l.target.split(['#', '?']).next().unwrap_or_default(),
            )
        })
        .filter(|p| {
            !p.is_root() && !p.is_note() && !p.as_str().split('/').any(|s| s.starts_with('.'))
        })
        .collect()
}
fn attachment_size(
    revision: &Revision,
    encoded: &Option<String>,
    assets: &[Attachment],
) -> Result<usize> {
    if assets.len() > 32 || (revision.content.is_none() && !assets.is_empty()) {
        return Err(Error::Limit);
    }
    if assets.is_empty() {
        return Ok(0);
    }
    let paths = attachment_paths(&revision.path, &decode(revision, encoded)?);
    let mut seen = std::collections::BTreeSet::new();
    let mut size = 0;
    for asset in assets {
        if !paths.contains(&asset.path)
            || !seen.insert(&asset.path)
            || asset.path.as_str().len() > 4096
        {
            return Err(Error::InvalidState);
        }
        size += asset.bytes()?.len();
        if size > MAX_CONTENT {
            return Err(Error::Limit);
        }
    }
    Ok(size)
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
    if p.branches.len() + p.history.len() > 20 {
        return Err(Error::Limit);
    }
    let mut size =
        content(p)?.len() + attachment_size(&p.revision, &p.content_base64, &p.attachments)?;
    for b in &p.branches {
        size += decode(&b.revision, &b.content_base64)?.len()
            + attachment_size(&b.revision, &b.content_base64, &b.attachments)?;
    }
    if size > MAX_CONTENT {
        return Err(Error::Limit);
    }
    Ok(size)
}

/// Replace divergent branch payloads with their immutable revision metadata.
/// The enclosing resolution bytes stay available and its causal graph is
/// unchanged. Callers decide whether device receipts authorize this operation.
pub fn prune_resolved_payloads(p: &mut Publication) -> Result<usize> {
    if p.branches.is_empty() {
        return Ok(0);
    }
    let before = payload_size(p)?;
    p.history
        .extend(p.branches.drain(..).map(|branch| branch.revision));
    Ok(before - payload_size(p)?)
}
/// Replay a publication into a disposable journal. Callers validate the complete
/// graph once after replay, and publish the journal only if that succeeds.
pub fn append(graph: &mut crate::Journal, p: &Publication) -> Result<()> {
    payload_size(p)?;
    let r = &p.revision;
    if p.workspace != graph.workspace || graph.heads.get(&r.note).copied() != p.expected {
        return Err(Error::Stale);
    }
    for v in &p.history {
        if v.note != r.note || v.parents.is_empty() {
            return Err(Error::InvalidGraph);
        }
        insert(graph, v)?;
    }
    for b in &p.branches {
        let v = &b.revision;
        if v.note != r.note || v.parents.is_empty() {
            return Err(Error::InvalidGraph);
        }
        insert(graph, v)?;
    }
    if !p.branches.is_empty() || !p.history.is_empty() {
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
                .map(|b| &b.revision)
                .chain(&p.history)
                .any(|branch| !graph.is_ancestor(branch.id, other))
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
