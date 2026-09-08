//! `fixtures/xss/` as a test suite.
//!
//! The corpus was committed at 0.1a with a README that called each file *"an
//! assertion, not a sample"*, and nothing read it — the crate that would arrive
//! at 0.1b. This is that crate reading it: **every file in the folder is a test
//! that fails if a payload escapes**, and a file added to the folder without a
//! test here fails the census at the bottom.
//!
//! Assertions are structural. `safe-in-code.md` has to render
//! `javascript:alert(1)` as text, so a suite that greps the HTML for
//! `javascript:` would demand the opposite of what the corpus requires; the
//! checks below read tags and attributes instead (`tests/html.rs`).

mod html;

use notes_markdown::{render_html, RenderOpts};
use notes_model::{RelPath, WorkspaceId};
use std::path::PathBuf;

const WORKSPACE: &str = "00000000-0000-0000-0000-000000000000";

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/xss")
}

/// The corpus is a folder **inside** a workspace, not a workspace: these notes
/// live at `xss/`, and `safe-links.md` links `../basic/nota-000.md` precisely to
/// prove that a relative path may climb one level and still be inside the root.
fn opts(remote_images: bool, raw_html: bool) -> RenderOpts {
    RenderOpts {
        base: RelPath::parse("xss").unwrap(),
        raw_html,
        remote_images,
        workspace_id: WorkspaceId::from_uuid(WORKSPACE.parse().unwrap()),
    }
}

fn render(name: &str, remote_images: bool, raw_html: bool) -> String {
    let src = std::fs::read_to_string(dir().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    render_html(&src, &opts(remote_images, raw_html)).html
}

/// Nothing in this list may appear as a **tag** in rendered output, under any
/// option. Each one is either an execution vector, a navigation vector or a
/// resource loader, and a note is untrusted content (scope §8.4).
const FORBIDDEN_TAGS: &[&str] = &[
    "script", "iframe", "frame", "frameset", "object", "embed", "applet", "form", "button",
    "textarea", "select", "option", "meta", "base", "link", "style", "svg", "math", "template",
    "noscript", "audio", "video", "source", "track", "canvas", "portal", "marquee", "body", "head",
    "html", "title", "dialog",
];

/// The only schemes an `href` or `src` may carry. Everything else — `file:`,
/// `javascript:`, `vbscript:`, an unknown scheme — is dropped by the rewrite
/// pass and again by `ammonia`.
const ALLOWED_SCHEMES: &[&str] = &["http", "https", "mailto", "notes-asset", "data"];

const TABLE_ALIGNMENTS: &[&str] = &[
    "text-align: left",
    "text-align: center",
    "text-align: right",
];

/// The invariants every file in the corpus must satisfy, under both settings of
/// `raw_html` and both of `remote_images`.
fn assert_safe(name: &str, rendered: &str, remote_images: bool) {
    for tag in html::tags(rendered) {
        assert!(
            !FORBIDDEN_TAGS.contains(&tag.name.as_str()),
            "{name}: <{}> survived rendering\n{rendered}",
            tag.name
        );

        for (attr, value) in &tag.attrs {
            // An event handler is script by another name.
            assert!(
                !attr.starts_with("on"),
                "{name}: {}[{attr}] survived rendering\n{rendered}",
                tag.name
            );
            assert!(
                !matches!(
                    attr.as_str(),
                    "srcdoc" | "formaction" | "xlink:href" | "action"
                ),
                "{name}: {}[{attr}] survived rendering\n{rendered}",
                tag.name
            );

            if attr == "style" {
                assert!(
                    matches!(tag.name.as_str(), "th" | "td")
                        && TABLE_ALIGNMENTS.contains(&value.trim()),
                    "{name}: a style attribute survived on <{}>: {value:?}\n{rendered}",
                    tag.name
                );
            }

            if matches!(attr.as_str(), "href" | "src") {
                match html::scheme(value) {
                    None => {
                        // A relative URL. It must not be protocol-relative,
                        // which is a remote URL wearing the shape of a path.
                        assert!(
                            !value.starts_with("//"),
                            "{name}: protocol-relative URL {value:?}\n{rendered}"
                        );
                    }
                    Some(s) => {
                        assert!(
                            ALLOWED_SCHEMES.contains(&s.as_str()),
                            "{name}: {attr}={value:?} carries the scheme {s:?}\n{rendered}"
                        );
                        if s == "data" {
                            // Only raster images, never `image/svg+xml`, which
                            // is a scriptable document.
                            assert!(
                                value.starts_with("data:image/png;base64,")
                                    || value.starts_with("data:image/jpeg;base64,")
                                    || value.starts_with("data:image/gif;base64,")
                                    || value.starts_with("data:image/webp;base64,"),
                                "{name}: a data: URL outside the raster allowlist\n{rendered}"
                            );
                        }
                        if s == "notes-asset" {
                            assert!(
                                value.starts_with(&format!("notes-asset://{WORKSPACE}/")),
                                "{name}: an asset URL for another workspace: {value:?}"
                            );
                            assert!(
                                !value.contains(".."),
                                "{name}: an asset URL climbing out: {value:?}"
                            );
                        }
                        if !remote_images && matches!(s.as_str(), "http" | "https") && attr == "src"
                        {
                            panic!("{name}: a remote resource is loaded with remote_images off: {value:?}\n{rendered}");
                        }
                    }
                }
            }
        }

        // The only `<input>` that can exist is a checkbox with nothing to
        // send: a preview submits nothing and accepts nothing. `<form>` and
        // `<button>` are refused outright, so there is nowhere for one to
        // submit to even if it carried a value.
        if tag.name == "input" && !tag.closing {
            assert_eq!(tag.attr("type"), Some("checkbox"), "{name}: {rendered}");
            for forbidden in ["name", "value", "form", "formaction", "src"] {
                assert!(
                    !tag.has(forbidden),
                    "{name}: an input carrying {forbidden}\n{rendered}"
                );
            }
        }
    }
}

/// The census: every `.md` in the folder is rendered under all four
/// combinations of the two options and must satisfy the invariants.
///
/// This is what makes adding a payload to the corpus enough — no test has to be
/// written for it, and forgetting to write one cannot make it pass.
#[test]
fn every_file_in_the_corpus_is_safe_under_every_setting() {
    let files = corpus();
    assert!(
        files.len() >= 17,
        "the corpus shrank to {} files — payloads are not deleted, they are fixed",
        files.len()
    );
    for name in &files {
        for remote_images in [false, true] {
            for raw_html in [false, true] {
                let out = render(name, remote_images, raw_html);
                assert_safe(name, &out, remote_images);
            }
        }
    }
    eprintln!("{} payload files, 4 settings each", files.len());
}

fn corpus() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir())
        .expect("fixtures/xss must exist")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".md") && n != "README.md")
        .collect();
    v.sort();
    v
}

