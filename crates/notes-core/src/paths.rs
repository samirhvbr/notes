use std::path::PathBuf;

use notes_model::{CoreError, WorkspaceId};

/// Where the application keeps everything that is not a note.
///
/// **The core resolves this itself and does not use Tauri's `app_data_dir()`**
/// (`ARCHITECTURE.md` §18.4): `notes-mcp` runs without Tauri from 0.3, and the
/// two processes have to agree on one directory or the registry has two
/// versions.
///
/// `NOTES_DATA_DIR` overrides it — that is what makes the whole service testable
/// without touching the developer's real data, and what "portable install"
/// will mean later.
pub fn data_dir() -> Result<PathBuf, CoreError> {
    if let Some(over) = std::env::var_os("NOTES_DATA_DIR") {
        return Ok(PathBuf::from(over));
    }
    dirs::data_dir()
        .map(|d| d.join("notes"))
        .ok_or_else(|| CoreError::Internal { message: "no data directory on this platform".into() })
}

pub fn workspaces_index(data: &std::path::Path) -> PathBuf {
    data.join("workspaces.json")
}

pub fn global_settings(data: &std::path::Path) -> PathBuf {
    data.join("settings.json")
}

pub fn workspace_dir(data: &std::path::Path, id: WorkspaceId) -> PathBuf {
    data.join("workspaces").join(id.to_string())
}

pub fn registry_file(ws: &std::path::Path) -> PathBuf {
    ws.join("registry.json")
}
pub fn session_file(ws: &std::path::Path) -> PathBuf {
    ws.join("session.json")
}
pub fn lock_file(ws: &std::path::Path) -> PathBuf {
    ws.join("write.lock")
}
pub fn drafts_dir(ws: &std::path::Path) -> PathBuf {
    ws.join("drafts")
}
pub fn conflicts_dir(ws: &std::path::Path) -> PathBuf {
    ws.join("conflicts")
}
