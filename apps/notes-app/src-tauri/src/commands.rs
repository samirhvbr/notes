//! One Tauri command per operation (`docs/ARCHITECTURE.md` §7.1, §18.8).
//!
//! Every function here is: parse → call `notes-core` → return. **No branching
//! on business state.** A single `dispatch` command was rejected because Tauri's
//! capabilities are per command, and permitting `dispatch` would permit
//! everything — which is exactly what the security posture forbids.

use std::path::PathBuf;
use std::sync::Mutex;

use notes_core::{
    ConflictChoice, Conflicts, Document, DraftChoice, DraftInfo, DraftReason, OpenedNote, Rendered,
    SaveResult, Session, Settings, WorkspaceEntry, WorkspaceInfo, WorkspaceService,
};
use notes_model::{BaseRev, CoreError, Entry, NoteId, RelPath};
use serde::Serialize;
use tauri::State;

pub struct App {
    pub svc: Mutex<WorkspaceService>,
    pub dmabuf: DmabufReport,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmabufReport {
    pub applied: bool,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvReport {
    pub os: String,
    pub arch: String,
    pub tauri_version: String,
    pub session: String,
    pub nvidia: bool,
    pub dmabuf_applied: bool,
    pub dmabuf_explanation: String,
    pub data_dir: String,
}

type R<T> = Result<T, CoreError>;

/// The service is behind a `Mutex` and every command takes it briefly. A
/// poisoned lock means a previous command panicked, which is a bug and is
/// reported as one rather than propagating a panic into the WebView.
fn svc<'a>(app: &'a State<'_, App>) -> R<std::sync::MutexGuard<'a, WorkspaceService>> {
    app.svc.lock().map_err(|_| CoreError::Internal {
        message: "the workspace service panicked in an earlier command".into(),
    })
}

#[tauri::command]
pub fn env_report(app: State<'_, App>) -> R<EnvReport> {
    Ok(EnvReport {
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        tauri_version: tauri::VERSION.into(),
        session: crate::linux::session_kind().into(),
        nvidia: crate::linux::nvidia_present(),
        dmabuf_applied: app.dmabuf.applied,
        dmabuf_explanation: app.dmabuf.explanation.clone(),
        data_dir: svc(&app)?.data_dir().display().to_string(),
    })
}

// ---- workspace ---------------------------------------------------------

#[tauri::command]
pub fn workspace_open(app: State<'_, App>, root: String) -> R<WorkspaceInfo> {
    svc(&app)?.open_workspace(&PathBuf::from(root))
}

#[tauri::command]
pub fn workspace_create(app: State<'_, App>, parent: String, name: String) -> R<WorkspaceInfo> {
    svc(&app)?.create_workspace(&PathBuf::from(parent), &name)
}

#[tauri::command]
pub fn workspace_restore_last(app: State<'_, App>) -> R<Option<WorkspaceInfo>> {
    svc(&app)?.restore_last_workspace()
}

#[tauri::command]
pub fn workspace_recent(app: State<'_, App>) -> R<Vec<WorkspaceEntry>> {
    svc(&app)?.recent_workspaces()
}

#[tauri::command]
pub fn workspace_close(app: State<'_, App>, dirty: Vec<NoteId>) -> R<()> {
    svc(&app)?.close_workspace(&dirty)
}

// ---- tree --------------------------------------------------------------

#[tauri::command]
pub fn tree_list(app: State<'_, App>, dir: RelPath) -> R<Vec<Entry>> {
    svc(&app)?.list_dir(&dir)
}

// ---- notes -------------------------------------------------------------

#[tauri::command]
pub fn note_open(app: State<'_, App>, path: RelPath) -> R<OpenedNote> {
    svc(&app)?.open_note(&path)
}

#[tauri::command]
pub fn note_save(
    app: State<'_, App>,
    note_id: NoteId,
    text: String,
    buffer_version: u64,
    base_rev: BaseRev,
) -> R<SaveResult> {
    svc(&app)?.save_note(note_id, &text, buffer_version, &base_rev)
}

/// The same call with no debounce. Separate so the frontend's intent is legible
/// in the log and in the capability list, not because the core does anything
/// different.
#[tauri::command]
pub fn note_flush(
    app: State<'_, App>,
    note_id: NoteId,
    text: String,
    buffer_version: u64,
    base_rev: BaseRev,
) -> R<SaveResult> {
    svc(&app)?.save_note(note_id, &text, buffer_version, &base_rev)
}

#[tauri::command]
pub fn note_create(app: State<'_, App>, dir: RelPath, name: String) -> R<Entry> {
    svc(&app)?.create_note(&dir, &name)
}

#[tauri::command]
pub fn dir_create(app: State<'_, App>, dir: RelPath, name: String) -> R<Entry> {
    svc(&app)?.create_dir(&dir, &name)
}

/// Re-read a note from disk. The caller decides when a buffer is clean enough
/// to be replaced; this command does not.
#[tauri::command]
pub fn note_reload(app: State<'_, App>, note_id: NoteId) -> R<OpenedNote> {
    svc(&app)?.reload_note(note_id)
}

/// Forget the per-note state a closed tab no longer needs. **Does not touch the
/// draft** — a draft outlives the tab by design.
#[tauri::command]
pub fn note_close(app: State<'_, App>, note_id: NoteId) -> R<()> {
    svc(&app)?.close_note(note_id)
}

/// Rewrite a note's line endings, because the user asked. The old bytes go to
/// `conflicts/` first.
#[tauri::command]
pub fn note_convert_eol(
    app: State<'_, App>,
    note_id: NoteId,
    eol: notes_model::Eol,
) -> R<OpenedNote> {
    svc(&app)?.convert_eol(note_id, eol)
}

// ---- conflicts ---------------------------------------------------------

/// Keep mine · use the disk's · save as a copy. **"Compare" is not here**: it
/// changes nothing on disk and reads two strings the frontend already holds, so
/// it is a screen rather than a command (`docs/ARCHITECTURE.md` §17.1).
#[tauri::command]
pub fn conflict_resolve(
    app: State<'_, App>,
    note_id: NoteId,
    text: String,
    base_rev: BaseRev,
    choice: ConflictChoice,
) -> R<OpenedNote> {
    svc(&app)?.resolve_conflict(note_id, &text, &base_rev, choice)
}

/// Everything kept in `conflicts/`, and what it costs on disk.
#[tauri::command]
pub fn conflict_list(app: State<'_, App>) -> R<Conflicts> {
    svc(&app)?.list_conflicts()
}

// ---- markdown ----------------------------------------------------------

/// Sanitized HTML for the buffer the frontend is holding.
///
/// The text is sent rather than read from disk because the preview follows what
/// is being typed; `path` only resolves relative links. **This is the whole of
/// the preview IR** — no AST crosses (`docs/ARCHITECTURE.md` §10).
#[tauri::command]
pub fn markdown_render(app: State<'_, App>, path: RelPath, text: String) -> R<Rendered> {
    svc(&app)?.render_markdown(&path, &text)
}

/// The outline, the links and the front-matter span, with no HTML rendered.
/// Separate from `markdown_render` because the sidebar wants it while the
/// preview pane is closed.
#[tauri::command]
pub fn markdown_outline(app: State<'_, App>, text: String) -> R<Document> {
    svc(&app)?.outline(&text)
}

/// Turn raw HTML or remote images on for **this** workspace.
#[tauri::command]
pub fn markdown_trust_set(
    app: State<'_, App>,
    raw_html: Option<bool>,
    remote_images: Option<bool>,
) -> R<()> {
    svc(&app)?.set_markdown_trust(raw_html, remote_images)
}

// ---- drafts ------------------------------------------------------------

#[tauri::command]
pub fn draft_write(
    app: State<'_, App>,
    note_id: NoteId,
    text: String,
    buffer_version: u64,
    base_rev: BaseRev,
    reason: DraftReason,
) -> R<DraftInfo> {
    svc(&app)?.write_draft(note_id, &text, buffer_version, &base_rev, reason)
}

#[tauri::command]
pub fn draft_list(app: State<'_, App>) -> R<Vec<DraftInfo>> {
    svc(&app)?.list_drafts()
}

#[tauri::command]
pub fn draft_resolve(app: State<'_, App>, note_id: NoteId, choice: DraftChoice) -> R<OpenedNote> {
    svc(&app)?.resolve_draft(note_id, choice)
}

// ---- session and settings ----------------------------------------------

#[tauri::command]
pub fn session_get(app: State<'_, App>) -> R<Session> {
    svc(&app)?.session()
}

#[tauri::command]
pub fn session_save(app: State<'_, App>, session: Session) -> R<()> {
    svc(&app)?.save_session(&session)
}

#[tauri::command]
pub fn settings_get(app: State<'_, App>) -> R<Settings> {
    Ok(svc(&app)?.settings().clone())
}

#[tauri::command]
pub fn settings_set(app: State<'_, App>, settings: Settings) -> R<()> {
    svc(&app)?.set_settings(settings)
}
