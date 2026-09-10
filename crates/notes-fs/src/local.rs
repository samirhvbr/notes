use crate::{hash, DeleteOutcome, FileSystem, Result, WriteOutcome};
use notes_model::{BaseRev, Caps, CoreError, Entry, EntryKind, NativeId, RelPath, Stat};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// The local filesystem, jailed to one root.
pub struct LocalFs {
    root: PathBuf,
    caps: Caps,
}

impl LocalFs {
    /// Adopt `root`. Canonicalises once, and **creates nothing** — scope §2.3
    /// and a 0.1a acceptance criterion both require that opening a folder leaves
    /// it untouched.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        let canonical = root
            .canonicalize()
            .map_err(|e| CoreError::io("canonicalize", root.display(), &e))?;
        if !canonical.is_dir() {
            return Err(CoreError::Unavailable {
                root: canonical.display().to_string(),
                reason: notes_model::UnavailableReason::NotADirectory,
            });
        }
        Ok(Self {
            root: canonical,
            caps: Caps::LOCAL,
        })
    }

    pub fn with_caps(mut self, caps: Caps) -> Self {
        self.caps = caps;
        self
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a relative path **and prove it stays inside the root**.
    ///
    /// [`RelPath`] has already refused `..`, absolute paths and backslashes;
    /// this is the half it cannot do, because it is about the disk rather than
    /// the string. Each segment is `symlink_metadata`'d as it is appended: a
    /// symlink anywhere along the way is refused rather than followed, which is
    /// both scope §7.6 and the only way a well-formed relative path can leave
    /// the root.
    pub fn resolve(&self, rel: &RelPath) -> Result<PathBuf> {
        let mut p = self.root.clone();
        if rel.is_root() {
            return Ok(p);
        }
        for seg in rel.as_str().split('/') {
            p.push(seg);
            if let Ok(meta) = fs::symlink_metadata(&p) {
                if meta.file_type().is_symlink() {
                    return Err(CoreError::SymlinkNotFollowed {
                        path: rel.to_string(),
                    });
                }
            }
        }
        // Belt and braces: RelPath cannot express an escape, so reaching this
        // is a bug in RelPath rather than a hostile input — but the cost of the
        // check is a string comparison.
        if !p.starts_with(&self.root) {
            return Err(CoreError::OutsideRoot {
                path: rel.to_string(),
            });
        }
        Ok(p)
    }

    /// A complete `Stat`, native id included.
    fn stat_at(path: &Path) -> Result<Stat> {
        let meta =
            fs::symlink_metadata(path).map_err(|e| CoreError::io("stat", path.display(), &e))?;
        Ok(Stat {
            native_id: native_id(path, &meta),
            ..Self::stat_from(&meta)
        })
    }

    /// Size, mtime and kind — everything except the native id.
    ///
    /// The identity is left out because it is not free: on Windows it costs an
    /// opened handle per file, and the only caller that wants it is identity
    /// correlation, one path at a time. `Entry` does not carry one at all, so
    /// paying for it while listing a directory would be paying for nothing.
    fn stat_from(meta: &fs::Metadata) -> Stat {
        let ft = meta.file_type();
        let kind = if ft.is_symlink() {
            EntryKind::Symlink
        } else if ft.is_dir() {
            EntryKind::Dir
        } else if ft.is_file() {
            EntryKind::File
        } else {
            EntryKind::Other
        };
        Stat {
            size: meta.len(),
            mtime_ns: mtime_ns(meta),
            native_id: None,
            kind,
        }
    }

    fn tmp_path(target: &Path) -> Result<PathBuf> {
        let dir = target.parent().ok_or_else(|| CoreError::Internal {
            message: "target has no parent directory".into(),
        })?;
        let name =
            target
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| CoreError::Internal {
                    message: "target has no file name".into(),
                })?;
        // Same directory, therefore the same volume, therefore the rename is
        // atomic and `same_volume_move` holds.
        //
        // **The name is deterministic, and that is a deliberate change from a
        // random one.** A `SIGKILL` between the write and the rename leaves the
        // temporary file behind — no process can clean up after being killed —
        // and with a random name every crash leaves a *new* one, so they
        // accumulate in the user's folder forever. With one name per note, a
        // crash leaves at most one, and the next save of that note overwrites
        // it. `tools/crash-save-loop.sh` asserts exactly that bound.
        //
        // Two of our processes writing the same note are serialised by
        // `write.lock`, so the shared name cannot collide; a third-party editor
        // does not use our naming.
        let _ = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        Ok(dir.join(format!(".{name}.tmp")))
    }
}

