//! Derived indexing runs independently of the UI service lock. Only LocalFs reads notes.
use crate::{Result, WorkspaceService};
use notes_fs::{FileSystem, LocalFs};
use notes_model::{CoreError, EntryKind, RelPath, TextProfile};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct IndexStatus {
    pub running: bool,
    pub stale: bool,
    pub cancelled: bool,
    pub scanned: usize,
    pub indexed: usize,
    pub unchanged: usize,
    pub skipped: usize,
    pub error: Option<String>,
}

pub(crate) struct Job {
    status: Arc<Mutex<IndexStatus>>,
    cancel: Arc<AtomicBool>,
}
impl Job {
    fn start(root: PathBuf, db: PathBuf, ignores: Vec<String>, hidden: bool, force: bool) -> Self {
        let status = Arc::new(Mutex::new(IndexStatus {
            running: true,
            ..Default::default()
        }));
        let cancel = Arc::new(AtomicBool::new(false));
        let (s, c) = (status.clone(), cancel.clone());
        let spawned = std::thread::Builder::new()
            .name("notes-content-index".into())
            .spawn(move || {
                let result = build(&root, &db, &ignores, hidden, force, &s, &c);
                let mut status = s.lock().unwrap();
                status.running = false;
                status.cancelled = c.load(Ordering::Relaxed);
                if let Err(e) = result {
                    status.error = Some(e.to_string());
                }
            });
        if let Err(e) = spawned {
            let mut s = status.lock().unwrap();
            s.running = false;
            s.error = Some(e.to_string());
        }
        Self { status, cancel }
    }
    fn status(&self) -> IndexStatus {
        self.status.lock().unwrap().clone()
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn build(
    root: &Path,
    db: &Path,
    ignores: &[String],
    hidden: bool,
    force: bool,
    status: &Mutex<IndexStatus>,
    cancel: &AtomicBool,
) -> Result<()> {
    let mut guard = crate::lock::acquire(&db.with_file_name("index.lock"))?;
    guard.with(|| build_locked(root, db, ignores, hidden, force, status, cancel))?
}

fn build_locked(
    root: &Path,
    db: &Path,
    ignores: &[String],
    hidden: bool,
    force: bool,
    status: &Mutex<IndexStatus>,
    cancel: &AtomicBool,
) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        return Ok(());
    }
    let fs = LocalFs::open(root)?;
    let mut index = match notes_index::Index::open(db) {
        Ok(index) => index,
        Err(CoreError::SchemaAhead {
            store,
            found,
            supported,
        }) => {
            return Err(CoreError::SchemaAhead {
                store,
                found,
                supported,
            })
        }
        Err(_) if force => {
            let suffix = notes_model::WorkspaceId::new().to_string();
            for name in ["index.db", "index.db-wal", "index.db-shm"] {
                let path = db.with_file_name(name);
                if path.exists() {
                    std::fs::rename(&path, db.with_file_name(format!("{name}.damaged-{suffix}")))
                        .map_err(|e| CoreError::io("quarantine_index", path.display(), &e))?;
                }
            }
            notes_index::Index::open(db)?
        }
        Err(e) => return Err(e),
    };
    let old = index.plan()?;
    let mut seen = BTreeSet::new();
    let mut dirs = vec![RelPath::root()];
    let mut complete = true;
    while let Some(dir) = dirs.pop() {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let entries = match fs.list(&dir) {
            Ok(v) => v,
            Err(_) => {
                status.lock().unwrap().skipped += 1;
                complete = false;
                continue;
            }
        };
        for entry in entries {
            if cancel.load(Ordering::Relaxed) {
                return Ok(());
            }
            if crate::ignore::is_hidden_name(&entry.name)
                && (!hidden || crate::ignore::IGNORE_DEFAULT.contains(&entry.name.as_str()))
            {
                continue;
            }
            if ignores
                .iter()
                .any(|i| i == &entry.name || i == entry.path.as_str())
            {
                continue;
            }
            if entry.kind == EntryKind::Dir {
                dirs.push(entry.path);
                continue;
            }
            if entry.kind != EntryKind::File || !entry.is_note {
                continue;
            }
            let path = entry.path;
            seen.insert(path.to_string());
            status.lock().unwrap().scanned += 1;
            let stat = match fs.stat(&path) {
                Ok(s) => s,
                Err(_) => {
                    status.lock().unwrap().skipped += 1;
                    continue;
                }
            };
            let current = notes_index::Seen {
                path: path.to_string(),
                size: stat.size,
                mtime: stat.mtime_ns.to_string(),
            };
            let cached = old.get(path.as_str());
            if !force && cached.is_some_and(|p| p.size == stat.size && p.mtime == current.mtime) {
                status.lock().unwrap().unchanged += 1;
                continue;
            }
            if stat.size > 8 * 1024 * 1024 {
                index.remove(&[path.to_string()])?;
                status.lock().unwrap().skipped += 1;
                continue;
            }
            let bytes = match fs.read(&path) {
                Ok(b) => b,
                Err(_) => {
                    index.remove(&[path.to_string()])?;
                    status.lock().unwrap().skipped += 1;
                    continue;
                }
            };
            // A raced write is retried on the next pass; never label it current.
            let after = fs.stat(&path)?;
            if after.size != stat.size || after.mtime_ns != stat.mtime_ns {
                status.lock().unwrap().skipped += 1;
                continue;
            }
            let hash = serde_json::to_string(&notes_fs::hash(&bytes)).map_err(|e| {
                CoreError::Internal {
                    message: e.to_string(),
                }
            })?;
            let (_, text) = TextProfile::detect(&bytes);
            let Some(text) = text else {
                index.remove(&[path.to_string()])?;
                status.lock().unwrap().skipped += 1;
                continue;
            };
            let changed = force || cached.is_none_or(|p| p.hash != hash);
            index.apply(
                notes_index::Indexed {
                    seen: &current,
                    hash: &hash,
                    text: &text,
                },
                changed,
            )?;
            status.lock().unwrap().indexed += 1;
        }
    }
    if complete && !cancel.load(Ordering::Relaxed) {
        let gone = old
            .keys()
            .filter(|p| !seen.contains(*p))
            .cloned()
            .collect::<Vec<_>>();
        index.remove(&gone)?;
    }
    Ok(())
}

