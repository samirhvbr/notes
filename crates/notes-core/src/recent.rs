use crate::{
    state::{self, Loaded, Schemad},
    Result, WorkspaceService,
};
use notes_fs::FileSystem;
use notes_model::{NoteId, RelPath};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RecentNote {
    pub note_id: NoteId,
    pub path: RelPath,
    pub opened: String,
}
#[derive(Serialize, Deserialize)]
struct Recents {
    schema: u32,
    notes: Vec<RecentNote>,
}
impl Schemad for Recents {
    const CURRENT: u32 = 1;
    const NAME: &'static str = "recent.json";
    fn schema(&self) -> u32 {
        self.schema
    }
}
impl WorkspaceService {
    pub fn recent_notes(&self) -> Result<Vec<RecentNote>> {
        let open = self.open()?;
        let mut notes = match state::load::<Recents>(&open.dir.join("recent.json"))? {
            Loaded::Ok(r) => r.notes,
            _ => Vec::new(),
        };
        notes.retain_mut(|n| {
            if let Some(r) = open.registry.record(n.note_id) {
                n.path = r.path.clone();
            }
            open.fs.stat(&n.path).is_ok()
        });
        Ok(notes)
    }
    pub(crate) fn touch_recent(&self, id: NoteId, path: &RelPath) -> Result<()> {
        let mut notes = self.recent_notes()?;
        notes.retain(|n| n.note_id != id);
        notes.insert(
            0,
            RecentNote {
                note_id: id,
                path: path.clone(),
                opened: crate::now(),
            },
        );
        notes.truncate(100);
        state::store(
            &self.open()?.dir.join("recent.json"),
            &Recents { schema: 1, notes },
        )
    }
}