impl FileSystem for LocalFs {
    fn caps(&self) -> Caps {
        self.caps
    }

    fn list(&self, dir: &RelPath) -> Result<Vec<Entry>> {
        let abs = self.resolve(dir)?;
        let mut out = Vec::new();
        let rd = fs::read_dir(&abs).map_err(|e| CoreError::io("read_dir", abs.display(), &e))?;
        for entry in rd {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue, // a file that vanished mid-listing is not an error
            };
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(path) = dir.join(&name) else { continue };
            let Ok(meta) = entry
                .metadata()
                .or_else(|_| fs::symlink_metadata(entry.path()))
            else {
                continue;
            };
            let stat = Self::stat_from(&meta);
            out.push(Entry {
                is_note: stat.kind == EntryKind::File && path.is_note(),
                size: (stat.kind == EntryKind::File).then_some(stat.size),
                kind: stat.kind,
                name,
                path,
            });
        }
        // Directories first, then names, case-insensitively — a stable order so
        // the tree does not reshuffle between listings.
        out.sort_by(|a, b| {
            let da = a.kind != EntryKind::Dir;
            let db = b.kind != EntryKind::Dir;
            da.cmp(&db)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(out)
    }

    fn read(&self, path: &RelPath) -> Result<Vec<u8>> {
        let abs = self.resolve(path)?;
        fs::read(&abs).map_err(|e| CoreError::io("read", path, &e))
    }

    fn write_atomic(
        &self,
        path: &RelPath,
        bytes: &[u8],
        expect: Option<&BaseRev>,
    ) -> Result<WriteOutcome> {
        let abs = self.resolve(path)?;
        let tmp = Self::tmp_path(&abs)?;

        let mut f = fs::File::create(&tmp).map_err(|e| CoreError::io("create_temp", path, &e))?;
        let write_then_sync = (|| -> std::io::Result<()> {
            f.write_all(bytes)?;
            f.sync_all()
        })();
        drop(f);
        if let Err(e) = write_then_sync {
            let _ = fs::remove_file(&tmp);
            return Err(CoreError::io("write", path, &e));
        }

        if self.caps.preserve_mode {
            if let Ok(meta) = fs::metadata(&abs) {
                let _ = copy_mode(&meta, &tmp);
            }
        }

        // The last possible moment to see a concurrent change. Anything after
        // this is the rename itself.
        if let Some(base) = expect {
            match Self::stat_at(&abs) {
                Ok(current) => {
                    // The final guard checks content even if size and mtime match.
                    let same = fs::read(&abs)
                        .map(|b| hash(&b) == base.hash)
                        .unwrap_or(false);
                    if !same {
                        let _ = fs::remove_file(&tmp);
                        return Ok(WriteOutcome::Diverged(current));
                    }
                }
                Err(CoreError::Io {
                    kind: notes_model::IoKind::NotFound,
                    ..
                }) => {
                    // Removed externally while we were writing. Not our call to
                    // recreate it silently — the core decides (scope §12).
                    let _ = fs::remove_file(&tmp);
                    return Ok(WriteOutcome::Diverged(Stat {
                        size: 0,
                        mtime_ns: 0,
                        native_id: None,
                        kind: EntryKind::Other,
                    }));
                }
                Err(e) => {
                    let _ = fs::remove_file(&tmp);
                    return Err(e);
                }
            }
        }

        if let Err(e) = replace(&tmp, &abs) {
            let _ = fs::remove_file(&tmp);
            return Err(CoreError::io("replace", path, &e));
        }
        sync_dir(&abs);
        Ok(WriteOutcome::Written(Self::stat_at(&abs)?))
    }

    fn create_new(&self, path: &RelPath, bytes: &[u8]) -> Result<Stat> {
        let abs = self.resolve(path)?;
        // Publish complete bytes without replacing a concurrently created note.
        let mut temp = tempfile::Builder::new()
            .prefix(".notes-create-")
            .suffix(".tmp")
            .tempfile_in(abs.parent().expect("jailed file has a parent"))
            .map_err(|e| CoreError::io("create_temp", path, &e))?;
        temp.write_all(bytes)
            .map_err(|e| CoreError::io("write", path, &e))?;
        temp.as_file()
            .sync_all()
            .map_err(|e| CoreError::io("fsync", path, &e))?;
        let abs = self.resolve(path)?;
        temp.persist_noclobber(&abs).map_err(|e| {
            if e.error.kind() == std::io::ErrorKind::AlreadyExists {
                CoreError::AlreadyExists {
                    path: path.to_string(),
                }
            } else {
                CoreError::io("create_new", path, &e.error)
            }
        })?;
        sync_dir(&abs);
        Self::stat_at(&abs)
    }

    fn create_dir(&self, path: &RelPath) -> Result<()> {
        let abs = self.resolve(path)?;
        match fs::create_dir(&abs) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(CoreError::AlreadyExists {
                    path: path.to_string(),
                })
            }
            Err(e) => Err(CoreError::io("create_dir", path, &e)),
        }
    }

    fn rename(&self, from: &RelPath, to: &RelPath) -> Result<()> {
        let a = self.resolve(from)?;
        let b = self.resolve(to)?;
        if b.exists() {
            return Err(CoreError::AlreadyExists {
                path: to.to_string(),
            });
        }
        fs::rename(&a, &b).map_err(|e| CoreError::io("rename", from, &e))
    }

    /// Move to the operating system's trash when the backend has one, and say
    /// which of the two happened.
    ///
    /// **The fallback is never silent** (scope §7.7): a delete that could not be
    /// undone and one that can are different events, and `DeleteOutcome` is what
    /// the UI has to show. `caps.trash` is the workspace's answer to *is there a
    /// bin here* — a removable exFAT stick and a network share have none — and
    /// a `trash` call that fails anyway degrades to a permanent delete rather
    /// than refusing, because the user asked for the file to go.
    fn delete(&self, path: &RelPath) -> Result<DeleteOutcome> {
        let abs = self.resolve(path)?;
        // `symlink_metadata` first: it proves the entry exists *and* refuses to
        // follow a link, which matters more here than anywhere else.
        let meta = fs::symlink_metadata(&abs).map_err(|e| CoreError::io("stat", path, &e))?;

        // The `cfg` is not a duplicate of the `caps.trash` test inside it \[0.4\]:
        // `Caps::LOCAL` already reports `trash: false` on iOS and Android, but
        // the `trash` crate has no implementation for either target and fails to
        // *compile* there, so the runtime test alone would never be reached to
        // save us. The two say the same thing at the two different moments it
        // has to be said.
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        {
            if self.caps.trash {
                match trash::delete(&abs) {
                    Ok(()) => return Ok(DeleteOutcome::Trashed),
                    Err(_) => {
                        // No bin on this backend, no session bus, a root-owned
                        // container: all real, none of them a reason to refuse.
                        eprintln!("[notes] trash unavailable; deletion reports its final outcome");
                    }
                }
            }
        }

        let r = if meta.is_dir() {
            fs::remove_dir_all(&abs)
        } else {
            fs::remove_file(&abs)
        };
        r.map_err(|e| CoreError::io("delete", path, &e))?;
        Ok(DeleteOutcome::Permanent)
    }

    fn stat(&self, path: &RelPath) -> Result<Stat> {
        let abs = self.resolve(path)?;
        Self::stat_at(&abs)
    }

    fn watch(&self) -> crate::Watch {
        if !self.caps.watch {
            return crate::Watch::none(crate::Degraded::Unsupported(
                "this backend reports no watch capability".into(),
            ));
        }
        crate::watch::watch(&self.root)
    }
}