impl WorkspaceService {
    pub fn index_start(&mut self, force: bool) -> Result<IndexStatus> {
        let open = self.open_mut()?;
        if !force
            && open
                .content_index
                .as_ref()
                .is_some_and(|j| j.status().running)
        {
            return self.index_status();
        }
        if open.read_only {
            return Err(CoreError::Unsupported {
                cap: "indexing a newer workspace schema".into(),
            });
        }
        // Dropping the prior job requests cancellation before replacing it.
        open.content_index = None;
        open.content_dirty.store(false, Ordering::Relaxed);
        open.content_index = Some(Job::start(
            open.fs.root().to_path_buf(),
            open.dir.join("index.db"),
            open.extra_ignore.clone(),
            false,
            force,
        ));
        self.index_status()
    }
    pub fn index_status(&self) -> Result<IndexStatus> {
        let open = self.open()?;
        let mut status = open
            .content_index
            .as_ref()
            .map(Job::status)
            .unwrap_or_default();
        status.stale = open.content_dirty.load(Ordering::Relaxed);
        Ok(status)
    }
    pub fn index_cancel(&mut self) -> Result<()> {
        if let Some(j) = self.open()?.content_index.as_ref() {
            j.cancel.store(true, Ordering::Relaxed);
        }
        Ok(())
    }
    pub fn word_hits(&self, query: &str) -> Result<(Vec<crate::search::SearchHit>, bool)> {
        let open = self.open()?;
        let path = open.dir.join("index.db");
        if !path.exists() {
            return Err(CoreError::Unsupported {
                cap: "word search before indexing; rebuild the index".into(),
            });
        }
        let status = self.index_status()?;
        if status.error.is_some() {
            return Err(CoreError::Unsupported {
                cap: "word search while the index is unavailable; rebuild the index".into(),
            });
        }
        let index = notes_index::Index::open(&path)?;
        let hits = index
            .words(query, 2001)?
            .into_iter()
            .filter_map(|h| {
                let path = RelPath::parse(&h.path).ok()?;
                open.fs.stat(&path).ok()?;
                Some(crate::search::SearchHit {
                    path,
                    line: h.line,
                    col: h.col,
                    context: h.context,
                })
            })
            .collect();
        Ok((
            hits,
            status.running || status.stale || status.cancelled || status.skipped > 0,
        ))
    }
}
