use notes_core::WorkspaceService;
use notes_model::RelPath;
use std::{
    fs, thread,
    time::{Duration, Instant},
};
fn p(s: &str) -> RelPath {
    RelPath::parse(s).unwrap()
}
fn indexed(s: &mut WorkspaceService) {
    s.index_start(false).unwrap();
    let start = Instant::now();
    loop {
        let state = s.index_status().unwrap();
        if !state.running {
            assert!(state.error.is_none(), "{state:?}");
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(20));
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn tags_backlinks_and_ambiguity_share_the_parser() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("sub")).unwrap();
    fs::write(
        root.path().join("a.md"),
        "---\ntags: [Rust]\n---\n#visible `#hidden` [[Same]] [[Unique]] [other](sub/Same.md)\n",
    )
    .unwrap();
    for path in ["Same.md", "sub/Same.md", "Unique.md"] {
        fs::write(root.path().join(path), "note").unwrap();
    }
    let mut s = WorkspaceService::with_data_dir(data.path()).unwrap();
    s.open_workspace(root.path()).unwrap();
    indexed(&mut s);
    let k = s.knowledge().unwrap();
    let note = k.notes.iter().find(|n| n.path == p("a.md")).unwrap();
    assert_eq!(note.tags, vec!["rust", "visible"]);
    assert!(k
        .edges
        .iter()
        .any(|e| e.from == p("a.md") && e.to == p("Unique.md")));
    assert!(!k
        .edges
        .iter()
        .any(|e| e.from == p("a.md") && e.to == p("Same.md")));
    assert_eq!(k.unresolved[0].candidates.len(), 2);
    assert_eq!(s.wiki_candidates("./Same.md").unwrap(), vec![p("Same.md")]);
    // A newly created homonym must not produce a false unique graph edge
    // while the content index is still catching up.
    fs::write(root.path().join("sub/Unique.md"), "new homonym").unwrap();
    let pending = s.knowledge().unwrap();
    assert!(pending.partial);
    assert!(!pending.edges.iter().any(|e| e.to == p("Unique.md")));
    assert!(pending
        .unresolved
        .iter()
        .any(|l| l.target == "Unique" && l.candidates.len() == 2));
}
#[test]
fn reviewed_wiki_rename_preserves_alias_and_code() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("a.md"),
        "[[Old#heading|label]] `[[Old]]`\n",
    )
    .unwrap();
    fs::write(root.path().join("Old.md"), "# Heading\n").unwrap();
    let mut s = WorkspaceService::with_data_dir(data.path()).unwrap();
    s.open_workspace(root.path()).unwrap();
    indexed(&mut s);
    let plan = s.reference_preview(&p("Old.md"), &p("New.md")).unwrap();
    assert_eq!(plan.files.len(), 1);
    let result = s.reference_apply(&plan.token, &[0]).unwrap();
    assert!(result.failed.is_empty());
    assert_eq!(
        fs::read_to_string(root.path().join("a.md")).unwrap(),
        "[[./New.md#heading|label]] `[[Old]]`\n"
    );
}
#[test]
fn clipboard_import_validates_bytes_and_never_changes_the_note() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("sub")).unwrap();
    fs::write(root.path().join("sub/a.md"), "original").unwrap();
    let mut bytes = std::io::Cursor::new(vec![]);
    image::DynamicImage::new_rgb8(1, 1)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let bytes = bytes.into_inner();
    let mut s = WorkspaceService::with_data_dir(data.path()).unwrap();
    s.open_workspace(root.path()).unwrap();
    let a = s.import_attachment(&p("sub/a.md"), &bytes).unwrap();
    let b = s.import_attachment(&p("sub/a.md"), &bytes).unwrap();
    assert_ne!(a.path, b.path);
    assert!(a.markdown.contains("../attachments/"));
    assert_eq!(fs::read(root.path().join(a.path.as_str())).unwrap(), bytes);
    assert!(s.import_attachment(&p("sub/a.md"), b"<svg></svg>").is_err());
    assert_eq!(
        fs::read_to_string(root.path().join("sub/a.md")).unwrap(),
        "original"
    );
}
