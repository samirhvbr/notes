//! Operator-only provisioning. HTTP never receives administrative authority.
use crate::Result;
use notes_core::agent::Permission;
use notes_model::RelPath;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use subtle::ConstantTimeEq;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credential {
    pub id: Uuid,
    pub label: String,
    pub workspace: String,
    pub scope: RelPath,
    pub permissions: BTreeSet<Permission>,
    pub review: bool,
    pub revoked: bool,
    digest: String,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Store {
    pub schema: u32,
    pub credentials: Vec<Credential>,
}

pub fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    if fs::symlink_metadata(path)?.file_type().is_symlink() || !path.is_dir() {
        return Err("directory must not be a symlink".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
pub fn private_file(path: &Path, exclusive: bool) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    if exclusive {
        options.create_new(true);
    } else {
        options.create(true).truncate(false);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    if path.exists() && fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err("file must not be a symlink".into());
    }
    Ok(options.open(path)?)
}
pub fn data_root(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err("NOTES_SERVER_DATA must be absolute".into());
    }
    private_dir(path)?;
    let canonical = fs::canonicalize(path)?;
    for dir in ["admin", "workspaces", "state", "audit"] {
        private_dir(&canonical.join(dir))?;
    }
    Ok(canonical)
}
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}
pub fn workspace(root: &Path, name: &str) -> Result<PathBuf> {
    if !valid_name(name) {
        return Err("invalid workspace name".into());
    }
    let path = root.join("workspaces").join(name);
    let stat = fs::symlink_metadata(&path)?;
    if !stat.is_dir() || stat.file_type().is_symlink() {
        return Err("invalid workspace directory".into());
    }
    Ok(path)
}
pub fn lock(root: &Path) -> Result<fd_lock::RwLock<File>> {
    Ok(fd_lock::RwLock::new(private_file(
        &root.join("admin/lock"),
        false,
    )?))
}
pub fn load(root: &Path) -> Result<Store> {
    let path = root.join("admin/tokens.json");
    if !path.exists() {
        return Ok(Store {
            schema: 1,
            credentials: vec![],
        });
    }
    let meta = fs::symlink_metadata(&path)?;
    if !meta.is_file() || meta.len() > 4 * 1024 * 1024 {
        return Err("invalid credential store".into());
    }
    let store: Store = serde_json::from_slice(&fs::read(path)?)?;
    if store.schema != 1 {
        return Err("unsupported credential schema; restore a compatible backup".into());
    }
    Ok(store)
}
pub fn save(root: &Path, store: &Store) -> Result<()> {
    let mut tmp = tempfile::NamedTempFile::new_in(root.join("admin"))?;
    tmp.write_all(&serde_json::to_vec_pretty(store)?)?;
    tmp.as_file().sync_all()?;
    tmp.persist(root.join("admin/tokens.json"))?;
    #[cfg(unix)]
    File::open(root.join("admin"))?.sync_all()?;
    Ok(())
}
pub fn authenticate(store: &Store, bearer: &str) -> Option<Credential> {
    let (id, secret) = bearer.strip_prefix("nt_")?.split_once('.')?;
    let id = Uuid::parse_str(id).ok()?;
    if secret.len() != 64 {
        return None;
    }
    let digest = blake3::hash(secret.as_bytes()).to_hex().to_string();
    store
        .credentials
        .iter()
        .find(|c| {
            c.id == id && !c.revoked && bool::from(c.digest.as_bytes().ct_eq(digest.as_bytes()))
        })
        .cloned()
}
pub fn create_token(
    root: &Path,
    label: String,
    name: String,
    scope: RelPath,
    permissions: BTreeSet<Permission>,
    review: bool,
    output: &Path,
) -> Result<Uuid> {
    if label.is_empty() || label.len() > 100 || label.chars().any(char::is_control) {
        return Err("invalid integration label".into());
    }
    let config = notes_core::agent::AgentConfig {
        workspace: workspace(root, &name)?,
        scope: scope.clone(),
        permissions: permissions.clone(),
        review,
    };
    notes_core::agent::AgentService::with_data_dir(config, &root.join("state"))?;
    let mut lock = lock(root)?;
    let _guard = lock.write()?;
    let mut store = load(root)?;
    if store.credentials.len() >= 1024 {
        return Err("credential limit reached".into());
    }
    let id = Uuid::new_v4();
    let secret = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let token = format!("nt_{id}.{secret}");
    let mut file = private_file(output, true)?;
    file.write_all(token.as_bytes())?;
    file.sync_all()?;
    store.credentials.push(Credential {
        id,
        label,
        workspace: name,
        scope,
        permissions,
        review,
        revoked: false,
        digest: blake3::hash(secret.as_bytes()).to_hex().to_string(),
    });
    save(root, &store)?;
    audit(
        root,
        "operator",
        "local",
        "token_create",
        "ok",
        &Uuid::new_v4().to_string(),
    )?;
    Ok(id)
}
pub fn revoke(root: &Path, id: Uuid) -> Result<()> {
    let mut lock = lock(root)?;
    let _guard = lock.write()?;
    let mut store = load(root)?;
    store
        .credentials
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or("unknown credential")?
        .revoked = true;
    save(root, &store)?;
    audit(
        root,
        "operator",
        "local",
        "token_revoke",
        "ok",
        &Uuid::new_v4().to_string(),
    )
}
/// Caller holds the admin lock (exclusive for administration, shared for HTTP).
/// A separate OS lock serializes audit rotation across threads and processes.
pub fn audit(
    root: &Path,
    actor: &str,
    peer: &str,
    operation: &str,
    result: &str,
    request: &str,
) -> Result<()> {
    audit_target(root, actor, peer, operation, result, request, None)
}
pub fn audit_target(
    root: &Path,
    actor: &str,
    peer: &str,
    operation: &str,
    result: &str,
    request: &str,
    target_ref: Option<&str>,
) -> Result<()> {
    let mut lock = fd_lock::RwLock::new(private_file(&root.join("audit/lock"), false)?);
    let _guard = lock.write()?;
    let path = root.join("audit/events.jsonl");
    if path.metadata().is_ok_and(|m| m.len() >= 10 * 1024 * 1024) {
        for n in (1..=4).rev() {
            let from = if n == 1 {
                path.clone()
            } else {
                root.join(format!("audit/events.{}.jsonl", n - 1))
            };
            let to = root.join(format!("audit/events.{n}.jsonl"));
            if to.exists() {
                fs::remove_file(&to)?;
            }
            if from.exists() {
                fs::rename(from, to)?;
            }
        }
    }
    let mut file = private_file(&path, false)?;
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::End(0))?;
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let event = serde_json::json!({"time":time,"actor":actor,"peer":peer,"operation":operation,"result":result,"request":request,"target_ref":target_ref});
    writeln!(file, "{event}")?;
    file.sync_data()?;
    Ok(())
}