// ---------------------------------------------------------------------------
// One test per file. The census above proves nothing executes; these prove each
// payload was refused for the right reason, and that the rest of the note
// still rendered — "bloquear recurso não impede ler o resto da nota"
// (scope §8.4).
// ---------------------------------------------------------------------------

fn assert_body_survived(name: &str, out: &str) {
    assert!(
        out.contains("<h1"),
        "{name}: the heading must still render after the payload is refused\n{out}"
    );
}

#[test]
fn script_tag_becomes_text_and_the_note_continues() {
    let out = render("script-tag.md", false, false);
    assert_body_survived("script-tag.md", &out);
    assert!(out.contains("&lt;script&gt;"), "{out}");
    assert!(out.contains("Texto depois."), "{out}");
}

#[test]
fn an_img_onerror_loses_the_handler() {
    let out = render("img-onerror.md", false, false);
    for tag in html::tags(&out) {
        assert!(!tag.has("onerror"), "{out}");
    }
    // The payload is still *legible* — escaped into text, not deleted. A note
    // the application silently rewrites is a note the user cannot trust.
    assert!(out.contains("&lt;img src=\"x\" onerror="), "{out}");
    assert_body_survived("img-onerror.md", &out);
}

#[test]
fn an_svg_onload_does_not_render_an_svg_at_all() {
    let out = render("svg-onload.md", false, false);
    assert!(!out.to_lowercase().contains("<svg"), "{out}");
}

