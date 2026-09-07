use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;
use uuid::Uuid;

macro_rules! uuid_newtype {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS,
        )]
        #[serde(transparent)]
        #[ts(export, type = "string")]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
            pub fn from_uuid(u: Uuid) -> Self {
                Self(u)
            }
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }
    };
}

uuid_newtype!(
    WorkspaceId,
    "Assigned when a root is registered. Lives only in app data."
);
uuid_newtype!(
    NoteId,
    "Assigned the first time a note is **opened**, and never written into the \
     note. Scope §6.2 and `ARCHITECTURE.md` §18.2."
);

/// A blake3 digest of a note's bytes.
///
/// Serialised as `b3:<hex>` — **prefixed by the algorithm**, so that changing
/// hash one day is a migration and not an ambiguity. It is a *correlation*
/// signal, never identity (`ARCHITECTURE.md` §9).
#[derive(Debug, Clone, PartialEq, Eq, Hash, TS)]
#[ts(export, type = "string")]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub fn from_bytes(b: [u8; 32]) -> Self {
        Self(b)
    }
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    /// True for the digest of an empty input. Zero-byte notes are never
    /// correlated by hash, because every empty file has the same one.
    pub fn is_of_empty(&self) -> bool {
        *self == Self::of_empty()
    }
    /// blake3 of `b""`, hard-coded so this crate stays free of a hash
    /// dependency; `notes-fs` asserts it against the real hasher.
    pub fn of_empty() -> Self {
        Self([
            0xaf, 0x13, 0x49, 0xb9, 0xf5, 0xf9, 0xa1, 0xa6, 0xa0, 0x40, 0x4d, 0xea, 0x36, 0xdc,
            0xc9, 0x49, 0x9b, 0xcb, 0x25, 0xc9, 0xad, 0xc1, 0x12, 0xb7, 0xcc, 0x9a, 0x93, 0xca,
            0xe4, 0x1f, 0x32, 0x62,
        ])
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("b3:")?;
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HashParseError {
    #[error("content hash must start with `b3:`")]
    UnknownAlgorithm,
    #[error("content hash must be 64 hex characters")]
    BadLength,
    #[error("content hash contains a non-hex character")]
    NotHex,
}

impl std::str::FromStr for ContentHash {
    type Err = HashParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s
            .strip_prefix("b3:")
            .ok_or(HashParseError::UnknownAlgorithm)?;
        if hex.len() != 64 {
            return Err(HashParseError::BadLength);
        }
        let mut out = [0u8; 32];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let hi = (chunk[0] as char)
                .to_digit(16)
                .ok_or(HashParseError::NotHex)?;
            let lo = (chunk[1] as char)
                .to_digit(16)
                .ok_or(HashParseError::NotHex)?;
            out[i] = (hi * 16 + lo) as u8;
        }
        Ok(Self(out))
    }
}

impl Serialize for ContentHash {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_round_trips_through_its_string_form() {
        let h = ContentHash::from_bytes([0xab; 32]);
        let s = h.to_string();
        assert!(s.starts_with("b3:"));
        assert_eq!(s.len(), 3 + 64);
        assert_eq!(s.parse::<ContentHash>().unwrap(), h);
    }

    #[test]
    fn hash_rejects_a_foreign_algorithm() {
        let hex = "0".repeat(64);
        assert_eq!(
            format!("sha256:{hex}").parse::<ContentHash>(),
            Err(HashParseError::UnknownAlgorithm)
        );
    }

    #[test]
    fn hash_rejects_wrong_length_and_non_hex() {
        assert_eq!(
            "b3:abc".parse::<ContentHash>(),
            Err(HashParseError::BadLength)
        );
        let bad = format!("b3:{}", "z".repeat(64));
        assert_eq!(bad.parse::<ContentHash>(), Err(HashParseError::NotHex));
    }

    #[test]
    fn ids_are_unique_and_round_trip() {
        let a = NoteId::new();
        assert_ne!(a, NoteId::new());
        assert_eq!(a.to_string().parse::<NoteId>().unwrap(), a);
    }
}
