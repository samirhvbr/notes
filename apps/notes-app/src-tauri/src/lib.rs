//! notes — milestone 0.0 spike.
//!
//! Not milestone 0.1a and not a draft of it. There is no BaseRev, no divergence
//! detection, no watcher, no draft recovery and no identity registry here; those
//! are specified in `.continue/SCOPE_final.md` §6, §7 and §12 and belong to the
//! crates under `crates/`, which do not exist yet. SCOPE §17 says of this
//! milestone: *nada vira produto*.
//!
//! What it does keep from the specification, because doing otherwise would make
//! the spike measure the wrong thing:
//!
//! * the frontend gets no filesystem capability, so every read and write below
//!   is a command (§2.5);
//! * every path is re-resolved and re-checked against the workspace root at the
//!   moment of use, not only when the folder is opened (§7.6);
//! * opening a folder writes nothing into it (§2.3);
//! * writes go through a temporary file and a rename (§7.4).

mod platform;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

const STATE_FILE: &str = "spike-state.json";

#[derive(Default)]
struct Spike {
    root: Mutex<Option<PathBuf>>,
    dmabuf_applied: bool,
    dmabuf: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SpikeEnv {
    os: String,
    arch: String,
    tauri_version: String,
    session: String,
    nvidia: bool,
    dmabuf_applied: bool,
    dmabuf_workaround: String,
    app_data_dir: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct NoteEntry {
    name: String,
    rel_path: String,
    size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceInfo {
    root: String,
    restored: bool,
    entries: Vec<NoteEntry>,
}

#[derive(Serialize, Deserialize)]
struct Persisted {
    root: String,
}

/// Errors reach the frontend as plain strings in this spike. Milestone 0.1a
/// replaces this with a coded `CoreError` the UI can translate — see the
/// architecture notes; a spike that invented its own error contract would only
/// have to unpick it.
type Res<T> = Result<T, String>;

fn state_path(app: &tauri::AppHandle) -> Res<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create app data dir: {e}"))?;
    Ok(dir.join(STATE_FILE))
}

/// Resolve `rel` inside `root` and refuse anything that escapes it.
///
/// The check is on the *resolved* path, so `..`, an absolute path and a symlink
/// pointing outside are all rejected by the same test rather than by three
/// string rules that each miss a case. It runs on every operation, because a
/// root validated once at open time says nothing about the path being used now.
fn resolve(root: &Path, rel: &str) -> Res<PathBuf> {
    if rel.is_empty() {
        return Err("empty path".into());
    }
    let candidate = root.join(rel);
    // The parent must exist for canonicalize to work on a file being created.
    let parent = candidate
        .parent()
        .ok_or_else(|| "path has no parent".to_string())?;
    let parent = parent
        .canonicalize()
        .map_err(|e| format!("cannot resolve {}: {e}", parent.display()))?;
    if !parent.starts_with(root) {
        return Err("path resolves outside the workspace root".into());
    }
    let name = candidate
        .file_name()
        .ok_or_else(|| "path has no file name".to_string())?;
    let full = parent.join(name);
    if !full.starts_with(root) {
        return Err("path resolves outside the workspace root".into());
    }
    Ok(full)
}

/// One level, names only. Nothing here opens a file: the 0.1a criterion is that
/// listing a 10 000-file workspace does not read content, and the spike should
/// not build a habit it will have to break.
fn list_markdown(root: &Path) -> Res<Vec<NoteEntry>> {
    let mut out = Vec::new();
    let dir = fs::read_dir(root).map_err(|e| format!("cannot read workspace: {e}"))?;
    for entry in dir.flatten() {
        let path = entry.path();
        let is_md = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
            .unwrap_or(false);
        if !is_md {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) if m.is_file() => m,
            _ => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        out.push(NoteEntry {
            rel_path: name.clone(),
            name,
            size: meta.len(),
        });
    }
    out.sort_by_key(|e| e.name.to_lowercase());
    Ok(out)
}

fn adopt(app: &tauri::AppHandle, spike: &Spike, path: &str, restored: bool) -> Res<WorkspaceInfo> {
    let root = PathBuf::from(path)
        .canonicalize()
        .map_err(|e| format!("cannot open {path}: {e}"))?;
    if !root.is_dir() {
        return Err(format!("{} is not a directory", root.display()));
    }
    let entries = list_markdown(&root)?;

    // Persisted in app data, never in the user's folder: opening a folder must
    // not modify it (SCOPE §2.3).
    let json = serde_json::to_vec_pretty(&Persisted {
        root: root.to_string_lossy().to_string(),
    })
    .map_err(|e| e.to_string())?;
    write_atomic(&state_path(app)?, &json)?;

    *spike.root.lock().unwrap() = Some(root.clone());
    Ok(WorkspaceInfo {
        root: root.to_string_lossy().to_string(),
        restored,
        entries,
    })
}

/// Temp file in the same directory, flushed, then renamed over the target.
///
/// This is the shape of SCOPE §7.4 and not its guarantee: there is no
/// permission copying, no Windows retry loop, and no `BaseRev` check, so it
/// prevents a truncated file and nothing else. Overwriting a concurrent external
/// change is exactly what milestone 0.1a adds, and this spike can lose one.
fn write_atomic(target: &Path, bytes: &[u8]) -> Res<()> {
    let dir = target
        .parent()
        .ok_or_else(|| "target has no parent".to_string())?;
    let name = target
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "target has no file name".to_string())?;
    let tmp = dir.join(format!(".{name}.tmp-{}", std::process::id()));

    let mut f = fs::File::create(&tmp).map_err(|e| format!("cannot create temp file: {e}"))?;
    f.write_all(bytes).map_err(|e| format!("write failed: {e}"))?;
    f.sync_all().map_err(|e| format!("fsync failed: {e}"))?;
    drop(f);

    fs::rename(&tmp, target).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("replace failed: {e}")
    })
}

