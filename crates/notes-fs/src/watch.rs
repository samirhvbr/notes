//! The watcher, and the 200 ms debouncer `ARCHITECTURE.md` §8 puts in front of
//! it.
//!
//! **The debouncer is written here rather than pulled in.** It is forty lines,
//! and it feeds a self-write filter that is ours anyway — one editor's save is
//! a burst of create/modify/rename events on every platform, and collapsing
//! them to *a set of paths that may have changed* is the entire job. What the
//! core then does with that set is `notes-core`'s reconciliation, which is
//! where every decision lives.
//!
//! This module never decides anything about a note. It answers one question —
//! *which paths moved* — and it is allowed to be wrong in the direction of
//! reporting too many: the reconciler stats and hashes before it believes
//! anything.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use notes_model::{CoreError, RelPath};
use notify::{RecursiveMode, Watcher as _};

/// `ARCHITECTURE.md` §8. Long enough to collapse an editor's save, short enough
/// that the acceptance criterion — *"editar no VS Code com o app aberto atualiza
/// a aba em <1s"* — has most of its second left over.
const DEBOUNCE: Duration = Duration::from_millis(200);

/// Directories the watcher does not descend into.
///
/// **This is not the visibility list.** `notes-core`'s `IGNORE_DEFAULT` decides
/// what the user *sees*, and `node_modules/` and `target/` are deliberately not
/// in it — they hold real Markdown, and hiding a folder by name is the
/// application deciding which of the user's files are real
/// (`docs/DECISIONS-0.1c.md` D-08).
///
/// Watching is a different question with a different currency: an inotify watch
/// is a finite kernel resource, one per directory, and a machine-generated tree
/// can hold hundreds of thousands of them. A change inside one still reaches the
/// application through the five-second poll and the focus scan — the same
/// degradation any unwatched path has — so the cost of skipping is latency, and
/// the cost of not skipping is the watch table.
const WATCH_SKIP: &[&str] = &[
    "node_modules",
    "target",
    "vendor",
    "dist",
    "build",
    ".git",
    ".svn",
    ".hg",
    ".cache",
    "__pycache__",
];

/// What the watcher has managed so far. Read while it is still walking.
#[derive(Debug, Default)]
pub struct WatchCounters {
    pub walking: AtomicBool,
    pub dirs: AtomicUsize,
    /// Directories that could not be read or watched — a permission, a mount
    /// that vanished. Counted and skipped; never a reason to stop.
    pub unreadable: AtomicUsize,
    /// Directories left unwatched because the platform's watch table is full.
    pub over_limit: AtomicUsize,
}

/// A snapshot of the above, for a caller that has to render it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchProgress {
    pub walking: bool,
    pub dirs: usize,
    pub unreadable: usize,
    pub over_limit: usize,
}

/// Why a workspace is not being watched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Degraded {
    /// The backend has no watch at all: a network mount, a SAF tree `[0.4]`.
    Unsupported(String),
    /// Linux ran out of inotify watches. **The `sysctl` is in the message**
    /// because it is the one thing the user can do about it
    /// (`ARCHITECTURE.md` §8).
    WatchLimit(String),
}

/// A running watch on one workspace root.
///
/// Dropping it stops the thread and releases the platform handle.
pub struct Watch {
    rx: Receiver<BTreeSet<RelPath>>,
    stop: Sender<()>,
    /// Set when **no** watch could be established at all. A directory that
    /// cannot be watched is not this: it is counted in `counters.unreadable`
    /// and the rest of the workspace is watched normally.
    pub degraded: Option<Degraded>,
    counters: Arc<WatchCounters>,
}

impl Watch {
    /// A watch that was never established. The caller polls; nothing breaks.
    pub fn none(reason: Degraded) -> Self {
        let (_tx, rx) = channel();
        let (stop, _) = channel();
        Self {
            rx,
            stop,
            degraded: Some(reason),
            counters: Arc::new(WatchCounters::default()),
        }
    }

