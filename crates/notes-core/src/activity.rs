//! A shared lease for open workspaces; offline sync application takes exclusive.
use notes_model::CoreError;
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
};
pub fn acquire(data: &Path, root: &str, exclusive: bool) -> Result<File, CoreError> {
    let dir = data.join("active");
    fs::create_dir_all(&dir).map_err(|e| CoreError::io("activity", "state", &e))?;
    if !fs::symlink_metadata(&dir)
        .map_err(|e| CoreError::io("activity", "state", &e))?
        .is_dir()
    {
        return Err(CoreError::Unsupported {
            cap: "invalid activity directory".into(),
        });
    }
    let path = dir.join(format!(
        "{}.lock",
        notes_fs::hash(root.as_bytes())
            .to_string()
            .trim_start_matches("b3:")
    ));
    if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file()) {
        return Err(CoreError::Unsupported {
            cap: "invalid activity lock".into(),
        });
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(&path)
        .map_err(|e| CoreError::io("activity", "state", &e))?;
    if exclusive {
        file.try_lock().map_err(|_| CoreError::LockTimeout)?;
    } else {
        file.try_lock_shared().map_err(|_| CoreError::LockTimeout)?;
    }
    Ok(file)
}
