//! The preview as the core exposes it: what crosses the IPC, and what the
//! `notes-asset://` handler is allowed to read.
//!
//! The renderer's own behaviour is `notes-markdown`'s two corpora. What is
//! tested here is the part that only exists once a workspace does — the root
//! jail on the asset path, the per-workspace trust settings, and the shape of
//! what goes on the wire.

use notes_core::WorkspaceService;
use notes_model::{CoreError, RelPath};

struct Fixture {
    _data: tempfile::TempDir,
    work: tempfile::TempDir,
    svc: WorkspaceService,
}

/// A one-pixel PNG, so the bytes served are a real image rather than a label.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

fn setup() -> Fixture {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("nota.md"), b"# nota\n").unwrap();
    std::fs::write(work.path().join("imagem.png"), PNG).unwrap();
    std::fs::write(work.path().join("segredo.txt"), b"nao e uma imagem\n").unwrap();
    std::fs::create_dir(work.path().join("sub")).unwrap();
    std::fs::write(work.path().join("sub/figura.png"), PNG).unwrap();

    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(work.path()).unwrap();
    Fixture {
        _data: data,
        work,
        svc,
    }
}

fn rel(s: &str) -> RelPath {
    RelPath::parse(s).unwrap()
}

// ---------------------------------------------------------------------------
// The asset scheme is a second door into the workspace
// ---------------------------------------------------------------------------

#[test]
fn an_image_inside_the_workspace_is_served_with_its_type() {
    let f = setup();
    let a = f.svc.read_asset(&rel("imagem.png")).unwrap();
    assert_eq!(a.mime, "image/png");
    assert_eq!(a.bytes, PNG);
    assert_eq!(
        f.svc.read_asset(&rel("sub/figura.png")).unwrap().mime,
        "image/png"
    );
}

/// The one that matters. `RelPath` cannot express `..`, so the escape a real
/// note reaches for is a symlink — a well-formed relative path that resolves
/// somewhere else. The handler must not follow it, and must not follow it *here*
/// rather than only in `note_open`.
#[test]
#[cfg(unix)]
fn the_asset_path_obeys_the_same_root_jail_as_every_command() {
    let f = setup();
    let outside = f.work.path().parent().unwrap().join("fora.png");
    std::fs::write(&outside, PNG).unwrap();
    std::os::unix::fs::symlink(&outside, f.work.path().join("atalho.png")).unwrap();

    match f.svc.read_asset(&rel("atalho.png")) {
        Err(CoreError::SymlinkNotFollowed { .. }) => {}
        other => panic!("a symlink out of the root must be refused, got {other:?}"),
    }
    // …and the file it pointed at is untouched.
    assert_eq!(std::fs::read(&outside).unwrap(), PNG);
}

#[test]
fn a_path_that_is_not_an_image_is_refused_by_type_not_by_luck() {
    let f = setup();
    match f.svc.read_asset(&rel("segredo.txt")) {
        Err(CoreError::Unsupported { cap }) => assert!(cap.contains("txt"), "{cap}"),
        other => panic!("expected Unsupported, got {other:?}"),
    }
    // A note is not an image either: the preview cannot be used to read one
    // note into another.
    assert!(matches!(
        f.svc.read_asset(&rel("nota.md")),
        Err(CoreError::Unsupported { .. })
    ));
}

#[test]
fn an_asset_needs_an_open_workspace() {
    let data = tempfile::tempdir().unwrap();
    let svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    assert!(matches!(
        svc.read_asset(&rel("imagem.png")),
        Err(CoreError::NoWorkspace)
    ));
}

// ---------------------------------------------------------------------------
// What crosses the IPC
// ---------------------------------------------------------------------------

#[test]
fn the_preview_crosses_as_sanitized_html_and_the_outline_as_data() {
    let f = setup();
    let src = "# Um título\n\n<script>window.x=1</script>\n\n[nota](outra.md)\n";
    let r = f.svc.render_markdown(&rel("nota.md"), src).unwrap();

    assert!(!r.html.contains("<script"), "{}", r.html);
    assert!(r.html.contains("data-note-path=\"outra.md\""), "{}", r.html);
    assert_eq!(r.outline.len(), 1);
    assert_eq!(r.outline[0].slug, "um-título");

    let doc = f.svc.outline(src).unwrap();
    assert_eq!(doc.headings.len(), 1);
    assert_eq!(doc.links.len(), 1);
}

/// A note in a subfolder resolves its links from **its own** directory, not
/// from the workspace root.
#[test]
fn relative_links_resolve_from_the_notes_own_folder() {
    let f = setup();
    let r = f
        .svc
        .render_markdown(
            &rel("sub/nota.md"),
            "![f](figura.png)\n\n[cima](../nota.md)",
        )
        .unwrap();
    let id = f.svc.workspace_id().unwrap();
    assert!(
        r.html
            .contains(&format!("notes-asset://{id}/sub/figura.png")),
        "{}",
        r.html
    );
    assert!(r.html.contains("data-note-path=\"nota.md\""), "{}", r.html);
}

#[test]
fn remote_images_are_blocked_until_this_workspace_says_otherwise() {
    let mut f = setup();
    let src = "![t](https://example.invalid/t.png)";

    let blocked = f.svc.render_markdown(&rel("nota.md"), src).unwrap();
    assert_eq!(blocked.blocked_remote.len(), 1);
    assert!(!blocked.html.contains("<img"), "{}", blocked.html);

    f.svc.set_markdown_trust(None, Some(true)).unwrap();
    let allowed = f.svc.render_markdown(&rel("nota.md"), src).unwrap();
    assert!(allowed.blocked_remote.is_empty());
    assert!(allowed.html.contains("<img"), "{}", allowed.html);
}

#[test]
fn raw_html_is_off_until_this_workspace_says_otherwise_and_is_still_sanitized() {
    let mut f = setup();
    let src = "<b>negrito</b> e <script>window.x=1</script>";

    let escaped = f.svc.render_markdown(&rel("nota.md"), src).unwrap();
    assert!(escaped.html.contains("&lt;b&gt;"), "{}", escaped.html);

    f.svc.set_markdown_trust(Some(true), None).unwrap();
    let raw = f.svc.render_markdown(&rel("nota.md"), src).unwrap();
    assert!(raw.html.contains("<b>negrito</b>"), "{}", raw.html);
    assert!(!raw.html.contains("script"), "{}", raw.html);
}

/// The trust settings are per workspace and survive a restart, which is the
/// only reason they live in the registry rather than in memory.
#[test]
fn trust_is_remembered_for_this_workspace_and_not_for_another() {
    let f = setup();
    let other = tempfile::tempdir().unwrap();
    std::fs::write(other.path().join("nota.md"), b"# n\n").unwrap();

    let mut svc = f.svc;
    svc.set_markdown_trust(None, Some(true)).unwrap();
    let root = f.work.path().to_path_buf();
    let data = svc.data_dir().to_path_buf();
    drop(svc);

    let mut reopened = WorkspaceService::with_data_dir(&data).unwrap();
    reopened.open_workspace(&root).unwrap();
    let src = "![t](https://example.invalid/t.png)";
    assert!(
        reopened
            .render_markdown(&rel("nota.md"), src)
            .unwrap()
            .blocked_remote
            .is_empty(),
        "the setting must survive a restart"
    );

    reopened.open_workspace(other.path()).unwrap();
    assert_eq!(
        reopened
            .render_markdown(&rel("nota.md"), src)
            .unwrap()
            .blocked_remote
            .len(),
        1,
        "another workspace does not inherit this one's trust"
    );
}