    /// What the walk has managed so far. Safe to call while it is still running.
    pub fn progress(&self) -> WatchProgress {
        WatchProgress {
            walking: self.counters.walking.load(Ordering::Relaxed),
            dirs: self.counters.dirs.load(Ordering::Relaxed),
            unreadable: self.counters.unreadable.load(Ordering::Relaxed),
            over_limit: self.counters.over_limit.load(Ordering::Relaxed),
        }
    }

    /// Every path that has moved since the last call, or an empty set.
    ///
    /// Never blocks: reconciliation is driven by the caller's own tick, and a
    /// watcher that could stall the UI thread would be worse than no watcher.
    pub fn drain(&self) -> BTreeSet<RelPath> {
        let mut out = BTreeSet::new();
        while let Ok(batch) = self.rx.try_recv() {
            out.extend(batch);
        }
        out
    }

    /// Block until something changes or the timeout expires. For tests, and for
    /// a caller that has nothing else to do.
    pub fn wait(&self, timeout: Duration) -> BTreeSet<RelPath> {
        match self.rx.recv_timeout(timeout) {
            Ok(mut batch) => {
                batch.extend(self.drain());
                batch
            }
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => BTreeSet::new(),
        }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}

/// Start watching `root`.
///
/// **Returns immediately.** The walk that installs one watch per directory
/// happens on the watcher's own thread, and `progress()` reports it while it
/// runs. That ordering is the whole point: `notify`'s `RecursiveMode::Recursive`
/// does the same walk *inside the call*, and on a folder of 21 000 directories
/// that call took **503 ms** — with the service mutex held, so the tree could not
/// be listed until it finished. On a folder of a few hundred thousand it is
/// minutes, which is what the owner saw (`docs/DECISIONS-0.1c.md` D-09).
///
/// The walk is ours rather than `notify`'s for the second reason too: a
/// recursive add fails **whole** on the first directory it cannot read, and one
/// unreadable subdirectory then demotes an entire workspace to polling. Here a
/// directory that cannot be read or watched is counted and skipped.
///
/// Symlinked directories are never descended into — the fixture has a loop, and
/// a walk that follows one does not return.
pub fn watch(root: &Path) -> Watch {
    let (batches, rx) = channel::<BTreeSet<RelPath>>();
    let (stop, stopped) = channel::<()>();
    let (raw_tx, raw_rx) = channel::<notify::Result<notify::Event>>();
    let counters = Arc::new(WatchCounters::default());

    let mut watcher = match notify::recommended_watcher(move |res| {
        // A send failure means the debouncer thread is gone, which happens
        // only while shutting down.
        let _ = raw_tx.send(res);
    }) {
        Ok(w) => w,
        Err(e) => return Watch::none(classify(&e)),
    };

    // The root itself, synchronously: if even this cannot be watched there is
    // nothing to watch, and the caller should poll.
    if let Err(e) = watcher.watch(root, RecursiveMode::NonRecursive) {
        return Watch::none(classify(&e));
    }
    counters.dirs.store(1, Ordering::Relaxed);
    counters.walking.store(true, Ordering::Relaxed);

    let root_buf = root.to_path_buf();
    let walk_counters = Arc::clone(&counters);
    std::thread::Builder::new()
        .name("notes-watch".into())
        .spawn(move || {
            // The watcher is moved into the thread so it lives exactly as long
            // as the loop does.
            let mut watcher = watcher;
            add_watches_below(&mut watcher, &root_buf, &walk_counters);
            walk_counters.walking.store(false, Ordering::Relaxed);

            let mut pending: BTreeSet<RelPath> = BTreeSet::new();
            loop {
                if stopped.try_recv().is_ok() {
                    return;
                }
                match raw_rx.recv_timeout(DEBOUNCE) {
                    Ok(Ok(event)) => {
                        for p in event.paths {
                            // A directory that appears after the walk needs its
                            // own watch, or nothing inside it is ever seen.
                            if p.is_dir() && !skip_dir(&p) {
                                let _ = watcher.watch(&p, RecursiveMode::NonRecursive);
                                walk_counters.dirs.fetch_add(1, Ordering::Relaxed);
                            }
                            if let Some(rel) = relativise(&root_buf, &p) {
                                pending.insert(rel);
                            }
                        }
                    }
                    // A dropped event is a reason to look at everything, and the
                    // reconciler's full scan is what the caller falls back to;
                    // reporting the root says exactly that.
                    Ok(Err(_)) => {
                        pending.insert(RelPath::root());
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        if !pending.is_empty()
                            && batches.send(std::mem::take(&mut pending)).is_err()
                        {
                            return;
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        })
        .ok();

    Watch {
        rx,
        stop,
        degraded: None,
        counters,
    }
}

/// Walk `root` and install one non-recursive watch per directory.
///
/// Errors are per directory and never stop the walk. A watch-table exhaustion
/// stops *adding* — there is nothing to be gained by asking again for every
/// remaining directory — but everything already watched keeps working, which is
/// the difference between a degraded workspace and a dead one.
fn add_watches_below(
    watcher: &mut notify::RecommendedWatcher,
    root: &Path,
    counters: &Arc<WatchCounters>,
) {
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    let mut table_full = false;

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => {
                counters.unreadable.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            // `symlink_metadata`, so a symlinked directory is not descended
            // into and a loop cannot be entered.
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if !meta.is_dir() || skip_dir(&path) {
                continue;
            }
            if table_full {
                counters.over_limit.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            match watcher.watch(&path, RecursiveMode::NonRecursive) {
                Ok(()) => {
                    counters.dirs.fetch_add(1, Ordering::Relaxed);
                    stack.push(path);
                }
                Err(e) if is_watch_limit(&e) => {
                    table_full = true;
                    counters.over_limit.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    counters.unreadable.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
}

fn skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| WATCH_SKIP.contains(&n))
        .unwrap_or(true)
}

fn is_watch_limit(e: &notify::Error) -> bool {
    matches!(classify(e), Degraded::WatchLimit(_))
}

fn relativise(root: &Path, path: &Path) -> Option<RelPath> {
    let rest = path.strip_prefix(root).ok()?;
    if rest.as_os_str().is_empty() {
        return Some(RelPath::root());
    }
    let mut parts = Vec::new();
    for seg in rest.iter() {
        let s = seg.to_str()?;
        if s.starts_with('.') && s.ends_with(".tmp") {
            return None;
        }
        parts.push(s);
    }
    RelPath::parse(&parts.join("/")).ok()
}

fn classify(e: &notify::Error) -> Degraded {
    // `notify` reports the inotify limit as an ordinary I/O error; the errno is
    // the only thing that distinguishes "this kernel has run out of watches"
    // from "this path does not exist", and they need different sentences.
    if let notify::ErrorKind::Io(io) = &e.kind {
        if io.raw_os_error() == Some(28) {
            return Degraded::WatchLimit(
                "the kernel's inotify watch limit was reached; raise it with \
                 `sysctl fs.inotify.max_user_watches=524288`"
                    .into(),
            );
        }
    }
    Degraded::Unsupported(e.to_string())
}

impl From<Degraded> for CoreError {
    fn from(d: Degraded) -> Self {
        CoreError::Unsupported {
            cap: match d {
                Degraded::Unsupported(m) => format!("watch: {m}"),
                Degraded::WatchLimit(m) => format!("watch: {m}"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn our_own_temporary_files_never_reach_the_reconciler() {
        let root = Path::new("/w");
        assert_eq!(
            relativise(root, Path::new("/w/nota.md")),
            Some(RelPath::parse("nota.md").unwrap())
        );
        assert_eq!(relativise(root, Path::new("/w/.nota.md.tmp")), None);
        assert_eq!(relativise(root, Path::new("/w/sub/.a.md.tmp")), None);
        // A file that merely *contains* the word is not one of ours.
        assert!(relativise(root, Path::new("/w/tmp.md")).is_some());
    }

    #[test]
    fn a_path_outside_the_root_is_not_relativised() {
        assert_eq!(relativise(Path::new("/w"), Path::new("/other/x.md")), None);
    }

    #[test]
    fn the_root_itself_relativises_to_the_root() {
        assert_eq!(
            relativise(Path::new("/w"), Path::new("/w")),
            Some(RelPath::root())
        );
    }
}
