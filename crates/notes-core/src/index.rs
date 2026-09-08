//! The quick-open path list, built off the critical path.
//!
//! Quick open matches paths from memory (ADR-032). Building that list means
//! walking the workspace, and on a folder of 21 000 directories that walk took
//! **550 ms** — inside the command, holding the service mutex, so the tree could
//! not be listed while it ran. On the owner's `~/x` it was minutes
//! (`docs/DECISIONS-0.1c.md` D-09).
//!
//! So it runs on its own thread and the caller matches whatever exists so far.
//! A partial list with `building: true` beside it is a better answer than a
//! frozen window, and it is an honest one — the interface says the index is
//! still filling.
//!
//! It also **never fails on a directory it cannot read**. The old walk
//! propagated the first `read_dir` error, so one unreadable subdirectory made
//! quick open return nothing for the entire workspace. Those are counted and
//! skipped, exactly as the watcher counts and skips them.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use notes_model::RelPath;

use crate::ignore::is_hidden_name;

/// What the index holds right now.
pub struct Snapshot {
    pub paths: Vec<RelPath>,
    pub building: bool,
    /// Directories skipped because they could not be read.
    pub unreadable: usize,
}

pub struct PathIndex {
    paths: Arc<Mutex<Vec<RelPath>>>,
    building: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    unreadable: Arc<AtomicUsize>,
}

/// How many paths are appended before the shared list is touched. A lock per
/// file would make the walk contend with every keystroke that reads it.
const BATCH: usize = 256;

impl PathIndex {
    /// Start walking `root` in the background.
    pub fn start(root: &Path, extra_ignore: Vec<String>, show_hidden: bool) -> Self {
        let index = Self {
            paths: Arc::new(Mutex::new(Vec::new())),
            building: Arc::new(AtomicBool::new(true)),
            cancel: Arc::new(AtomicBool::new(false)),
            unreadable: Arc::new(AtomicUsize::new(0)),
        };

        let root = root.to_path_buf();
        let paths = Arc::clone(&index.paths);
        let building = Arc::clone(&index.building);
        let cancel = Arc::clone(&index.cancel);
        let unreadable = Arc::clone(&index.unreadable);

        std::thread::Builder::new()
            .name("notes-index".into())
            .spawn(move || {
                walk(
                    &root,
                    &extra_ignore,
                    show_hidden,
                    &paths,
                    &cancel,
                    &unreadable,
                );
                if !cancel.load(Ordering::Relaxed) {
                    // Sorted once, at the end: a list that reorders itself under
                    // the user as it fills is worse than one that fills.
                    paths.lock().unwrap().sort();
                }
                building.store(false, Ordering::Release);
            })
            .ok();

        index
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            paths: self.paths.lock().unwrap().clone(),
            building: self.building.load(Ordering::Acquire),
            unreadable: self.unreadable.load(Ordering::Relaxed),
        }
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for PathIndex {
    /// Dropping the index stops its walk. Without this, changing workspace or
    /// invalidating the list would leave a thread walking a folder nobody is
    /// going to ask about.
    fn drop(&mut self) {
        self.cancel();
    }
}

fn walk(
    root: &Path,
    extra_ignore: &[String],
    show_hidden: bool,
    out: &Arc<Mutex<Vec<RelPath>>>,
    cancel: &Arc<AtomicBool>,
    unreadable: &Arc<AtomicUsize>,
) {
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    let mut batch: Vec<RelPath> = Vec::with_capacity(BATCH);

    while let Some(dir) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => {
                unreadable.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if hidden(&name, show_hidden, extra_ignore) {
                continue;
            }
            // `symlink_metadata`: a symlinked directory is not descended into,
            // so a loop cannot be entered, and a symlink is not a note.
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                let Ok(rel) = path.strip_prefix(root) else {
                    continue;
                };
                let rel = rel.to_string_lossy().replace('\\', "/");
                if let Ok(rel) = RelPath::parse(&rel) {
                    if rel.is_note() {
                        batch.push(rel);
                    }
                }
            }
        }
        if batch.len() >= BATCH {
            out.lock().unwrap().append(&mut batch);
        }
    }
    if !batch.is_empty() {
        out.lock().unwrap().append(&mut batch);
    }
}

/// The visibility rule, by name — the same one `ignore::is_hidden` applies to an
/// `Entry`, without needing one.
fn hidden(name: &str, show_hidden: bool, extra: &[String]) -> bool {
    if is_hidden_name(name) {
        return !show_hidden
            || crate::ignore::IGNORE_DEFAULT.contains(&name)
            || name.ends_with(".tmp");
    }
    extra.iter().any(|e| e == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn settle(index: &PathIndex) -> Snapshot {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let s = index.snapshot();
            if !s.building || Instant::now() > deadline {
                return s;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn it_finds_notes_and_ignores_everything_else() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.md"), b"").unwrap();
        std::fs::write(d.path().join("b.txt"), b"").unwrap();
        std::fs::create_dir_all(d.path().join("sub")).unwrap();
        std::fs::write(d.path().join("sub/c.markdown"), b"").unwrap();
        std::fs::create_dir_all(d.path().join(".git")).unwrap();
        std::fs::write(d.path().join(".git/d.md"), b"").unwrap();

        let s = settle(&PathIndex::start(d.path(), vec![], false));
        let names: Vec<_> = s.paths.iter().map(|p| p.as_str()).collect();
        assert_eq!(names, vec!["a.md", "sub/c.markdown"]);
        assert!(!s.building);
    }

    /// The bug this module exists to remove: one unreadable directory used to
    /// make quick open return nothing at all.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_directory_is_counted_and_skipped_not_fatal() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("visivel.md"), b"").unwrap();
        let denied = d.path().join("negado");
        std::fs::create_dir(&denied).unwrap();
        std::fs::write(denied.join("escondida.md"), b"").unwrap();
        std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o000)).unwrap();

        let s = settle(&PathIndex::start(d.path(), vec![], false));
        std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert!(
            s.paths.iter().any(|p| p.as_str() == "visivel.md"),
            "the rest of the workspace is still indexed: {:?}",
            s.paths
        );
        assert_eq!(
            s.unreadable, 1,
            "and the directory is counted, not swallowed"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_loop_does_not_hang_the_walk() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.md"), b"").unwrap();
        let loopy = d.path().join("loop");
        std::fs::create_dir(&loopy).unwrap();
        std::os::unix::fs::symlink(d.path(), loopy.join("up")).unwrap();

        let s = settle(&PathIndex::start(d.path(), vec![], false));
        assert!(!s.building, "the walk returned");
        assert_eq!(s.paths.len(), 1);
    }

    #[test]
    fn a_cancelled_walk_stops() {
        let d = tempfile::tempdir().unwrap();
        for i in 0..200 {
            std::fs::write(d.path().join(format!("n{i}.md")), b"").unwrap();
        }
        let index = PathIndex::start(d.path(), vec![], false);
        index.cancel();
        let s = settle(&index);
        assert!(!s.building);
    }
}
