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
}
pub fn content(p: &Publication) -> Result<Vec<u8>> {
    match (&p.revision.content, &p.content_base64) {
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
