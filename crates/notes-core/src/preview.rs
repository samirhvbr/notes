//! Preview, outline, and the bytes behind `notes-asset://`.
//!
//! The rule this module exists to keep is `ARCHITECTURE.md` §10: **the frontend
//! has no Markdown parser**, so the preview is rendered here and crosses the IPC
//! as sanitized HTML. What crosses as data is the slim [`Document`] — the
//! outline, the links and the tasks — because those are what the frontend has to
//! *act* on.

use notes_fs::FileSystem;
use notes_model::{CoreError, RelPath};

/// A file served through the `notes-asset://` scheme.
///
/// `Debug` prints the length rather than the bytes: a megabyte of PNG in a
/// panic message helps nobody.
pub struct Asset {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
}

impl std::fmt::Debug for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Asset")
            .field("mime", &self.mime)
            .field("bytes", &format_args!("{} bytes", self.bytes.len()))
            .finish()
    }
}

/// What the preview may load from the workspace.
///
/// **Raster only.** An `<img>` cannot execute the script inside an SVG — the
/// specification forbids it in that context — but the format is a document
/// rather than a picture, and admitting it means relying on a browser rule
/// instead of on a decision. Scope §8.4 says "formatos suportados" without
/// naming them; this is that list, and adding `svg` is one line and an ADR
/// (`docs/DECISIONS-0.1b.md` D-08).
const IMAGE_TYPES: &[(&str, &str)] = &[
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("bmp", "image/bmp"),
    ("avif", "image/avif"),
];

/// Refuse to read a file larger than this into memory for a preview.
///
/// A note is allowed to reference a 2 GB file; a preview is not allowed to
/// allocate it. The limit is generous for a photograph and small enough that
/// the window cannot be brought down by a link.
const MAX_ASSET_BYTES: u64 = 64 * 1024 * 1024;

impl super::WorkspaceService {
    /// Sanitized HTML for a buffer the frontend is holding.
    ///
    /// The **text comes from the caller**, not from disk: the preview follows
    /// what is being typed, and reading the file here would render the last
    /// saved version instead (`ARCHITECTURE.md` §13, preview debounced 300 ms).
    /// `path` is only used to resolve relative links.
    pub fn render_markdown(&self, path: &RelPath, text: &str) -> super::Result<Rendered> {
        let open = self.open()?;
        let md = &self.settings.markdown;
        let opts = RenderOpts {
            base: path.parent().unwrap_or_else(RelPath::root),
            // Both default to off and are per workspace, overriding the global
            // when set — the same shape as every other setting here.
            raw_html: open.registry.settings.raw_html.unwrap_or(md.raw_html),
            remote_images: open
                .registry
                .settings
                .remote_images
                .unwrap_or(md.remote_images),
            workspace_id: open.id,
        };
        Ok(notes_markdown::render_html(text, &opts))
    }

    /// The outline, links and front-matter span, with no HTML rendered.
    pub fn outline(&self, text: &str) -> super::Result<Document> {
        // A workspace has to be open for the same reason `render_markdown`
        // needs one: a note belongs to a workspace, and answering without one
        // would be answering about nothing.
        self.open()?;
        Ok(notes_markdown::parse(text))
    }

    /// Turn raw HTML or remote images on or off **for this workspace**.
    ///
    /// Per workspace rather than globally because trusting the notes in one
    /// folder says nothing about another: a workspace synced from elsewhere and
    /// one the user has written by hand are the same application and different
    /// threat models (scope §8.4). `None` clears the override and falls back to
    /// the global setting, which is off.
    pub fn set_markdown_trust(
        &mut self,
        raw_html: Option<bool>,
        remote_images: Option<bool>,
    ) -> super::Result<()> {
        let dir = self.open()?.dir.clone();
        let open = self.open_mut()?;
        if open.read_only {
            return Err(CoreError::Unavailable {
                root: open.registry.root.clone(),
                reason: notes_model::UnavailableReason::PermissionRevoked,
            });
        }
        open.registry.settings.raw_html = raw_html;
        open.registry.settings.remote_images = remote_images;
        let registry = open.registry.clone();
        crate::store_registry(&dir, &registry)
    }

    /// The bytes behind a `notes-asset://` URL.
    ///
    /// **The same root jail as every other path**, applied here rather than
    /// trusted from the URL: the scheme handler is a second entry point into
    /// the workspace, and a second entry point that skips the check is the
    /// whole reason `notes-fs` re-resolves on every call.
    pub fn read_asset(&self, path: &RelPath) -> super::Result<Asset> {
        let open = self.open()?;
        let ext = path.extension().unwrap_or_default();
        let mime = IMAGE_TYPES
            .iter()
            .find(|(e, _)| *e == ext)
            .map(|(_, m)| *m)
            .ok_or_else(|| CoreError::Unsupported {
                cap: format!("preview of a .{ext} file"),
            })?;

        let stat = open.fs.stat(path)?;
        if stat.size > MAX_ASSET_BYTES {
            return Err(CoreError::Unsupported {
                cap: format!("an image of {} bytes", stat.size),
            });
        }
        Ok(Asset {
            bytes: open.fs.read(path)?,
            mime,
        })
    }
}

pub use notes_markdown::{Document, Heading, Link, LinkKind, RenderOpts, Rendered, Span, Task};
