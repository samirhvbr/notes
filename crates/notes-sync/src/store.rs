//! Schema-versioned operational state. No source note lives in this directory.
use crate::{Error, Journal, Result};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;
const MAX_STATE_BYTES: u64 = 64 * 1024 * 1024;

pub struct Store {
    dir: PathBuf,
}
impl Store {
    pub fn open(dir: &Path) -> Result<Self> {
        if !dir.is_absolute() {
            return Err(Error::InvalidState);
        }
        fs::create_dir_all(dir).map_err(|_| Error::Storage)?;
        if fs::symlink_metadata(dir)
            .map_err(|_| Error::Storage)?
            .file_type()
            .is_symlink()
        {
            return Err(Error::InvalidState);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
                .map_err(|_| Error::Storage)?;
        }
        Ok(Self {
            dir: dir.to_path_buf(),
        })
    }
    fn lock(&self) -> Result<fd_lock::RwLock<File>> {
        let path = self.dir.join("sync.lock");
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::InvalidState);
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        Ok(fd_lock::RwLock::new(
            options.open(path).map_err(|_| Error::Storage)?,
        ))
    }
    fn load_unlocked(&self) -> Result<(Journal, String)> {
        let path = self.dir.join("journal.json");
        if !fs::symlink_metadata(&path)
            .map_err(|_| Error::Storage)?
            .is_file()
        {
            return Err(Error::InvalidState);
        }
        let mut bytes = vec![];
        File::open(path)
            .map_err(|_| Error::Storage)?
            .take(MAX_STATE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Storage)?;
        if bytes.len() as u64 > MAX_STATE_BYTES {
            return Err(Error::Limit);
        }
        let probe: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| Error::InvalidState)?;
        if probe.get("schema").and_then(|v| v.as_u64()) != Some(crate::SCHEMA as u64) {
            return Err(Error::Schema);
        }
        let journal: Journal = serde_json::from_slice(&bytes).map_err(|_| Error::InvalidState)?;
        journal.validate()?;
        Ok((journal, blake3::hash(&bytes).to_hex().to_string()))
    }
    pub fn load(&self) -> Result<(Journal, String)> {
        let lock = self.lock()?;
        let _guard = lock.try_read().map_err(|_| Error::Busy)?;
        self.load_unlocked()
    }
    pub fn initialize(&self, workspace: Uuid) -> Result<()> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        if self.dir.join("journal.json").exists() {
            return Err(Error::Stale);
        }
        self.persist(&Journal::new(workspace), true)
    }
    /// The caller's digest came from load(). A failed closure or stale digest
    /// leaves the journal unchanged, including receipts and all previous heads.
    pub fn transact<T>(
        &self,
        expected: &str,
        change: impl FnOnce(&mut Journal) -> Result<T>,
    ) -> Result<T> {
        let mut lock = self.lock()?;
        let _guard = lock.try_write().map_err(|_| Error::Busy)?;
        let (mut journal, digest) = self.load_unlocked()?;
        if digest != expected {
            return Err(Error::Stale);
        }
        let output = change(&mut journal)?;
        journal.validate()?;
        self.persist(&journal, false)?;
        Ok(output)
    }
    fn persist(&self, journal: &Journal, exclusive: bool) -> Result<()> {
        journal.validate()?;
        let bytes = serde_json::to_vec(journal).map_err(|_| Error::InvalidState)?;
        if bytes.len() as u64 > MAX_STATE_BYTES {
            return Err(Error::Limit);
        }
        let mut temp = tempfile::NamedTempFile::new_in(&self.dir).map_err(|_| Error::Storage)?;
        temp.write_all(&bytes).map_err(|_| Error::Storage)?;
        temp.as_file().sync_all().map_err(|_| Error::Storage)?;
        let target = self.dir.join("journal.json");
        if exclusive {
            temp.persist_noclobber(target).map_err(|_| Error::Storage)?;
        } else {
            temp.persist(target).map_err(|_| Error::Storage)?;
        }
        #[cfg(unix)]
        File::open(&self.dir)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Storage)?;
        Ok(())
    }
}
