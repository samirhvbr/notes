use std::path::Path;
use std::time::{Duration, Instant};

use notes_model::CoreError;

/// The cross-process write lock (`ARCHITECTURE.md` §6).
///
/// One per workspace, guarding the *stat → compare → replace* sequence and the
/// registry update — hold time is milliseconds. It coordinates **our** processes
/// (the app and, from 0.3, `notes-mcp`); a third-party editor does not take it,
/// and its writes are caught by the base-rev check instead.
///
/// There is no stale-lock problem by construction: an advisory lock is released
/// by the kernel when its holder dies. That is why it is a lock and not a pid
/// file.
///
/// It exists at 0.1a, when there is only one process, so that the protocol is
/// exercised by tests before a second one arrives.
pub struct WriteLock {
    file: fd_lock::RwLock<std::fs::File>,
}

const TIMEOUT: Duration = Duration::from_secs(5);
const RETRY: Duration = Duration::from_millis(20);

/// Take the lock, blocking up to five seconds.
///
/// Returns a guard-carrying value; dropping it releases. `LockTimeout` is
/// handled as a write failure — a draft is written and the UI shows the error
/// (§5 step 9).
pub fn acquire(path: &Path) -> Result<WriteLock, CoreError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| CoreError::io("mkdir", dir.display(), &e))?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
        .map_err(|e| CoreError::io("open_lock", path.display(), &e))?;

    // Opening the file takes no lock. The lock itself is taken inside `with`,
    // for exactly the length of the critical section — a lock held from
    // `acquire` to drop would serialise a whole command instead of the
    // stat → compare → replace sequence it exists to guard.
    Ok(WriteLock { file: fd_lock::RwLock::new(file) })
}

impl WriteLock {
    /// Run `f` while holding the lock for writing.
    pub fn with<T>(&mut self, f: impl FnOnce() -> T) -> Result<T, CoreError> {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            match self.file.try_write() {
                Ok(_guard) => return Ok(f()),
                Err(_) if Instant::now() < deadline => std::thread::sleep(RETRY),
                Err(_) => return Err(CoreError::LockTimeout),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lock_file_is_created_and_the_section_runs() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("sub/write.lock");
        let mut l = acquire(&p).unwrap();
        assert!(p.exists());
        let out = l.with(|| 42).unwrap();
        assert_eq!(out, 42);
    }

    #[test]
    fn a_second_acquisition_in_the_same_process_still_works() {
        // Advisory locks are per-file-descriptor: two handles in one process do
        // not deadlock, and the real contention case is two processes, which is
        // covered by tools/crash-save-loop from 0.3.
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("write.lock");
        let _a = acquire(&p).unwrap();
        let _b = acquire(&p).unwrap();
    }
}
