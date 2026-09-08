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
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use notes_model::{CoreError, RelPath};
use notify::{RecursiveMode, Watcher as _};

/// `ARCHITECTURE.md` §8. Long enough to collapse an editor's save, short enough
/// that the acceptance criterion — *"editar no VS Code com o app aberto atualiza
/// a aba em <1s"* — has most of its second left over.
const DEBOUNCE: Duration = Duration::from_millis(200);

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
    /// Set when the watch could not be established. The caller is expected to
    /// poll instead, and the UI to say why.
    pub degraded: Option<Degraded>,
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

/// Start watching `root`, recursively.
///
/// Returns a [`Watch`] whose `degraded` field is set rather than an `Err` when
/// the platform cannot watch: *not being able to watch is a state of the
/// workspace*, not a failure of the call, and the application keeps working by
/// polling.
pub fn watch(root: &Path) -> Watch {
    let (batches, rx) = channel::<BTreeSet<RelPath>>();
    let (stop, stopped) = channel::<()>();
    let (raw_tx, raw_rx) = channel::<notify::Result<notify::Event>>();

    let mut watcher = match notify::recommended_watcher(move |res| {
        // A send failure means the debouncer thread is gone, which happens
        // only while shutting down.
        let _ = raw_tx.send(res);
    }) {
        Ok(w) => w,
        Err(e) => return Watch::none(classify(&e)),
    };
    if let Err(e) = watcher.watch(root, RecursiveMode::Recursive) {
        return Watch::none(classify(&e));
    }

    let root = root.to_path_buf();
    std::thread::Builder::new()
        .name("notes-watch".into())
        .spawn(move || {
            // The watcher is moved into the thread so it lives exactly as long
            // as the loop does.
            let _watcher = watcher;
            let mut pending: BTreeSet<RelPath> = BTreeSet::new();
            loop {
                if stopped.try_recv().is_ok() {
                    return;
                }
                match raw_rx.recv_timeout(DEBOUNCE) {
                    Ok(Ok(event)) => {
                        for p in event.paths {
                            if let Some(rel) = relativise(&root, &p) {
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
    }
}

/// Turn an absolute path from the platform into a workspace-relative one.
///
/// **Our own temporary files are dropped here** (`ARCHITECTURE.md` §8): every
/// atomic save creates and renames a `.<name>.tmp`, and reporting those would
/// make the application watch itself work.
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
