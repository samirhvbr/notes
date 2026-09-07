//! The Tauri shell. Thin by rule (ADR-003): it wires commands to `notes-core`
//! and holds no policy of its own.

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
        .invoke_handler(tauri::generate_handler![
            commands::env_report,
            commands::workspace_open,
            commands::workspace_create,
            commands::workspace_restore_last,
            commands::workspace_recent,
            commands::workspace_close,
            commands::tree_list,
            commands::note_open,
            commands::note_save,
            commands::note_flush,
            commands::note_create,
            commands::dir_create,
            commands::draft_write,
            commands::draft_list,
            commands::draft_resolve,
            commands::session_get,
            commands::session_save,
            commands::settings_get,
            commands::settings_set,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
