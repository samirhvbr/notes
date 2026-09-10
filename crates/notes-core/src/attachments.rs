//! Clipboard images enter the workspace through the same confined filesystem.
use crate::{Result, WorkspaceService};
use notes_fs::FileSystem;
use notes_model::{CoreError, RelPath, WorkspaceId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Attachment {
    pub path: RelPath,
    pub markdown: String,
}
impl WorkspaceService {
    pub fn import_attachment(&mut self, note: &RelPath, bytes: &[u8]) -> Result<Attachment> {
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(CoreError::Unsupported {
                cap: "clipboard image over 8 MiB".into(),
            });
        }
        let format = image::guess_format(bytes).map_err(|_| CoreError::Unsupported {
            cap: "invalid clipboard image".into(),
        })?;
        let ext = match format {
            image::ImageFormat::Png => "png",
            image::ImageFormat::Jpeg => "jpg",
            _ => {
                return Err(CoreError::Unsupported {
                    cap: "clipboard image must be PNG or JPEG".into(),
                })
            }
        };
        let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        reader.decode().map_err(|_| CoreError::Unsupported {
            cap: "invalid or oversized clipboard image".into(),
        })?;
        let open = self.open()?;
        if open.read_only {
            return Err(CoreError::Unsupported {
                cap: "attachment in a read-only workspace".into(),
            });
        }
        if !note.is_note() || open.fs.stat(note)?.kind != notes_model::EntryKind::File {
            return Err(CoreError::Unsupported {
                cap: "attachment target must be a Markdown note".into(),
            });
        }
        let directory = RelPath::parse("attachments")?;
        match open.fs.stat(&directory) {
            Ok(_) => {}
            Err(
                CoreError::NotFound { .. }
                | CoreError::Io {
                    kind: notes_model::IoKind::NotFound,
                    ..
                },
            ) => open.fs.create_dir(&directory)?,
            Err(e) => return Err(e),
        }
        let path = directory.join(&format!("image-{}.{}", WorkspaceId::new(), ext))?;
        open.fs.create_new(&path, bytes)?;
        let base = note.parent().unwrap_or_else(RelPath::root);
        let markdown = format!(
            "![image]({})",
            notes_markdown::rewrite::relative(&base, &path)
        );
        self.invalidate_paths();
        Ok(Attachment { path, markdown })
    }
}
