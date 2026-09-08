//! `fixtures/markdown/` as a golden corpus.
//!
//! Each `NAME.md` has two files beside it, and both are compared **byte for
//! byte** with no normalisation on either side:
//!
//! - `NAME.html` — what `render_html` must produce;
//! - `NAME.doc.json` — what `parse` must report: the front-matter span, the
//!   outline with its slugs, the link inventory and the task list.
//!
//! ```bash
//! cargo test -p notes-markdown                 # compare
//! NOTES_BLESS=1 cargo test -p notes-markdown   # rewrite from the renderer
//! ```
//!
//! **Blessing is not accepting.** `NOTES_BLESS=1` writes what the renderer
//! currently does; the diff is then read against `fixtures/markdown/README.md`
//! before it is committed. A golden blessed without being read records a bug as
//! a decision, which is the only failure mode a corpus like this has
//! (`docs/DECISIONS-0.1b.md` D-04).

mod html;

use notes_markdown::{parse, render_html, RenderOpts};
use notes_model::{RelPath, WorkspaceId};
use std::path::{Path, PathBuf};

const WORKSPACE: &str = "00000000-0000-0000-0000-000000000000";

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/markdown")
}

/// Fixed, so the corpus is reproducible on any machine — a random workspace id
/// would change every `notes-asset://` URL in every golden file.
fn opts() -> RenderOpts {
    RenderOpts {
        base: RelPath::root(),
        raw_html: false,
        remote_images: false,
        workspace_id: WorkspaceId::from_uuid(WORKSPACE.parse().unwrap()),
    }
}

fn blessing() -> bool {
    std::env::var_os("NOTES_BLESS").is_some()
}

fn compare(path: &Path, produced: &str) -> Option<String> {
    if blessing() {
        std::fs::write(path, produced).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        return None;
    }
    let expected = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return Some(format!(
                "{} is missing ({e}). Run NOTES_BLESS=1 and read the diff.",
                path.display()
            ))
        }
    };
    if expected == produced {
        return None;
    }
    Some(format!(
        "{} differs.\n--- expected ---\n{expected}\n--- produced ---\n{produced}\n\
         Run NOTES_BLESS=1 and read the diff before committing it.",
        path.display()
    ))
}

fn inputs() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir())
        .expect("fixtures/markdown must exist")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".md") && n != "README.md")
        .collect();
    v.sort();
    v
}

#[test]
fn every_fixture_renders_to_its_golden_html() {
    let mut failures = Vec::new();
    for name in inputs() {
        let src = std::fs::read_to_string(dir().join(&name)).unwrap();
        let rendered = render_html(&src, &opts());
        let target = dir().join(name.replace(".md", ".html"));
        if let Some(f) = compare(&target, &rendered.html) {
            failures.push(f);
        }
    }
    assert!(failures.is_empty(), "\n\n{}", failures.join("\n\n"));
}

#[test]
fn every_fixture_parses_to_its_golden_document() {
    let mut failures = Vec::new();
    for name in inputs() {
        let src = std::fs::read_to_string(dir().join(&name)).unwrap();
        let doc = parse(&src);
        let mut json = serde_json::to_string_pretty(&doc).unwrap();
        json.push('\n');
        let target = dir().join(name.replace(".md", ".doc.json"));
        if let Some(f) = compare(&target, &json) {
            failures.push(f);
        }
    }
    assert!(failures.is_empty(), "\n\n{}", failures.join("\n\n"));
}

/// A fixture with no golden beside it, or a golden with no fixture, is a hole
/// in the corpus that nothing else would notice.
#[test]
fn the_corpus_has_no_orphans() {
    let inputs = inputs();
    for name in &inputs {
        for suffix in [".html", ".doc.json"] {
            let g = dir().join(name.replace(".md", suffix));
            assert!(g.exists(), "{} has no {suffix}", name);
        }
    }
    let orphans: Vec<String> = std::fs::read_dir(dir())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".html") || n.ends_with(".doc.json"))
        .filter(|n| {
            let stem = n.trim_end_matches(".html").trim_end_matches(".doc.json");
            !inputs.contains(&format!("{stem}.md"))
        })
        .collect();
    assert!(orphans.is_empty(), "goldens with no fixture: {orphans:?}");
}

/// The corpus is prose, not payloads — but the same invariants hold, and a
/// golden file that was blessed carelessly is exactly how one would stop
/// holding without anyone noticing.
#[test]
fn the_prose_corpus_obeys_the_same_rules_as_the_payload_corpus() {
    for name in inputs() {
        let src = std::fs::read_to_string(dir().join(&name)).unwrap();
        let out = render_html(&src, &opts()).html;
        for tag in html::tags(&out) {
            assert!(
                !matches!(
                    tag.name.as_str(),
                    "script" | "style" | "iframe" | "object" | "embed" | "form" | "meta" | "base"
                ),
                "{name}: <{}>\n{out}",
                tag.name
            );
            for (attr, value) in &tag.attrs {
                assert!(!attr.starts_with("on"), "{name}: {attr}\n{out}");
                if matches!(attr.as_str(), "href" | "src") {
                    if let Some(s) = html::scheme(value) {
                        assert!(
                            ["http", "https", "mailto", "notes-asset"].contains(&s.as_str()),
                            "{name}: {attr}={value:?}\n{out}"
                        );
                        assert!(
                            attr != "src" || s == "notes-asset",
                            "{name}: a remote resource with remote_images off\n{out}"
                        );
                    }
                }
            }
        }
    }
}
