//! One Tauri command per operation (`docs/ARCHITECTURE.md` §7.1, §18.8).
//!
//! Every function here is: parse → call `notes-core` → return. **No branching
//! on business state.** A single `dispatch` command was rejected because Tauri's
//! capabilities are per command, and permitting `dispatch` would permit
//! everything — which is exactly what the security posture forbids.

use std::path::PathBuf;
use std::sync::Mutex;

use notes_core::search::{QuickOpen, SearchId, SearchOpts, SearchProgress};
use notes_core::{
    ConflictChoice, Conflicts, Deleted, Document, DraftChoice, DraftInfo, DraftReason, OpenedNote,
    Reconciled, Rendered, SaveResult, Session, Settings, WorkspaceEntry, WorkspaceInfo,
    WorkspaceService,
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
    pub nouveau: bool,
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
        nvidia: crate::linux::nvidia_proprietary(),
        nouveau: crate::linux::nouveau_present(),
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

/// Open an `http(s)` URL in the operating system's browser.
///
/// **The scheme is checked here as well as in the capability file**, and that
/// is not redundancy for its own sake: the capability is what the WebView may
/// ask for, and this is what the process will do. Scope §8.4 — *"Links externos
/// `http(s)` abrem no navegador do SO por clique. Outros esquemas recusados.
/// Link não dispara shell."* The renderer never emits another scheme; this is
/// the guarantee that holds even if it one day does.
/// **`shell().open` is deprecated in favour of `tauri-plugin-opener`, and the
/// migration is deliberately not made here.** Swapping the plugin means
/// replacing `shell:allow-open` with `opener:allow-open-url` in
/// `capabilities/default.json` — a permission edit and a new dependency, and
/// scope §19 sends both to the owner rather than letting an agent make them on
/// the way past. The call still works and the scope restriction is unchanged;
/// `docs/DECISIONS-0.1b.md` D-09 carries the one-line change for whoever does.
#[allow(deprecated)]
#[tauri::command]
pub fn shell_open(app: tauri::AppHandle, url: String) -> R<()> {
    use tauri_plugin_shell::ShellExt;

    let lower = url.trim().to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err(CoreError::InvalidPath {
            path: url,
            reason: "only http and https links are opened".into(),
        });
    }
    app.shell()
        .open(&url, None)
        .map_err(|e| CoreError::Internal {
            message: format!("opening {url}: {e}"),
        })
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

// ---- reconciliation ----------------------------------------------------

/// Start watching the open workspace. Returns the reason when the platform
/// cannot, so the interface can say why it is polling instead — the inotify
/// limit comes back with the `sysctl` that raises it.
#[tauri::command]
pub fn watch_start(app: State<'_, App>) -> R<Option<String>> {
    svc(&app)?.start_watch()
}

/// One watcher-driven tick. **The frontend passes which notes are dirty**,
/// because the core does not hold buffers and cannot know
/// (`docs/DECISIONS-0.1a.md` D-11).
#[tauri::command]
pub fn reconcile_tick(app: State<'_, App>, dirty: Vec<NoteId>) -> R<Reconciled> {
    svc(&app)?.tick(&dirty)
}

/// A full scan: window focus, tab switch, manual refresh.
#[tauri::command]
pub fn reconcile_all(app: State<'_, App>, dirty: Vec<NoteId>) -> R<Reconciled> {
    svc(&app)?.reconcile_all(&dirty)
}

// ---- entries -----------------------------------------------------------

/// Rename in place, **keeping the `NoteId`** — the tab and its cursor survive
/// (scope §17). Renaming a folder carries every note beneath it.
#[tauri::command]
pub fn entry_rename(app: State<'_, App>, path: RelPath, new_name: String) -> R<Entry> {
    svc(&app)?.rename_entry(&path, &new_name)
}

/// Move into another directory, keeping the name and the `NoteId`. A collision
/// comes back as `AlreadyExists` naming what is in the way, so the interface can
/// ask rather than guess.
#[tauri::command]
pub fn entry_move(app: State<'_, App>, path: RelPath, to_dir: RelPath) -> R<Entry> {
    svc(&app)?.move_entry(&path, &to_dir)
}

/// Copy beside the original as `nome (copy).md`. Never overwrites, and the copy
/// gets an identity of its own.
#[tauri::command]
pub fn entry_duplicate(app: State<'_, App>, path: RelPath) -> R<Entry> {
    svc(&app)?.duplicate_entry(&path)
}

/// Delete, and say whether it went to the bin or not (scope §7.7).
#[tauri::command]
pub fn entry_delete(app: State<'_, App>, path: RelPath) -> R<Deleted> {
    svc(&app)?.delete_entry(&path)
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

/// What the watcher has managed so far. Polled beside `reconcile_tick`.
#[tauri::command]
pub fn watch_status(app: State<'_, App>) -> R<notes_core::WatchStatus> {
    Ok(svc(&app)?.watch_status())
}

// ---- search (0.1c) -----------------------------------------------------

/// Fuzzy match over paths, from memory. Never reads a file, so it is safe to
/// call on every keystroke.
#[tauri::command]
pub fn quick_open(app: State<'_, App>, query: String, limit: usize) -> R<QuickOpen> {
    svc(&app)?.quick_open(&query, limit)
}

/// Start a content search. Starting one cancels the previous, because the caller
/// is a search box and the previous query is no longer wanted.
#[tauri::command]
pub fn search_start(app: State<'_, App>, query: String, opts: SearchOpts) -> R<SearchId> {
    svc(&app)?.search_start(&query, opts)
}

/// Drain the hits found since the last poll. Following the same shape as
/// `reconcile_tick`: the core collects, the frontend asks.
#[tauri::command]
pub fn search_poll(app: State<'_, App>, id: SearchId) -> R<SearchProgress> {
    svc(&app)?.search_poll(id)
}

#[tauri::command]
pub fn search_cancel(app: State<'_, App>, id: SearchId) -> R<()> {
    svc(&app)?.search_cancel(id)
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
