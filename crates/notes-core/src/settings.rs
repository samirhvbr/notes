use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::state::Schemad;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EditorSettings {
    pub font_size: u32,
    pub line_numbers: bool,
    pub word_wrap: bool,
    pub tab_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FileSettings {
    pub autosave_ms: u64,
    pub show_hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MarkdownSettings {
    pub default_view: String,
    pub raw_html: bool,
    pub remote_images: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UiSettings {
    pub locale: String,
    pub theme: String,
}

/// `auto` applies the WebKitGTK workaround when Wayland and NVIDIA are both
/// present; `off` never does; `force` always does.
///
/// **A missing or unreadable settings file degrades to `auto`, never to `off`**
/// (`ARCHITECTURE.md` §12): not applying it yields a black window, applying it
/// needlessly yields slightly slower compositing.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinuxSettings {
    pub webkit_dmabuf_workaround: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Settings {
    pub schema: u32,
    pub editor: EditorSettings,
    pub files: FileSettings,
    pub markdown: MarkdownSettings,
    pub ui: UiSettings,
    pub linux: LinuxSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: 1,
            editor: EditorSettings { font_size: 14, line_numbers: true, word_wrap: true, tab_size: 2 },
            files: FileSettings { autosave_ms: 750, show_hidden: false },
            markdown: MarkdownSettings {
                default_view: "source".into(),
                raw_html: false,
                remote_images: false,
            },
            ui: UiSettings { locale: "auto".into(), theme: "dark".into() },
            linux: LinuxSettings { webkit_dmabuf_workaround: "auto".into() },
        }
    }
}

impl Schemad for Settings {
    const CURRENT: u32 = 1;
    const NAME: &'static str = "settings.json";
    fn schema(&self) -> u32 {
        self.schema
    }
}

/// UI state, per workspace. Resettable: invalid content starts an empty session
/// and never stops the workspace opening.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Session {
    pub schema: u32,
    #[serde(default)]
    pub tabs: Vec<Tab>,
    #[serde(default)]
    pub active_tab: Option<notes_model::NoteId>,
    #[serde(default)]
    pub view_mode: String,
    #[serde(default = "yes")]
    pub sidebar_open: bool,
    #[serde(default = "sidebar_width")]
    pub sidebar_width: u32,
}

fn yes() -> bool {
    true
}
fn sidebar_width() -> u32 {
    300
}

/// Tabs are restored at 0.1c. The fields are named now so that arriving there is
/// not a schema migration for a reason already known today.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Tab {
    pub note_id: notes_model::NoteId,
    pub path: notes_model::RelPath,
    pub line: u32,
    pub col: u32,
    pub scroll_top: u32,
    pub pinned: bool,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            schema: 1,
            tabs: Vec::new(),
            active_tab: None,
            view_mode: "source".into(),
            sidebar_open: true,
            sidebar_width: 300,
        }
    }
}

impl Schemad for Session {
    const CURRENT: u32 = 1;
    const NAME: &'static str = "session.json";
    fn schema(&self) -> u32 {
        self.schema
    }
}