#[tauri::command]
fn spike_env(app: tauri::AppHandle, spike: State<'_, Spike>) -> Res<SpikeEnv> {
    Ok(SpikeEnv {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        tauri_version: tauri::VERSION.to_string(),
        session: platform::session_kind().to_string(),
        nvidia: platform::nvidia_present(),
        dmabuf_applied: spike.dmabuf_applied,
        dmabuf_workaround: spike.dmabuf.clone(),
        app_data_dir: app
            .path()
            .app_data_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|e| format!("unavailable: {e}")),
    })
}

#[tauri::command]
fn open_workspace(app: tauri::AppHandle, spike: State<'_, Spike>, path: String) -> Res<WorkspaceInfo> {
    adopt(&app, &spike, &path, false)
}

/// Acceptance criterion 3 lives here: after the process is killed and restarted,
/// does the folder chosen last time still open? A failure is returned rather
/// than swallowed, because "no workspace" and "the authorisation is gone" are
/// the two answers the spike exists to tell apart.
#[tauri::command]
fn restore_workspace(app: tauri::AppHandle, spike: State<'_, Spike>) -> Res<Option<WorkspaceInfo>> {
    let p = state_path(&app)?;
    let Ok(bytes) = fs::read(&p) else {
        return Ok(None);
    };
    let saved: Persisted =
        serde_json::from_slice(&bytes).map_err(|e| format!("state file unreadable: {e}"))?;
    adopt(&app, &spike, &saved.root, true).map(Some)
}

#[tauri::command]
fn read_note(spike: State<'_, Spike>, rel_path: String) -> Res<String> {
    let guard = spike.root.lock().unwrap();
    let root = guard.as_ref().ok_or("no workspace open")?;
    let path = resolve(root, &rel_path)?;
    let bytes = fs::read(&path).map_err(|e| format!("cannot read: {e}"))?;
    String::from_utf8(bytes).map_err(|_| "not valid UTF-8 — 0.1a opens this read-only".to_string())
}

#[tauri::command]
fn write_note(spike: State<'_, Spike>, rel_path: String, contents: String) -> Res<u64> {
    let guard = spike.root.lock().unwrap();
    let root = guard.as_ref().ok_or("no workspace open")?;
    let path = resolve(root, &rel_path)?;
    let bytes = contents.as_bytes();
    write_atomic(&path, bytes)?;
    Ok(bytes.len() as u64)
}

pub fn run() {
    // Before the builder, and therefore before any webview: WebKit reads the
    // variable when it creates one.
    let decision = platform::apply_dmabuf_workaround();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Spike {
            root: Mutex::new(None),
            dmabuf_applied: decision.applied,
            dmabuf: decision.explanation,
        })
        .invoke_handler(tauri::generate_handler![
            spike_env,
            open_workspace,
            restore_workspace,
            read_note,
            write_note
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("notes-spike-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d.canonicalize().unwrap()
    }

    #[test]
    fn rejects_parent_traversal() {
        let root = tmpdir("traversal");
        fs::create_dir_all(root.join("sub")).unwrap();
        assert!(resolve(&root, "../outside.md").is_err());
        assert!(resolve(&root, "sub/../../outside.md").is_err());
    }

    #[test]
    fn rejects_absolute_path() {
        let root = tmpdir("absolute");
        assert!(resolve(&root, "/etc/passwd").is_err());
    }

    #[test]
    fn accepts_path_inside_root() {
        let root = tmpdir("inside");
        fs::write(root.join("a.md"), b"x").unwrap();
        assert_eq!(resolve(&root, "a.md").unwrap(), root.join("a.md"));
    }

    #[test]
    fn atomic_write_roundtrips_bytes() {
        let root = tmpdir("roundtrip");
        let target = root.join("n.md");
        for payload in [
            &b""[..],
            &b"plain"[..],
            &b"crlf\r\nlines\r\n"[..],
            &[0xEF, 0xBB, 0xBF, b'b', b'o', b'm'][..],
            "acentuação e emoji 🌱".as_bytes(),
        ] {
            write_atomic(&target, payload).unwrap();
            assert_eq!(fs::read(&target).unwrap(), payload);
        }
    }

    #[test]
    fn atomic_write_leaves_no_temp_file() {
        let root = tmpdir("notemp");
        write_atomic(&root.join("n.md"), b"hello").unwrap();
        let leftovers: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty(), "temp files left behind: {leftovers:?}");
    }

    #[test]
    fn listing_ignores_non_markdown() {
        let root = tmpdir("listing");
        fs::write(root.join("a.md"), b"").unwrap();
        fs::write(root.join("b.markdown"), b"").unwrap();
        fs::write(root.join("c.txt"), b"").unwrap();
        fs::create_dir(root.join("d.md")).unwrap();
        let names: Vec<_> = list_markdown(&root).unwrap().into_iter().map(|e| e.name).collect();
        assert_eq!(names, vec!["a.md", "b.markdown"]);
    }
}