#[test]
fn a_javascript_link_keeps_its_text_and_loses_its_href() {
    let out = render("js-link.md", false, false);
    for tag in html::tags(&out) {
        assert!(tag.name != "a" || tag.closing, "a link survived:\n{out}");
    }
    assert!(out.contains("clique"), "the words must survive\n{out}");
    assert!(out.contains("clique tambem"), "{out}");
}

#[test]
fn a_data_uri_link_is_dropped() {
    let out = render("data-uri-link.md", false, false);
    for tag in html::tags(&out) {
        assert!(tag.attr("href").is_none(), "a data: link survived\n{out}");
    }
}

#[test]
fn an_iframe_is_never_a_frame() {
    let out = render("iframe.md", false, false);
    assert!(!out.contains("<iframe"), "{out}");
    assert!(!out.contains("<frame"), "{out}");
}

#[test]
fn a_form_cannot_submit_anything() {
    for raw_html in [false, true] {
        let out = render("form.md", false, raw_html);
        for tag in html::tags(&out) {
            assert!(tag.name != "form" && tag.name != "button", "{out}");
            if tag.name == "input" && !tag.closing {
                // Even with raw HTML on, the control that survives is a
                // checkbox with no name and no value: there is nothing to send
                // and nowhere to send it.
                assert_eq!(tag.attr("type"), Some("checkbox"), "{out}");
                assert!(!tag.has("name") && !tag.has("value"), "{out}");
            }
        }
    }
}

/// The task-list checkbox is disabled, which is the shape the whole preview is:
/// something to read, not something to operate.
#[test]
fn a_task_list_checkbox_is_disabled() {
    let out = notes_markdown::render_html("- [ ] a\n- [x] b\n", &opts(false, false)).html;
    let inputs: Vec<_> = html::tags(&out)
        .into_iter()
        .filter(|t| t.name == "input" && !t.closing)
        .collect();
    assert_eq!(inputs.len(), 2, "{out}");
    for i in &inputs {
        assert_eq!(i.attr("type"), Some("checkbox"), "{out}");
        assert!(i.has("disabled"), "{out}");
    }
    assert!(!inputs[0].has("checked"), "{out}");
    assert!(inputs[1].has("checked"), "{out}");
}

#[test]
fn object_and_embed_load_no_plugin() {
    let out = render("object-embed.md", false, false);
    for tag in html::tags(&out) {
        assert!(!matches!(tag.name.as_str(), "object" | "embed"), "{out}");
    }
}

#[test]
fn meta_refresh_and_base_cannot_navigate_the_webview() {
    let out = render("meta-refresh.md", false, false);
    for tag in html::tags(&out) {
        assert!(!matches!(tag.name.as_str(), "meta" | "base"), "{out}");
    }
}

#[test]
fn a_style_block_and_an_inline_style_both_go() {
    let out = render("style-injection.md", false, false);
    for tag in html::tags(&out) {
        assert!(tag.name != "style", "{out}");
        assert!(
            !tag.has("style") || matches!(tag.name.as_str(), "th" | "td"),
            "{out}"
        );
    }
    // The URL survives as *text*, which is the correct outcome: nothing
    // fetches it, and deleting it would be rewriting the note.
    for tag in html::tags(&out) {
        for (_, value) in &tag.attrs {
            assert!(
                !value.contains("example.invalid"),
                "a beacon URL reached an attribute\n{out}"
            );
        }
    }
}

#[test]
fn event_handlers_on_otherwise_allowed_tags_are_stripped() {
    let out = render("event-handlers.md", false, false);
    for tag in html::tags(&out) {
        for (attr, _) in &tag.attrs {
            assert!(!attr.starts_with("on"), "{out}");
        }
    }
    // With raw_html on, the <a> is allowed to survive — without its handler.
    let raw = render("event-handlers.md", false, true);
    for tag in html::tags(&raw) {
        for (attr, _) in &tag.attrs {
            assert!(!attr.starts_with("on"), "{raw}");
        }
    }
}

#[test]
fn a_tab_inside_a_scheme_does_not_smuggle_it_through() {
    let out = render("mixed-case-and-entities.md", false, false);
    assert!(!out.to_lowercase().contains("<script"), "{out}");
    for tag in html::tags(&out) {
        if let Some(href) = tag.attr("href") {
            let s = html::scheme(href).unwrap_or_default();
            assert!(
                s.is_empty() || ALLOWED_SCHEMES.contains(&s.as_str()),
                "a smuggled scheme survived: {href:?}\n{out}"
            );
        }
    }
}