fn mtime_ns(meta: &fs::Metadata) -> i128 {
    meta.modified()
        .ok()
        .map(|t| match t.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_nanos() as i128,
            // A timestamp before 1970 is legal and rare; it is signed, not an error.
            Err(e) => -(e.duration().as_nanos() as i128),
        })
        .unwrap_or(0)
}

/// The filesystem's own identity for a file: what survives a rename and what a
/// copy does not share.
///
/// Correlation after an external rename asks one question — *is this the same
/// file under a different name?* — and a content hash cannot answer it for two
/// notes with identical text. The native id can.
#[cfg(unix)]
fn native_id(_: &Path, meta: &fs::Metadata) -> Option<NativeId> {
    use std::os::unix::fs::MetadataExt;
    Some(NativeId::Unix {
        dev: meta.dev(),
        ino: meta.ino(),
    })
}

/// Windows: `GetFileInformationByHandle`, which needs a handle rather than a
/// path, so the file is opened to ask.
///
/// The standard library exposes the same two numbers through
/// `MetadataExt::volume_serial_number` and `file_index`, but both sit behind the
/// unstable `windows_by_handle` feature — a stable build does not compile
/// against them, which is why this returned `None` from 0.1a until now
/// (`docs/DECISIONS-0.1a.md` D-24).
///
/// The open is a query, not a read: `access_mode(0)` asks for no access at all,
/// which is the documented way to read metadata for a file another process holds
/// open exclusively. `FILE_FLAG_BACKUP_SEMANTICS` is what allows a **directory**
/// to be opened this way; `FILE_FLAG_OPEN_REPARSE_POINT` makes a symlink report
/// its own identity rather than its target's, so that this agrees with the
/// `symlink_metadata` the rest of `stat_at` is built on.
///
/// A failure is `None` — the file vanished, or the volume does not keep an
/// index. That is the same degradation `ARCHITECTURE.md` §11 already specifies
/// for a backend without ids, and it is the safe direction: correlation falls
/// back to the hash and mints a new `NoteId` rather than reusing the wrong one.
#[cfg(windows)]
fn native_id(path: &Path, meta: &fs::Metadata) -> Option<NativeId> {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT,
    };

    let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
    if meta.file_type().is_symlink() {
        flags |= FILE_FLAG_OPEN_REPARSE_POINT;
    }
    let file = fs::OpenOptions::new()
        .access_mode(0)
        .custom_flags(flags)
        .open(path)
        .ok()?;

    // SAFETY: `info` is written by the call and only read when it returns
    // non-zero; the handle is owned by `file` and outlives the call.
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, &mut info) };
    if ok == 0 {
        return None;
    }

    Some(NativeId::Windows {
        volume: u64::from(info.dwVolumeSerialNumber),
        // One 64-bit index, delivered in two halves.
        index: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
    })
}

