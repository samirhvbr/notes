//! Offline backups include original Markdown, identity state and revoked tokens.
use crate::{admin, Result};
use std::{
    fs::{self, File},
    io::Write,
    path::{Component, Path},
};

pub fn instance_lock(root: &Path) -> Result<fd_lock::RwLock<File>> {
    Ok(fd_lock::RwLock::new(admin::private_file(
        &root.join("server.lock"),
        false,
    )?))
}
fn check_tree(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || (!meta.is_file() && !meta.is_dir()) {
        return Err("backup refuses symlinks and special files".into());
    }
    if meta.is_dir() {
        for entry in fs::read_dir(path)? {
            check_tree(&entry?.path())?;
        }
    }
    Ok(())
}
pub fn backup(root: &Path, output: &Path) -> Result<()> {
    let mut instance = instance_lock(root)?;
    let _instance = instance
        .try_write()
        .map_err(|_| "stop the server before backup")?;
    let mut admin_lock = admin::lock(root)?;
    let _admin = admin_lock.write()?;
    admin::load(root)?;
    check_tree(root)?;
    let parent = output.parent().ok_or("backup needs a parent directory")?;
    if fs::canonicalize(parent)?.starts_with(root) {
        return Err("backup must be outside the data directory".into());
    }
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    {
        let gzip =
            flate2::write::GzEncoder::new(temp.as_file_mut(), flate2::Compression::default());
        let mut archive = tar::Builder::new(gzip);
        archive.follow_symlinks(false);
        archive.append_dir_all("data", root)?;
        let manifest = serde_json::to_vec(&serde_json::json!({"schema":1,"source_root":root}))?;
        let mut header = tar::Header::new_gnu();
        header.set_size(manifest.len() as u64);
        header.set_mode(0o600);
        header.set_cksum();
        archive.append_data(
            &mut header,
            "data/backup-manifest.json",
            manifest.as_slice(),
        )?;
        archive.into_inner()?.finish()?.flush()?;
    }
    temp.as_file().sync_all()?;
    temp.persist_noclobber(output)?;
    admin::audit(
        root,
        "operator",
        "local",
        "backup",
        "ok",
        &uuid::Uuid::new_v4().to_string(),
    )?;
    Ok(())
}
pub fn restore(archive: &Path, destination: &Path) -> Result<()> {
    restore_as(archive, destination, None)
}
pub fn restore_as(archive: &Path, destination: &Path, mounted_root: Option<&Path>) -> Result<()> {
    if mounted_root.is_some_and(|p| {
        !p.is_absolute() || p.components().any(|c| matches!(c, Component::ParentDir))
    }) {
        return Err("final data root must be absolute without traversal".into());
    }
    if destination.exists() {
        return Err("restore requires a new destination".into());
    }
    let parent = destination
        .parent()
        .ok_or("restore needs a parent directory")?;
    let stage = tempfile::tempdir_in(parent)?;
    let gzip = flate2::read::GzDecoder::new(File::open(archive)?);
    let mut tar = tar::Archive::new(gzip);
    let mut count = 0;
    let mut total = 0u64;
    for entry in tar.entries()? {
        let mut entry = entry?;
        count += 1;
        total = total
            .checked_add(entry.size())
            .ok_or("archive size overflow")?;
        if count > 1_000_000 || total > 1024 * 1024 * 1024 * 1024 {
            return Err("archive exceeds restore limits".into());
        }
        let path = entry.path()?.into_owned();
        let mut components = path.components();
        if components.next() != Some(Component::Normal("data".as_ref()))
            || components.any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err("unsafe archive path".into());
        }
        let kind = entry.header().entry_type();
        if !kind.is_file() && !kind.is_dir() {
            return Err("archive links and special files are forbidden".into());
        }
        if !entry.unpack_in(stage.path())? {
            return Err("unsafe archive entry".into());
        }
    }
    let restored = stage.path().join("data");
    check_tree(&restored)?;
    admin::load(&restored)?;
    for required in ["admin", "state", "workspaces", "audit"] {
        if !restored.join(required).is_dir() {
            return Err("incomplete backup".into());
        }
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(restored.join("backup-manifest.json"))?)?;
    if manifest["schema"] != 1 {
        return Err("unsupported backup schema".into());
    }
    let original = Path::new(
        manifest["source_root"]
            .as_str()
            .ok_or("missing backup source")?,
    );
    let final_root =
        fs::canonicalize(parent)?.join(destination.file_name().ok_or("invalid destination")?);
    let final_root = mounted_root.unwrap_or(&final_root);
    notes_core::WorkspaceService::with_data_dir(restored.join("state"))?
        .rebind_restored_workspaces(&original.join("workspaces"), &final_root.join("workspaces"))?;
    fs::remove_file(restored.join("backup-manifest.json"))?;
    // Reserve the destination exclusively; never replace somebody else's data.
    #[cfg(unix)]
    fs::create_dir(destination)?;
    if let Err(error) = fs::rename(&restored, destination) {
        #[cfg(unix)]
        let _ = fs::remove_dir(destination);
        return Err(error.into());
    }
    admin::private_dir(destination)?;
    admin::audit(
        destination,
        "operator",
        "local",
        "restore",
        "ok",
        &uuid::Uuid::new_v4().to_string(),
    )?;
    Ok(())
}
