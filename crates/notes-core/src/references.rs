//! Review first, then guarded writes with recoverable originals in app data.
use crate::{Result, WorkspaceService};
use notes_fs::{FileSystem, WriteOutcome};
use notes_markdown::rewrite::{self, LinkEdit};
use notes_model::{BaseRev, CoreError, RelPath, TextProfile, WorkspaceId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReferenceFile {
    pub path: RelPath,
    pub destination: RelPath,
    pub edits: Vec<LinkEdit>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReferencePlan {
    pub token: String,
    pub from: RelPath,
    pub to: RelPath,
    pub files: Vec<ReferenceFile>,
    pub skipped: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReferenceResult {
    pub path: RelPath,
    pub updated: Vec<RelPath>,
    pub failed: Vec<RelPath>,
    pub backup: String,
}
struct Original {
    bytes: Vec<u8>,
    base: BaseRev,
    text: String,
    profile: TextProfile,
}
pub(crate) struct Pending {
    workspace: WorkspaceId,
    plan: ReferencePlan,
    originals: Vec<Original>,
}

impl WorkspaceService {
    pub fn reference_preview(&mut self, from: &RelPath, to: &RelPath) -> Result<ReferencePlan> {
        let open = self.open()?;
        if open.read_only {
            return Err(CoreError::Unsupported {
                cap: "renaming a newer workspace schema".into(),
            });
        }
        if from.is_root() || to.is_root() || to.as_str().starts_with(&format!("{}/", from.as_str()))
        {
            return Err(CoreError::InvalidPath {
                path: to.to_string(),
                reason: "invalid move destination".into(),
            });
        }
        notes_model::portable_name(to.file_name()).map_err(|e| CoreError::InvalidPath {
            path: to.to_string(),
            reason: e.to_string(),
        })?;
        open.fs.stat(from)?;
        if from != to {
            self.check_collision(to)?;
        }
        let status = self.index_status()?;
        if open.content_index.is_none()
            || status.running
            || status.stale
            || status.cancelled
            || status.error.is_some()
        {
            return Err(CoreError::Unsupported {
                cap: "link review requires a complete index; rebuild it first".into(),
            });
        }
        let index = notes_index::Index::open(&open.dir.join("index.db"))?;
        let mut files = Vec::new();
        let mut originals = Vec::new();
        let mut skipped = status.skipped;
        let paths = self
            .walk()?
            .into_iter()
            .filter(RelPath::is_note)
            .collect::<Vec<_>>();
        for (path, doc) in index.documents()? {
            let path = RelPath::parse(&path)?;
            let base = path.parent().unwrap_or_else(RelPath::root);
            let candidate = rewrite::repath(&path, from, to) != path
                || doc.links.iter().any(|l| {
                    if l.kind == notes_markdown::LinkKind::Wiki {
                        let candidates = crate::knowledge::candidates(&paths, &l.target);
                        return candidates
                            .iter()
                            .any(|p| rewrite::repath(p, from, to) != *p);
                    }
                    let end = l.target.find(['#', '?']).unwrap_or(l.target.len());
                    notes_markdown::url::resolve_relative(&base, &l.target[..end])
                        .is_some_and(|p| rewrite::repath(&p, from, to) != p)
                });
            if !candidate {
                continue;
            }
            let stat = open.fs.stat(&path)?;
            let bytes = open.fs.read(&path)?;
            let (profile, text) = TextProfile::detect(&bytes);
            let Some(text) = text else {
                skipped += 1;
                continue;
            };
            if profile.read_only_reason().is_some() {
                skipped += 1;
                continue;
            }
            let mut edits = rewrite::edits(&text, &path, from, to);
            // Resolve wiki names in the core; the parser only identifies source spans.
            for link in notes_markdown::parse(&text)
                .links
                .into_iter()
                .filter(|l| l.kind == notes_markdown::LinkKind::Wiki && !l.in_code)
            {
                let targets = crate::knowledge::candidates(&paths, &link.target);
                if targets.len() != 1 {
                    skipped += 1;
                    continue;
                }
                let next = rewrite::repath(&targets[0], from, to);
                if next == targets[0] {
                    continue;
                }
                let raw = &text[link.span.start..link.span.end];
                let Some(start) = raw.find("[[").map(|i| link.span.start + i + 2) else {
                    continue;
                };
                let Some(end) = text[start..link.span.end]
                    .find(['|', ']'])
                    .map(|i| start + i)
                else {
                    continue;
                };
                if text[start..end] != link.target || next.as_str().contains(['[', ']']) {
                    skipped += 1;
                    continue;
                }
                let fragment = link
                    .target
                    .find('#')
                    .map(|i| &link.target[i..])
                    .unwrap_or("");
                let after = format!("./{next}{fragment}");
                edits.push(LinkEdit {
                    start,
                    end,
                    before: link.target,
                    after,
                });
            }
            edits.sort_by_key(|e| e.start);

            if edits.is_empty() {
                skipped += 1;
                continue;
            }
            let base = BaseRev {
                size: stat.size,
                mtime_ns: stat.mtime_ns,
                hash: notes_fs::hash(&bytes),
            };
            files.push(ReferenceFile {
                path: path.clone(),
                destination: rewrite::repath(&path, from, to),
                edits,
            });
            originals.push(Original {
                bytes,
                base,
                text,
                profile,
            });
        }
        let plan = ReferencePlan {
            token: WorkspaceId::new().to_string(),
            from: from.clone(),
            to: to.clone(),
            files,
            skipped,
        };
        self.reference_plan = Some(Pending {
            workspace: open.id,
            plan: plan.clone(),
            originals,
        });
        Ok(plan)
    }
    pub fn reference_apply(&mut self, token: &str, selected: &[usize]) -> Result<ReferenceResult> {
        let pending = self
            .reference_plan
            .take()
            .ok_or_else(|| CoreError::Unsupported {
                cap: "expired link review".into(),
            })?;
        if pending.plan.token != token || pending.workspace != self.open()?.id {
            return Err(CoreError::Unsupported {
                cap: "expired link review".into(),
            });
        }
        let selected = selected
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        if selected.iter().any(|i| *i >= pending.plan.files.len()) {
            return Err(CoreError::Unsupported {
                cap: "unknown review selection".into(),
            });
        }
        let dir = self.open()?.dir.clone();
        let mut lock = crate::lock::acquire(&crate::paths::lock_file(&dir))?;
        lock.with(|| {
            // Validate every selected base before renaming anything.
            for &i in &selected {
                let path = &pending.plan.files[i].path;
                let bytes = self.open()?.fs.read(path)?;
                if notes_fs::hash(&bytes) != pending.originals[i].base.hash {
                    return Err(CoreError::Unsupported {
                        cap: format!("link review changed on disk: {path}; review again"),
                    });
                }
            }
            let backup = dir.join("reference-backups").join(token);
            std::fs::create_dir_all(&backup)
                .map_err(|e| CoreError::io("backup_links", backup.display(), &e))?;
            for &i in &selected {
                crate::state::write_atomic(
                    &backup.join(format!("{i}.md")),
                    &pending.originals[i].bytes,
                )?;
            }
            let manifest =
                serde_json::to_vec_pretty(&pending.plan).map_err(|e| CoreError::Internal {
                    message: e.to_string(),
                })?;
            crate::state::write_atomic(&backup.join("plan.json"), &manifest)?;
            self.relocate(
                &pending.plan.from,
                &pending.plan.to,
                pending.plan.to.file_name(),
            )?;
            let mut result = ReferenceResult {
                path: pending.plan.to.clone(),
                updated: Vec::new(),
                failed: Vec::new(),
                backup: backup.display().to_string(),
            };
            for &i in &selected {
                let file = &pending.plan.files[i];
                let original = &pending.originals[i];
                let text = rewrite::apply(&original.text, &file.edits);
                let bytes = original.profile.encode(&text);
                let hash = notes_fs::hash(&bytes);
                let open = self.open_mut()?;
                open.arm(&file.destination, hash.clone());
                match open
                    .fs
                    .write_atomic(&file.destination, &bytes, Some(&original.base))
                {
                    Ok(WriteOutcome::Written(stat)) => {
                        open.registry.observe(&file.destination, &stat, hash);
                        open.queue(file.destination.clone());
                        result.updated.push(file.destination.clone());
                    }
                    _ => result.failed.push(file.destination.clone()),
                }
            }
            crate::store_registry(&dir, &self.open()?.registry)?;
            let manifest = serde_json::to_vec_pretty(&result).map_err(|e| CoreError::Internal {
                message: e.to_string(),
            })?;
            crate::state::write_atomic(&backup.join("result.json"), &manifest)?;
            self.invalidate_paths();
            Ok(result)
        })?
    }
}