#[cfg(not(any(unix, windows)))]
fn native_id(_: &Path, _: &fs::Metadata) -> Option<NativeId> {
    None
}

#[cfg(unix)]
fn copy_mode(meta: &fs::Metadata, to: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(to, fs::Permissions::from_mode(meta.permissions().mode()))
}

#[cfg(not(unix))]
fn copy_mode(_: &fs::Metadata, _: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Rename over the target.
///
/// On Windows an antivirus scanner or the search indexer can hold a handle for
/// a few milliseconds after the file is closed, and the replace fails with a
/// sharing violation that is gone by the next attempt (`ARCHITECTURE.md` §5.2).
fn replace(tmp: &Path, target: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let mut last = None;
        for attempt in 0..5 {
            match fs::rename(tmp, target) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last = Some(e);
                    std::thread::sleep(std::time::Duration::from_millis(50 * (attempt + 1)));
                }
            }
        }
        Err(last.unwrap())
    }
    #[cfg(not(windows))]
    {
        fs::rename(tmp, target)
    }
}

/// Fsync the containing directory so the rename itself is durable. Without it
/// the file's contents survive a power cut and its name may not.
fn sync_dir(file: &Path) {
    #[cfg(unix)]
    if let Some(dir) = file.parent() {
        if let Ok(d) = fs::File::open(dir) {
            let _ = d.sync_all();
        }
    }
    #[cfg(not(unix))]
    let _ = file;
}
