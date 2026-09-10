//! Reading and writing schema-versioned state, with one migration rule for all
//! of it.
//!
//! `ARCHITECTURE.md` §4.1 wrote the rule once, inside the registry section, and
//! named `registry.json.bak-<old-schema>` in it — which read as registry-only.
//! Drafts, sessions and settings need the same treatment, so it lives here and
//! every state file goes through it (`docs/DECISIONS-0.1a.md` D-14).

use std::path::Path;

use notes_model::CoreError;
use serde::{de::DeserializeOwned, Serialize};

/// Every state file carries one.
pub trait Schemad: Serialize + DeserializeOwned {
    const CURRENT: u32;
    const NAME: &'static str;
    fn schema(&self) -> u32;
    fn baseline(&self) -> Option<Vec<u8>> {
        None
    }
    fn remember(&self, _bytes: &[u8]) {}
}

/// What happened on load, so the caller can tell "nothing there yet" from
/// "there is something and we must not touch it".
pub enum Loaded<T> {
    Fresh,
    Ok(T),
    /// Written by a newer version of the application. **Never overwritten** —
    /// doing so would destroy state we cannot interpret. The workspace opens
    /// read-only and the UI says why.
    TooNew {
        found: u32,
    },
}

pub fn load<T: Schemad>(path: &Path) -> Result<Loaded<T>, CoreError> {
    let db_path = path.with_extension("db");
    let registry = T::NAME == "registry.json";
    if registry {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                if let Some(found) = v.get("schema").and_then(|v| v.as_u64()) {
                    if found > u64::from(T::CURRENT) {
                        return Ok(Loaded::TooNew {
                            found: found as u32,
                        });
                    }
                }
            }
        }
    }
    let stored = if registry && db_path.exists() {
        notes_index::RegistryStore::open(&db_path)?.read()?
    } else {
        None
    };
    let bytes = match stored.map(Ok).unwrap_or_else(|| std::fs::read(path)) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Loaded::Fresh),
        Err(e) => return Err(CoreError::io("read_state", path.display(), &e)),
    };

    // Read the schema before the body: a file from the future must be detected
    // even when its shape no longer deserialises into ours.
    let probe: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) if registry => {
            return Err(CoreError::Internal {
                message: format!("invalid identity registry: {e}"),
            })
        }
        Err(_) => return Ok(Loaded::Fresh), // corrupt: treated as absent, never as an error the user must clear
    };
    let found = probe.get("schema").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    if found > T::CURRENT {
        return Ok(Loaded::TooNew { found });
    }
    if found < T::CURRENT {
        backup(path, found)?;
        // No migration exists yet — schema 1 is the first. When one does, it
        // runs here, on a file that has already been copied aside.
    }

    match serde_json::from_slice::<T>(&bytes) {
        Ok(v) => {
            v.remember(&bytes);
            Ok(Loaded::Ok(v))
        }
        Err(e) if registry => Err(CoreError::Internal {
            message: format!("invalid identity registry: {e}"),
        }),
        Err(_) => Ok(Loaded::Fresh),
    }
}

fn backup(path: &Path, old_schema: u32) -> Result<(), CoreError> {
    let backup = path.with_extension(format!("json.bak-{old_schema}"));
    std::fs::copy(path, &backup)
        .map(|_| ())
        .map_err(|e| CoreError::io("backup_state", backup.display(), &e))
}

/// Atomic, like every other write in this application: state truncated by a
/// power cut costs the same as a note truncated by one.
pub fn store<T: Schemad>(path: &Path, value: &T) -> Result<(), CoreError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| CoreError::io("mkdir", dir.display(), &e))?;
    }
    // A state file whose `schema` field disagrees with the constant it was
    // written against is the failure this whole module exists to prevent, and
    // it is cheap to refuse at the only place a write happens.
    if value.schema() != T::CURRENT {
        return Err(CoreError::Internal {
            message: format!(
                "{} carries schema {} but the build writes {}",
                T::NAME,
                value.schema(),
                T::CURRENT
            ),
        });
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| CoreError::Internal {
        message: format!("serialising {}: {e}", T::NAME),
    })?;
    if T::NAME == "registry.json" {
        if path.exists() {
            let backup = path.with_extension("json.bak-1");
            if !backup.exists() {
                std::fs::copy(path, &backup)
                    .map_err(|e| CoreError::io("backup_registry", backup.display(), &e))?;
            }
        }
        notes_index::RegistryStore::open(&path.with_extension("db"))?
            .write_merged(value.baseline().as_deref(), &bytes)?;
        value.remember(&bytes);
        Ok(())
    } else {
        write_atomic(path, &bytes)
    }
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    use std::io::Write;
    let dir = path.parent().ok_or_else(|| CoreError::Internal {
        message: "state path has no parent".into(),
    })?;
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("state");
    let tmp = dir.join(format!(".{name}.tmp-{}", std::process::id()));

    let mut f =
        std::fs::File::create(&tmp).map_err(|e| CoreError::io("create_temp", tmp.display(), &e))?;
    let r = (|| -> std::io::Result<()> {
        f.write_all(bytes)?;
        f.sync_all()
    })();
    drop(f);
    if let Err(e) = r {
        let _ = std::fs::remove_file(&tmp);
        return Err(CoreError::io("write_state", path.display(), &e));
    }
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        CoreError::io("replace_state", path.display(), &e)
    })
}
