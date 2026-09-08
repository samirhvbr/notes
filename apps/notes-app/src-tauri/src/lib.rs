//! The Tauri shell. Thin by rule (ADR-003): it wires commands to `notes-core`
//! and holds no policy of its own.

mod asset;
mod commands;
pub mod linux;

use std::sync::Mutex;

use commands::{App, DmabufReport};
use notes_core::WorkspaceService;

pub fn run() {
    // The service is built before the window so the settings that govern the
    // WebView are readable before it exists. A settings file that cannot be read
    // yields defaults, and the default is `auto` — never `off`.
    let service = WorkspaceService::new().expect("resolve the data directory");
    let setting = service.settings().linux.webkit_dmabuf_workaround.clone();

    // Before `tauri::Builder`, and therefore before any WebView: WebKit reads
    // the variable when it creates one.
    let decision = linux::apply(&setting);
    eprintln!("[notes] dmabuf: {}", decision.explanation);

    tauri::Builder::default()
        // Window lifecycle, logged.
        //
        // A window that disappears with the process exiting **0** is not a
        // crash: Tauri ends its event loop when the last window is gone, so
        // "closed" and "destroyed by something else" are indistinguishable from
        // outside. This says which, and who asked — without it the next
        // occurrence is as undiagnosable as the first.
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { .. } => {
                eprintln!("[notes] window {}: close requested", window.label());
            }
            tauri::WindowEvent::Destroyed => {
                eprintln!("[notes] window {}: destroyed", window.label());
            }
            tauri::WindowEvent::Focused(f) => {
                eprintln!("[notes] window {}: focused={f}", window.label());
            }
            _ => {}
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .manage(App {
            svc: Mutex::new(service),
            dmabuf: DmabufReport {
                applied: decision.applied,
                explanation: decision.explanation,
            },
        })
        // The `notes-asset://` scheme, and the second entry point into the
        // workspace (`docs/ARCHITECTURE.md` §10). It resolves nothing itself:
        // the path goes to `notes-core`, which applies the same root jail as
        // every command, and only image types come back.
        .register_uri_scheme_protocol("notes-asset", asset::serve)
        .invoke_handler(tauri::generate_handler![
            commands::env_report,
            commands::workspace_open,
            commands::markdown_render,
            commands::markdown_outline,
            commands::markdown_trust_set,
            commands::workspace_create,
            commands::workspace_restore_last,
            commands::workspace_recent,
            commands::workspace_close,
            commands::tree_list,
            commands::note_open,
            commands::note_save,
            commands::note_flush,
            commands::note_reload,
            commands::note_close,
            commands::note_convert_eol,
            commands::conflict_resolve,
            commands::conflict_list,
            commands::shell_open,
            commands::note_create,
            commands::dir_create,
            commands::watch_start,
            commands::reconcile_tick,
            commands::reconcile_all,
            commands::entry_rename,
            commands::entry_move,
            commands::entry_duplicate,
            commands::entry_delete,
            commands::draft_write,
            commands::draft_list,
            commands::draft_resolve,
            commands::quick_open,
            commands::search_start,
            commands::search_poll,
            commands::search_cancel,
            commands::session_get,
            commands::session_save,
            commands::settings_get,
            commands::settings_set,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