#[test]
fn a_file_scheme_image_is_refused_on_both_platforms_spellings() {
    let out = render("file-image.md", false, false);
    for tag in html::tags(&out) {
        for (_, value) in &tag.attrs {
            assert!(
                !value.to_lowercase().starts_with("file:"),
                "a file: URL reached an attribute\n{out}"
            );
        }
    }
    for tag in html::tags(&out) {
        assert!(tag.name != "img" || tag.closing, "{out}");
    }
}

#[test]
fn an_image_escaping_the_root_is_refused_rather_than_resolved() {
    let out = render("escaping-image.md", false, false);
    assert!(!out.contains("passwd"), "{out}");
    for tag in html::tags(&out) {
        assert!(tag.attr("src").is_none(), "{out}");
    }
}

#[test]
fn a_remote_image_is_blocked_by_default_and_named() {
    let src = std::fs::read_to_string(dir().join("remote-image.md")).unwrap();
    let blocked = render_html(&src, &opts(false, false));
    assert_eq!(
        blocked.blocked_remote,
        vec![
            "https://example.invalid/tracker.png",
            // `//example.invalid/…` is protocol-relative: a remote URL wearing
            // the shape of a path. It is resolved to `https:` and blocked, not
            // treated as a file in the workspace.
            "https://example.invalid/tracker.png",
        ],
        "both the absolute and the protocol-relative form are remote"
    );
    assert!(!blocked.html.contains("<img"), "{}", blocked.html);
    assert!(
        blocked.html.contains("example.invalid/tracker.png"),
        "the URL is shown as text so the gap is not silent\n{}",
        blocked.html
    );

    // Opted in, it becomes an image — and nothing else changes.
    let allowed = render_html(&src, &opts(true, false));
    assert!(allowed.blocked_remote.is_empty());
    assert!(allowed.html.contains("<img"), "{}", allowed.html);
}

#[test]
fn the_same_payloads_inside_code_render_as_text() {
    let out = render("safe-in-code.md", false, false);
    // The block payload is text, not markup…
    assert!(
        out.contains("&lt;script&gt;window.__pwned = 1&lt;/script&gt;"),
        "{out}"
    );
    assert!(out.contains("&lt;img src=\"x\" onerror="), "{out}");
    // …and the inline one keeps its literal `javascript:`, which is exactly
    // what a suite of substring bans would have destroyed.
    assert!(out.contains("javascript:alert(1)"), "{out}");
    // Both live inside <code>, and no <a> or <img> was produced.
    assert!(out.contains("<code"), "{out}");
    for tag in html::tags(&out) {
        assert!(
            !matches!(tag.name.as_str(), "a" | "img" | "script"),
            "code was rendered as markup\n{out}"
        );
    }
}

#[test]
fn the_links_that_must_survive_do() {
    let out = render("safe-links.md", false, false);
    let tags = html::tags(&out);

    let note = tags
        .iter()
        .find(|t| t.attr("data-note-path").is_some())
        .unwrap_or_else(|| panic!("the relative note link must survive\n{out}"));
    assert_eq!(note.attr("data-note-path"), Some("basic/nota-000.md"));

    let anchor = tags
        .iter()
        .find(|t| t.attr("href") == Some("#uma-secao"))
        .unwrap_or_else(|| panic!("the anchor must survive\n{out}"));
    assert!(anchor.attr("target").is_none(), "an anchor opens no window");

    let external = tags
        .iter()
        .find(|t| t.attr("href") == Some("https://example.com/pagina"))
        .unwrap_or_else(|| panic!("the external link must survive\n{out}"));
    assert_eq!(external.attr("target"), Some("_blank"));
    assert_eq!(external.attr("rel"), Some("noopener noreferrer"));

    let img = tags
        .iter()
        .find(|t| t.name == "img")
        .unwrap_or_else(|| panic!("the relative image must survive\n{out}"));
    assert_eq!(
        img.attr("src"),
        Some(format!("notes-asset://{WORKSPACE}/xss/imagem.png").as_str())
    );
}
