//! Explicitly scoped agent operations. The stdio transport contains no note policy.
use crate::{Result, WorkspaceService};
use notes_fs::{FileSystem, WriteOutcome};
use notes_model::{BaseRev, CoreError, EntryKind, RelPath, TextProfile};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Read,
    Create,
    Update,
    Move,
    Delete,
    Search,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    pub workspace: PathBuf,
    #[serde(default = "RelPath::root")]
    pub scope: RelPath,
    #[serde(default)]
    pub permissions: BTreeSet<Permission>,
    /// Write operations are restricted to scope/proposals. Reads retain scope.
    #[serde(default)]
    pub review: bool,
}
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentArgs {
    pub path: Option<RelPath>,
    pub to: Option<RelPath>,
    pub text: Option<String>,
    pub base_rev: Option<BaseRev>,
    pub query: Option<String>,
    pub limit: Option<usize>,
    /// Offset within the authorized, sorted result set.
    pub offset: Option<usize>,
}
pub struct AgentService {
    service: WorkspaceService,
    config: AgentConfig,
}
fn denied() -> CoreError {
    CoreError::Unsupported {
        cap: "agent permission denied".into(),
    }
}
fn invalid(message: &str) -> CoreError {
    CoreError::Unsupported {
        cap: message.into(),
    }
}
fn within(path: &RelPath, scope: &RelPath) -> bool {
    scope.is_root() || path == scope || path.as_str().starts_with(&format!("{scope}/"))
}
#[derive(Serialize, Deserialize)]
struct AppendReceipt {
    request_hash: notes_model::ContentHash,
    output_hash: notes_model::ContentHash,
    completed: Option<BaseRev>,
}
impl AgentService {
    pub fn new(config: AgentConfig) -> Result<Self> {
        Self::with_service(config, WorkspaceService::new()?)
    }
    pub fn with_data_dir(config: AgentConfig, data: &std::path::Path) -> Result<Self> {
        Self::with_service(config, WorkspaceService::with_data_dir(data)?)
    }
    fn with_service(config: AgentConfig, mut service: WorkspaceService) -> Result<Self> {
        if !config.workspace.is_absolute() {
            return Err(invalid("agent workspace must be absolute"));
        }
        service.record_visits = false;
        service.open_workspace(&config.workspace)?;
        let stat = service.open()?.fs.stat(&config.scope)?;
        if stat.kind != EntryKind::Dir {
            return Err(invalid("agent scope must be a directory"));
        }
        Ok(Self { service, config })
    }
    pub fn permission(tool: &str) -> Option<Permission> {
        Some(match tool {
            "notes_list" | "notes_read" => Permission::Read,
            "notes_search" => Permission::Search,
            "notes_create" => Permission::Create,
            "notes_update" | "notes_append" => Permission::Update,
            "notes_move" => Permission::Move,
            "notes_delete" => Permission::Delete,
            _ => return None,
        })
    }
    pub fn allowed(&self, tool: &str) -> bool {
        Self::permission(tool).is_some_and(|p| self.config.permissions.contains(&p))
    }
    fn authorize(&self, path: &RelPath, write: bool) -> Result<()> {
        if !within(path, &self.config.scope) {
            return Err(denied());
        }
        if write && self.config.review && !within(path, &self.config.scope.join("proposals")?) {
            return Err(denied());
        }
        if write && self.service.open()?.read_only {
            return Err(denied());
        }
        if path.as_str().split('/').any(crate::ignore::is_hidden_name) {
            return Err(denied());
        }
        Ok(())
    }
    fn read(&mut self, path: &RelPath) -> Result<crate::OpenedNote> {
        self.authorize(path, false)?;
        if !path.is_note() {
            return Err(invalid("agent operations require a Markdown note"));
        }
        if self.service.open()?.fs.stat(path)?.size > 8 * 1024 * 1024 {
            return Err(invalid("note exceeds 8 MiB"));
        }
        self.service.open_note(path)
    }
    fn paths(&self) -> Result<Vec<RelPath>> {
        // Walk only the authorized subtree: even enumeration avoids other content.
        let open = self.service.open()?;
        let mut dirs = vec![self.config.scope.clone()];
        let mut paths = vec![];
        while let Some(dir) = dirs.pop() {
            for entry in open.fs.list(&dir)? {
                if crate::ignore::is_hidden_name(&entry.name)
                    || open
                        .extra_ignore
                        .iter()
                        .any(|x| x == &entry.name || x == entry.path.as_str())
                {
                    continue;
                }
                match entry.kind {
                    EntryKind::Dir => dirs.push(entry.path),
                    EntryKind::File if entry.is_note => paths.push(entry.path),
                    _ => {}
                }
            }
        }
        paths.sort();
        Ok(paths)
    }
    pub fn call(&mut self, tool: &str, args: AgentArgs) -> Result<Value> {
        if !self.allowed(tool) {
            return Err(denied());
        }
        let limit = args.limit.unwrap_or(100).clamp(1, 200);
        let offset = args.offset.unwrap_or(0);
        if offset > 1_000_000 {
            return Err(invalid("offset exceeds limit"));
        }
        if tool == "notes_list" {
            let paths = self.paths()?;
            return Ok(
                json!({"paths":paths.iter().skip(offset).take(limit).collect::<Vec<_>>(),"truncated":paths.len()>offset.saturating_add(limit)}),
            );
        }
        if tool == "notes_search" {
            let query = args.query.ok_or_else(|| invalid("query is required"))?;
            if query.is_empty() || query.len() > 4096 {
                return Err(invalid("query must contain 1 to 4096 bytes"));
            }
            let mut hits = vec![];
            let mut skipped = 0;
            let open = self.service.open()?;
            for path in self.paths()? {
                if open.fs.stat(&path)?.size > 8 * 1024 * 1024 {
                    continue;
                }
                let bytes = open.fs.read(&path)?;
                let Some(text) = TextProfile::detect(&bytes).1 else {
                    continue;
                };
                for (line, content) in text.lines().enumerate() {
                    if content.contains(&query) {
                        if skipped < offset {
                            skipped += 1;
                            continue;
                        }
                        hits.push(json!({"path":path,"line":line+1,"context":content.chars().take(240).collect::<String>()}));
                        if hits.len() > limit {
                            hits.truncate(limit);
                            return Ok(json!({"hits":hits,"truncated":true,"mode":"literal"}));
                        }
                    }
                }
            }
            return Ok(json!({"hits":hits,"truncated":false,"mode":"literal"}));
        }
        let path = args.path.ok_or_else(|| invalid("path is required"))?;
        if tool == "notes_read" {
            return Ok(note_value(self.read(&path)?));
        }
        self.authorize(&path, true)?;
        if !path.is_note() {
            return Err(invalid("agent writes require a Markdown note"));
        }
        if matches!(tool, "notes_create" | "notes_update" | "notes_append") && args.text.is_none() {
            return Err(invalid("text is required"));
        }
        let text = args.text.unwrap_or_default();
        if text.len() > 8 * 1024 * 1024 {
            return Err(invalid("text exceeds 8 MiB"));
        }
        let dir = self.service.open()?.dir.clone();
        if tool == "notes_create" {
            let mut lock = crate::lock::acquire(&crate::paths::lock_file(&dir))?;
            return lock.with(|| {
                self.service.check_name(path.file_name(), &path)?;
                self.service.open()?.fs.create_new(&path, text.as_bytes())?;
                self.service.invalidate_paths();
                Ok(note_value(self.service.open_note_locked(&path)?))
            })?;
        }
        let base = args
            .base_rev
            .ok_or_else(|| invalid("base_rev from notes_read is required"))?;
        let note = self.read(&path)?;
        let to = args.to;
        if let Some(to) = &to {
            self.authorize(to, true)?;
            if !to.is_note() {
                return Err(invalid("move destination must be a Markdown note"));
            }
        }
        let mut lock = crate::lock::acquire(&crate::paths::lock_file(&dir))?;
        lock.with(|| {
            let bytes = self.service.open()?.fs.read(&path)?;
            let stat = self.service.open()?.fs.stat(&path)?;
            let current = BaseRev {
                size: stat.size,
                mtime_ns: stat.mtime_ns,
                hash: notes_fs::hash(&bytes),
            };
            let receipt_path = dir.join("agent-appends").join(format!(
                "{}-{}.json",
                note.note_id,
                notes_fs::hash(&serde_json::to_vec(&base).map_err(|_| invalid("invalid base"))?)
                    .to_string()
                    .replace(':', "-")
            ));
            let request_hash = notes_fs::hash(text.as_bytes());
            if tool == "notes_append" && receipt_path.exists() {
                let raw = std::fs::read(&receipt_path)
                    .map_err(|e| CoreError::io("read_append_receipt", "receipt", &e))?;
                let mut receipt: AppendReceipt =
                    serde_json::from_slice(&raw).map_err(|_| invalid("invalid append receipt"))?;
                if receipt.request_hash != request_hash {
                    return Err(invalid("append base was already used with different text"));
                }
                if let Some(rev) = receipt.completed {
                    return Ok(json!({"note_id":note.note_id,"base_rev":rev,"replayed":true}));
                }
                if current.hash == receipt.output_hash {
                    receipt.completed = Some(current.clone());
                    save_receipt(&receipt_path, &receipt)?;
                    return Ok(json!({"note_id":note.note_id,"base_rev":current,"replayed":true}));
                }
            }
            if current != base {
                return Err(CoreError::Conflict {
                    note_id: note.note_id,
                    disk_rev: current,
                });
            }
            if tool == "notes_move" {
                let to = to.as_ref().ok_or_else(|| invalid("to is required"))?;
                self.service.relocate(&path, to, to.file_name())?;
                return Ok(json!({"path":to,"note_id":note.note_id,"links_updated":false}));
            }
            if tool == "notes_delete" {
                let result = self.service.delete_entry_locked(&path)?;
                return serde_json::to_value(result).map_err(|e| CoreError::Internal {
                    message: e.to_string(),
                });
            }
            let (profile, old) = TextProfile::detect(&bytes);
            if let Some(reason) = profile.read_only_reason() {
                return Err(CoreError::ReadOnly {
                    note_id: note.note_id,
                    reason,
                });
            }
            let next = if tool == "notes_append" {
                format!("{}{}", old.unwrap_or_default(), text)
            } else {
                text.clone()
            };
            if next.len() > 8 * 1024 * 1024 {
                return Err(invalid("result exceeds 8 MiB"));
            }
            let encoded = profile.encode(&next);
            let output_hash = notes_fs::hash(&encoded);
            let mut receipt = AppendReceipt {
                request_hash,
                output_hash: output_hash.clone(),
                completed: None,
            };
            if tool == "notes_append" {
                save_receipt(&receipt_path, &receipt)?;
            }
            let stat =
                match self
                    .service
                    .open()?
                    .fs
                    .write_atomic(&path, &encoded, Some(&current))?
                {
                    WriteOutcome::Written(stat) => stat,
                    _ => {
                        return Err(CoreError::Conflict {
                            note_id: note.note_id,
                            disk_rev: current,
                        })
                    }
                };
            let rev = BaseRev {
                size: stat.size,
                mtime_ns: stat.mtime_ns,
                hash: output_hash.clone(),
            };
            self.service
                .open_mut()?
                .registry
                .observe(&path, &stat, output_hash);
            crate::store_registry(&dir, &self.service.open()?.registry)?;
            self.service.invalidate_paths();
            if tool == "notes_append" {
                receipt.completed = Some(rev.clone());
                save_receipt(&receipt_path, &receipt)?;
            }
            Ok(json!({"note_id":note.note_id,"base_rev":rev,"replayed":false}))
        })?
    }
}
fn save_receipt(path: &std::path::Path, receipt: &AppendReceipt) -> Result<()> {
    std::fs::create_dir_all(path.parent().unwrap())
        .map_err(|e| CoreError::io("mkdir_receipt", "receipt", &e))?;
    crate::state::write_atomic(
        path,
        &serde_json::to_vec(receipt).map_err(|e| CoreError::Internal {
            message: e.to_string(),
        })?,
    )
}

fn note_value(note: crate::OpenedNote) -> Value {
    json!({"note_id":note.note_id,"path":note.path,"text":note.text,"base_rev":note.base_rev,"read_only":note.read_only})
}
