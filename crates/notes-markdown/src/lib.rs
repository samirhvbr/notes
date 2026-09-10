//! `notes-markdown` — the only semantic authority over Markdown in this
//! application (scope §8.1).
//!
//! Preview, outline, links and front matter all come from here. **The frontend
//! contains no Markdown parser**, and the thing that crosses the IPC for the
//! preview is sanitized HTML rather than an AST (`ARCHITECTURE.md` §10,
//! ADR item 6): a second parser in TypeScript would be a second security policy,
//! and the two would drift.
//!
//! Zero I/O and no Tauri. Everything here is `&str` in, values out, which is
//! what lets `fixtures/xss/` be a test suite rather than a manual review.
//!
//! ## The two layers, and why there are two
//!
//! 1. **The rewrite pass** ([`url`]) decides what every destination is allowed
//!    to become, on the way through the parser. It is where the policy lives.
//! 2. **`ammonia`** runs over the finished HTML with a closed allowlist of tags
//!    and attributes. It knows nothing about notes and re-derives nothing.
//!
//! Either one alone would be enough on a good day. Together, a mistake in the
//! rewrite pass has to coincide with a hole in the allowlist to reach a user.

pub mod rewrite;
mod slug;
pub mod url;

use notes_model::{RelPath, WorkspaceId};
use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use url::LinkKind;
use url::{ImagePolicy, LinkPolicy};

/// A byte range in the note's source, for the editor to scroll to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Span {
    #[ts(type = "number")]
    pub start: usize,
    #[ts(type = "number")]
    pub end: usize,
}

impl From<std::ops::Range<usize>> for Span {
    fn from(r: std::ops::Range<usize>) -> Self {
        Span {
            start: r.start,
            end: r.end,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub slug: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Link {
    /// Exactly what the note says, before any resolution. 0.2's rename tool
    /// rewrites this string, so it must be the one on disk.
    pub target: String,
    pub kind: LinkKind,
    pub span: Span,
    /// True for a link-shaped string inside a code span or code block.
    ///
    /// Reported and **never rewritten** (`ARCHITECTURE.md` §10): the 0.2 rename
    /// tool needs to know these exist so it can say what it did not touch, and
    /// a renderer that turned them into links would be rewriting what the user
    /// wrote.
    pub in_code: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Task {
    pub checked: bool,
    pub span: Span,
}

/// Everything the application knows about a note without rendering it.
///
/// The slim half of the IR: the outline, the links and the task list cross the
/// IPC as data, because the frontend needs to *act* on them. The tree of blocks
/// does not, because the frontend only needs to *show* it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Document {
    /// The byte span of the YAML front matter, delimiters included, when the
    /// note opens with one. **The bytes are not parsed here and never
    /// rewritten** — front matter survives because nothing touches the buffer
    /// (`ARCHITECTURE.md` §5.1), not because this crate preserves it. Reading
    /// it is the 0.3 index's job.
    pub front_matter: Option<Span>,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
    pub tasks: Vec<Task>,
    /// `#tag` and YAML tags, from 0.3. Always empty today, and named now so
    /// that arriving there is not a wire change.
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RenderOpts {
    /// The directory the note is in. Relative destinations resolve from here,
    /// not from the workspace root.
    pub base: RelPath,
    /// Render raw HTML instead of escaping it. Off by default and per
    /// workspace; even when on, everything still goes through `ammonia`
    /// (scope §8.4).
    pub raw_html: bool,
    /// Fetch remote images. Off by default: a note that loads a remote image
    /// tells its author's server that the note was opened.
    pub remote_images: bool,
    /// Carried into `notes-asset://` URLs so the scheme handler knows which
    /// root to jail the request to.
    pub workspace_id: WorkspaceId,
}

impl RenderOpts {
    pub fn for_note(path: &RelPath, workspace_id: WorkspaceId) -> Self {
        Self {
            base: path.parent().unwrap_or_else(RelPath::root),
            raw_html: false,
            remote_images: false,
            workspace_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Rendered {
    /// Sanitized. This is what the frontend assigns to `innerHTML`, and the
    /// reason it is allowed to.
    pub html: String,
    pub outline: Vec<Heading>,
    /// Remote images that were blocked, in document order, so the UI can offer
    /// to turn them on for this workspace rather than leaving the user to guess
    /// why a picture is missing.
    pub blocked_remote: Vec<String>,
}

fn options(src: &str) -> Options {
    // The 0.1 profile, and only it (scope §8.1): CommonMark plus the four GFM
    // extensions, plus footnotes as the documented extension. "Everything
    // GitHub renders" is not a specification.
    let base = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;

    // **The metadata extension is enabled only when the note actually opens
    // with `---`.** `pulldown-cmark` will otherwise treat *any* `---`-fenced
    // block as front matter, wherever it appears, and swallow it — so a note
    // with a thematic break followed by a `key: value` line would silently lose
    // a paragraph. Scope §8.2 is explicit that front matter is YAML *"entre
    // `---` na primeira linha"*, and `fixtures/markdown/front-matter-not-first.md`
    // is that case.
    if starts_with_front_matter(src) {
        base | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
    } else {
        base
    }
}

fn starts_with_front_matter(src: &str) -> bool {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    matches!(src.strip_prefix("---"), Some(rest) if rest.starts_with('\n') || rest.starts_with("\r\n"))
}

/// The outline, the links and the front-matter span. No HTML, no allocation of
/// a rendered string — this is what the sidebar and the 0.2 rename tool call.
pub fn parse(src: &str) -> Document {
    analyse(src).0
}

/// Sanitized HTML for the preview, plus the outline that came free with it.
pub fn render_html(src: &str, opts: &RenderOpts) -> Rendered {
    let (doc, events) = analyse(src);
    let mut blocked_remote = Vec::new();
    let rewritten = rewrite(&events, &doc, opts, &mut blocked_remote);

    let mut raw = String::with_capacity(src.len() * 2);
    pulldown_cmark::html::push_html(&mut raw, rewritten.into_iter());

    Rendered {
        html: sanitize(&raw),
        outline: doc.headings,
        blocked_remote,
    }
}

// ---------------------------------------------------------------------------
// Pass one: what the note says
// ---------------------------------------------------------------------------

type Spanned<'a> = (Event<'a>, std::ops::Range<usize>);

fn analyse(src: &str) -> (Document, Vec<Spanned<'_>>) {
    let events: Vec<Spanned<'_>> = Parser::new_ext(src, options(src))
        .into_offset_iter()
        .collect();

    let mut doc = Document {
        front_matter: None,
        headings: Vec::new(),
        links: Vec::new(),
        tasks: Vec::new(),
        tags: Vec::new(),
    };
    let mut slugger = slug::Slugger::new();
    // Heading text is only known once the heading has ended, so it is
    // accumulated rather than read.
    let mut heading: Option<(u8, std::ops::Range<usize>, String)> = None;
    let base = RelPath::root();

    for (event, range) in &events {
        match event {
            // Belt and braces beside `options()`: front matter is the block at
            // the top of the file and nothing else.
            Event::Start(Tag::MetadataBlock(_)) if range.start == 0 => {
                doc.front_matter = Some(range.clone().into());
            }
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some((*level as u8, range.clone(), String::new()));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, span, text)) = heading.take() {
                    let text = text.trim().to_string();
                    doc.headings.push(Heading {
                        level,
                        slug: slugger.slug(&text),
                        text,
                        span: span.into(),
                    });
                }
            }
            Event::Start(Tag::Link {
                dest_url,
                link_type,
                ..
            })
            | Event::Start(Tag::Image {
                dest_url,
                link_type,
                ..
            }) => {
                let (kind, _) = classify(&base, *link_type, dest_url);
                doc.links.push(Link {
                    target: dest_url.to_string(),
                    kind,
                    span: range.clone().into(),
                    in_code: false,
                });
            }
            Event::TaskListMarker(checked) => doc.tasks.push(Task {
                checked: *checked,
                span: range.clone().into(),
            }),
            Event::Text(t) => {
                if let Some((_, _, acc)) = heading.as_mut() {
                    acc.push_str(t);
                }
                for r in autolinks(t) {
                    doc.links.push(Link {
                        target: t[r.start..r.end].to_string(),
                        kind: LinkKind::Url,
                        span: Span {
                            start: range.start + r.start,
                            end: range.start + r.end,
                        },
                        in_code: false,
                    });
                }
            }
            Event::Code(t) => {
                if let Some((_, _, acc)) = heading.as_mut() {
                    acc.push_str(t);
                }
                collect_links_in_code(t, range.start, &base, &mut doc.links);
            }
            Event::Start(Tag::CodeBlock(_)) => {}
            _ => {}
        }
    }

    // Code blocks arrive as `Text` inside a `CodeBlock`, which the loop above
    // treats as prose. Walking them separately keeps that loop readable and
    // costs one more pass over what is usually a small part of the note.
    collect_links_in_code_blocks(&events, &base, &mut doc.links);
    doc.links.sort_by_key(|l| (l.span.start, l.span.end));

    (doc, events)
}

/// Find link-shaped strings inside code, so `Document.links` is a complete
/// inventory rather than only the ones that render.
fn collect_links_in_code_blocks(events: &[Spanned<'_>], base: &RelPath, out: &mut Vec<Link>) {
    let mut depth = 0usize;
    for (event, range) in events {
        match event {
            Event::Start(Tag::CodeBlock(_)) => depth += 1,
            Event::End(TagEnd::CodeBlock) => depth = depth.saturating_sub(1),
            Event::Text(t) if depth > 0 => collect_links_in_code(t, range.start, base, out),
            _ => {}
        }
    }
}

/// `[text](destination)`, found by hand.
///
/// Deliberately not a Markdown parse: the point is to notice the *shape* a
/// human would call a link, in text the renderer will never turn into one.
fn collect_links_in_code(text: &str, offset: usize, base: &RelPath, out: &mut Vec<Link>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while let Some(open) = bytes[i..].iter().position(|b| *b == b'[').map(|p| p + i) {
        let Some(close) = bytes[open..]
            .iter()
            .position(|b| *b == b']')
            .map(|p| p + open)
        else {
            break;
        };
        if bytes.get(close + 1) != Some(&b'(') {
            i = close + 1;
            continue;
        }
        let Some(end) = bytes[close + 2..]
            .iter()
            .position(|b| *b == b')')
            .map(|p| p + close + 2)
        else {
            break;
        };
        let target = &text[close + 2..end];
        let (kind, _) = url::classify_link(base, target);
        out.push(Link {
            target: target.to_string(),
            kind,
            span: Span {
                start: offset + open,
                end: offset + end + 1,
            },
            in_code: true,
        });
        i = end + 1;
    }
}

// ---------------------------------------------------------------------------
// Pass two: what the note is allowed to become
// ---------------------------------------------------------------------------

fn rewrite<'a>(
    events: &[Spanned<'a>],
    doc: &Document,
    opts: &RenderOpts,
    blocked: &mut Vec<String>,
) -> Vec<Event<'a>> {
    let mut out: Vec<Event<'a>> = Vec::with_capacity(events.len());
    let mut headings = doc.headings.iter();
    let mut i = 0;
    // A refused link still shows its text; the stack remembers whether the
    // matching `End` has a tag to close.
    let mut open_links: Vec<bool> = Vec::new();
    let mut in_metadata = false;
    let mut code_depth = 0usize;
    let mut link_depth = 0usize;

    while i < events.len() {
        let event = &events[i].0;
        i += 1;
        match event {
            // Front matter is not content. `pulldown-cmark` already declines to
            // write it; dropping it here as well means the decision is visible
            // in this file rather than inherited from a dependency's behaviour.
            Event::Start(Tag::MetadataBlock(_)) => in_metadata = true,
            Event::End(TagEnd::MetadataBlock(_)) => in_metadata = false,
            _ if in_metadata => {}

            Event::Start(Tag::Heading {
                level,
                classes,
                attrs,
                ..
            }) => {
                let id = headings.next().map(|h| CowStr::from(h.slug.clone()));
                out.push(Event::Start(Tag::Heading {
                    level: *level,
                    id,
                    classes: classes.clone(),
                    attrs: attrs.clone(),
                }));
            }

            Event::Start(Tag::Link {
                dest_url,
                title,
                link_type,
                ..
            }) => {
                link_depth += 1;
                match classify(&opts.base, *link_type, dest_url).1 {
                    LinkPolicy::Anchor(a) => {
                        out.push(html(format!("<a href=\"{}\">", attr(&a))));
                        open_links.push(true);
                    }
                    LinkPolicy::External(u) => {
                        out.push(html(format!(
                            "<a href=\"{}\"{} target=\"_blank\" rel=\"noopener noreferrer\">",
                            attr(&u),
                            title_attr(title)
                        )));
                        open_links.push(true);
                    }
                    LinkPolicy::Note(p, anchor) => {
                        // `href` and `data-note-path` carry the same path on
                        // purpose: the frontend intercepts the click and calls
                        // `note_open`, so there is one path and no chance of
                        // the two disagreeing. The fragment travels beside it
                        // rather than inside it — `data-note-path` names a file
                        // and `#uma-secao` is not part of a file name.
                        let fragment = anchor
                            .as_deref()
                            .map(|a| format!("#{a}"))
                            .unwrap_or_default();
                        out.push(html(format!(
                            "<a href=\"{}{}\" data-note-path=\"{}\"{}{}>",
                            attr(p.as_str()),
                            attr(&fragment),
                            attr(p.as_str()),
                            anchor
                                .as_deref()
                                .map(|a| format!(" data-note-anchor=\"{}\"", attr(a)))
                                .unwrap_or_default(),
                            title_attr(title)
                        )));
                        open_links.push(true);
                    }
                    LinkPolicy::Refused => open_links.push(false),
                }
            }
            Event::End(TagEnd::Link) => {
                link_depth = link_depth.saturating_sub(1);
                if open_links.pop().unwrap_or(false) {
                    out.push(html("</a>".to_string()));
                }
            }

            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                // An image's alt text is the events between Start and End.
                // Collecting it here is what lets the whole image become one
                // element we wrote ourselves.
                let (alt, next) = alt_text(events, i);
                i = next;
                match url::classify_image(&opts.base, dest_url, opts.remote_images) {
                    ImagePolicy::Asset(p) => out.push(html(format!(
                        "<img src=\"notes-asset://{}/{}\" alt=\"{}\"{}>",
                        opts.workspace_id,
                        attr(&percent_encode_path(p.as_str())),
                        attr(&alt),
                        title_attr(title)
                    ))),
                    ImagePolicy::Remote(u) | ImagePolicy::Data(u) => out.push(html(format!(
                        "<img src=\"{}\" alt=\"{}\"{}>",
                        attr(&u),
                        attr(&alt),
                        title_attr(title)
                    ))),
                    ImagePolicy::Blocked(u) => {
                        blocked.push(u.clone());
                        // The URL as text, per §10: blocking a resource never
                        // stops the rest of the note rendering, and a silent
                        // gap is worse than a visible one.
                        out.push(html(format!(
                            "<span class=\"blocked-image\" data-blocked-src=\"{}\">{}</span>",
                            attr(&u),
                            text(&u)
                        )));
                    }
                    // A refused image still shows its alt text, exactly as a
                    // refused link still shows its words. The note is not
                    // rewritten and the reader is not left with a silent gap.
                    ImagePolicy::Refused => {
                        if !alt.is_empty() {
                            out.push(Event::Text(CowStr::from(alt)));
                        }
                    }
                }
            }

            // Raw HTML is untrusted content, and by default it is shown rather
            // than run (scope §8.4). Emitting it as `Text` makes the writer
            // escape it, which is also why `safe-in-code.md` still reads.
            Event::Html(s) | Event::InlineHtml(s) => {
                if opts.raw_html {
                    out.push(event.clone());
                } else {
                    out.push(Event::Text(s.clone()));
                }
            }

            Event::Start(Tag::CodeBlock(kind)) => {
                code_depth += 1;
                out.push(Event::Start(Tag::CodeBlock(kind.clone())));
            }
            Event::End(TagEnd::CodeBlock) => {
                code_depth = code_depth.saturating_sub(1);
                out.push(Event::End(TagEnd::CodeBlock));
            }

            // A bare `https://…` in prose becomes a link (scope §8.1 lists
            // autolinks among the GFM features). Never inside code, where the
            // text is what the user meant, and never inside a link, where it
            // would nest an anchor.
            Event::Text(t) if code_depth == 0 && link_depth == 0 => {
                let ranges = autolinks(t);
                if ranges.is_empty() {
                    out.push(Event::Text(t.clone()));
                } else {
                    let mut at = 0usize;
                    for r in ranges {
                        if r.start > at {
                            out.push(Event::Text(CowStr::from(t[at..r.start].to_string())));
                        }
                        let u = &t[r.start..r.end];
                        out.push(html(format!(
                            "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">",
                            attr(u)
                        )));
                        out.push(Event::Text(CowStr::from(u.to_string())));
                        out.push(html("</a>".to_string()));
                        at = r.end;
                    }
                    if at < t.len() {
                        out.push(Event::Text(CowStr::from(t[at..].to_string())));
                    }
                }
            }

            other => out.push(other.clone()),
        }
    }
    out
}

/// An email autolink is a `mailto:` by another spelling, and refused for the
/// same reason (`url::classify_link`). `pulldown-cmark` reports it without the
/// scheme, so `<alguem@example.com>` would otherwise be classified as a
/// relative path and rendered as a note link to a file with an `@` in its name
/// — which the golden corpus caught on its first reading.
fn classify(
    base: &RelPath,
    link_type: pulldown_cmark::LinkType,
    dest: &str,
) -> (LinkKind, LinkPolicy) {
    if link_type == pulldown_cmark::LinkType::Email {
        return (LinkKind::Refused, LinkPolicy::Refused);
    }
    url::classify_link(base, dest)
}

/// GFM autolink literals, narrowed to `http://` and `https://`.
///
/// **`www.` without a scheme is deliberately not linkified.** GFM guesses
/// `http://` for it, and guessing an insecure scheme on the user's behalf is
/// not something a note-taking application should do quietly; a user who wants
/// the link writes the scheme. The rest of the rule is GFM's: the URL runs to
/// the next space, and trailing punctuation that a sentence would end with is
/// not part of it.
fn autolinks(text: &str) -> Vec<std::ops::Range<usize>> {
    const TRAILING: &[char] = &['?', '!', '.', ',', ':', ';', '*', '_', '~', '\'', '"'];
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &text[i..];
        let len = if rest.starts_with("https://") {
            8
        } else if rest.starts_with("http://") {
            7
        } else {
            i += 1;
            while i < bytes.len() && !text.is_char_boundary(i) {
                i += 1;
            }
            continue;
        };
        // It has to begin a word: `nothttp://x` is not a link.
        let preceded_by_word = text[..i]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || matches!(c, '.' | ':' | '/' | '@' | '-'));
        if preceded_by_word {
            i += len;
            continue;
        }
        let mut end = i + len;
        while end < bytes.len() {
            let c = text[end..].chars().next().unwrap();
            if c.is_whitespace() || c == '<' {
                break;
            }
            end += c.len_utf8();
        }
        // Trailing punctuation belongs to the sentence, not to the URL — and a
        // closing parenthesis only if it is unbalanced, so a Wikipedia URL
        // ending in `(disambiguation)` survives.
        while let Some(c) = text[i..end].chars().next_back() {
            if TRAILING.contains(&c) {
                end -= c.len_utf8();
                continue;
            }
            if c == ')' {
                let slice = &text[i..end];
                if slice.matches(')').count() > slice.matches('(').count() {
                    end -= 1;
                    continue;
                }
            }
            break;
        }

        if end > i + len {
            out.push(i..end);
        }
        i = end.max(i + len);
    }
    out
}

/// The alt text of an image, and the index just past its `End`.
fn alt_text(events: &[Spanned<'_>], from: usize) -> (String, usize) {
    let mut alt = String::new();
    let mut i = from;
    let mut depth = 0usize;
    while i < events.len() {
        match &events[i].0 {
            Event::Start(Tag::Image { .. }) => depth += 1,
            Event::End(TagEnd::Image) if depth == 0 => return (alt, i + 1),
            Event::End(TagEnd::Image) => depth -= 1,
            Event::Text(t) | Event::Code(t) => alt.push_str(t),
            Event::SoftBreak | Event::HardBreak => alt.push(' '),
            _ => {}
        }
        i += 1;
    }
    (alt, i)
}

fn html(s: String) -> Event<'static> {
    Event::Html(CowStr::from(s))
}

fn title_attr(title: &str) -> String {
    if title.is_empty() {
        String::new()
    } else {
        format!(" title=\"{}\"", attr(title))
    }
}

/// Escape for an attribute value in double quotes.
fn attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escape for text content.
fn text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Percent-encode the characters a URL path cannot carry literally.
///
/// The path came from a note and names a file, so it may hold spaces, `#`, `?`
/// and `%` — each of which means something else in a URL. Everything else is
/// left alone, including non-ASCII: the WebView handles UTF-8 in a URL, and
/// encoding it would make the `notes-asset` handler undo work for nothing.
fn percent_encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '#' => out.push_str("%23"),
            '?' => out.push_str("%3F"),
            '%' => out.push_str("%25"),
            '"' => out.push_str("%22"),
            '\'' => out.push_str("%27"),
            _ => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Pass three: the allowlist that knows nothing about notes
// ---------------------------------------------------------------------------

/// Alignment is the one style a table cell may carry, and only these three
/// exact strings. `pulldown-cmark` writes them; anything else in a `style`
/// attribute is `fixtures/xss/style-injection.md`.
const TABLE_ALIGNMENTS: &[&str] = &[
    "text-align: left",
    "text-align: center",
    "text-align: right",
];

/// The allowlist is built **once**.
///
/// Measured rather than assumed, which is also why the win is stated small:
/// `notes-core`'s `tests/cost.rs` put a 337-byte note at **0.385 ms** with the
/// builder rebuilt per call and **0.293 ms** with it cached — about 0.09 ms of
/// `HashSet` construction per render, not the 0.3 ms the shape of the code
/// suggested. The rest is `html5ever` parsing, which is the work itself.
/// Everything the builder holds is `'static` and `clean` takes `&self`, so one
/// of them serves every render; the change costs nothing and is kept on that
/// basis rather than on the size of the number.
fn builder() -> &'static ammonia::Builder<'static> {
    static BUILDER: std::sync::OnceLock<ammonia::Builder<'static>> = std::sync::OnceLock::new();
    BUILDER.get_or_init(build)
}

fn sanitize(raw: &str) -> String {
    builder().clean(raw).to_string()
}

fn build() -> ammonia::Builder<'static> {
    let mut builder = ammonia::Builder::default();
    builder
        .tags(
            [
                // What the Markdown writer emits.
                "h1",
                "h2",
                "h3",
                "h4",
                "h5",
                "h6",
                "p",
                "a",
                "img",
                "ul",
                "ol",
                "li",
                "input",
                "table",
                "thead",
                "tbody",
                "tr",
                "th",
                "td",
                "pre",
                "code",
                "blockquote",
                "em",
                "strong",
                "del",
                "hr",
                "br",
                "sup",
                "sub",
                "span",
                "div",
                // …and what a note that has opted into raw HTML may reasonably
                // contain. A workspace with `raw_html` on that rendered only
                // half of ordinary formatting would be worse than one that
                // rendered none: the user cannot tell a stripped tag from a
                // typo. Nothing here can load a resource, run a script or
                // navigate — that property, not the length of the list, is what
                // the allowlist is for.
                "b",
                "i",
                "u",
                "s",
                "mark",
                "small",
                "kbd",
                "abbr",
                "caption",
                "dl",
                "dt",
                "dd",
                "figure",
                "figcaption",
                "details",
                "summary",
            ]
            .into_iter()
            .collect(),
        )
        .generic_attributes(std::collections::HashSet::new())
        .tag_attributes(
            [
                (
                    "a",
                    [
                        "href",
                        "title",
                        "target",
                        "rel",
                        "data-note-path",
                        "data-note-anchor",
                    ]
                    .into_iter()
                    .collect(),
                ),
                ("img", ["src", "alt", "title"].into_iter().collect()),
                ("code", ["class"].into_iter().collect()),
                ("pre", ["class"].into_iter().collect()),
                ("input", ["checked", "disabled"].into_iter().collect()),
                ("th", ["style", "align"].into_iter().collect()),
                ("td", ["style", "align"].into_iter().collect()),
                ("h1", ["id"].into_iter().collect()),
                ("h2", ["id"].into_iter().collect()),
                ("h3", ["id"].into_iter().collect()),
                ("h4", ["id"].into_iter().collect()),
                ("h5", ["id"].into_iter().collect()),
                ("h6", ["id"].into_iter().collect()),
                // Footnote definitions and references, which the HTML writer
                // emits directly.
                ("div", ["class", "id"].into_iter().collect()),
                ("sup", ["class"].into_iter().collect()),
                ("span", ["class", "data-blocked-src"].into_iter().collect()),
                ("li", ["id"].into_iter().collect()),
                ("abbr", ["title"].into_iter().collect()),
                ("details", ["open"].into_iter().collect()),
            ]
            .into_iter()
            .collect::<std::collections::HashMap<&str, std::collections::HashSet<&str>>>(),
        )
        // `notes-asset` is ours and jailed to the root by its handler; `data`
        // is here only for the raster allowlist, and the filter below is what
        // enforces that — the scheme alone would let `data:text/html` through
        // when `raw_html` is on.
        .url_schemes(
            ["http", "https", "mailto", "notes-asset", "data"]
                .into_iter()
                .collect(),
        )
        .url_relative(ammonia::UrlRelative::PassThrough)
        // Off: `rel` is set by the rewrite pass, on external links only, so a
        // relative note link does not carry a `rel` that means nothing.
        .link_rel(None)
        // The only `<input>` this application renders is the task-list
        // checkbox, and `type` is **forced** rather than allowed: an
        // `<input name="notes" value="tudo">` in a workspace that has opted into
        // raw HTML becomes a nameless, valueless checkbox with no form around
        // it — `fixtures/xss/form.md` is that payload.
        //
        // **Exactly one attribute is forced, and that is load-bearing.**
        // `ammonia` lifts a forced attribute out of its position and re-appends
        // it while iterating a `HashMap`, so forcing two makes the byte output
        // of a `<input checked disabled type=checkbox>` vary between runs and
        // the golden corpus flap. `disabled` is therefore allowlisted instead:
        // the task-list marker always carries it (the writer emits it, and
        // `fixtures/markdown/tasklists.html` pins it), and the only `<input>`
        // that can reach a reader without it is one a raw-HTML note wrote —
        // which can submit nothing, carries no data, and is a checkbox that
        // does not persist, exactly like the disabled ones beside it.
        .set_tag_attribute_value("input", "type", "checkbox")
        .attribute_filter(|element, attribute, value| match (element, attribute) {
            ("th" | "td", "style") if !TABLE_ALIGNMENTS.contains(&value.trim()) => None,
            ("img", "src") if value.trim_start().to_ascii_lowercase().starts_with("data:") => {
                let compact: String = value.chars().filter(|c| !c.is_whitespace()).collect();
                match url::classify_image(&RelPath::root(), &compact, false) {
                    ImagePolicy::Data(u) => Some(u.into()),
                    _ => None,
                }
            }
            _ => Some(value.into()),
        });
    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> RenderOpts {
        RenderOpts {
            base: RelPath::root(),
            raw_html: false,
            remote_images: false,
            workspace_id: WorkspaceId::from_uuid(uuid_nil()),
        }
    }

    fn uuid_nil() -> uuid::Uuid {
        "00000000-0000-0000-0000-000000000000".parse().unwrap()
    }

    #[test]
    fn front_matter_is_reported_and_not_rendered() {
        let src = "---\ntitle: x\n---\n\n# Título\n";
        let doc = parse(src);
        let fm = doc.front_matter.expect("front matter must be reported");
        // The span is the block itself; the newline that ends it belongs to
        // the document. What matters is that the bytes round-trip.
        assert_eq!(&src[fm.start..fm.end], "---\ntitle: x\n---");
        let r = render_html(src, &opts());
        assert!(!r.html.contains("title: x"), "{}", r.html);
        assert!(
            r.html.contains("<h1 id=\"título\">Título</h1>"),
            "{}",
            r.html
        );
    }

    #[test]
    fn a_dashed_rule_after_text_is_not_front_matter() {
        let src = "Uma linha.\n\n---\ntitle: não\n---\n";
        assert_eq!(parse(src).front_matter, None);
    }

    #[test]
    fn a_relative_link_carries_the_path_the_frontend_opens() {
        let r = render_html("[x](sub/outra.md)", &opts());
        assert!(
            r.html.contains("data-note-path=\"sub/outra.md\""),
            "{}",
            r.html
        );
    }

    #[test]
    fn an_external_link_cannot_reach_the_opener_without_noopener() {
        let r = render_html("[x](https://example.com/)", &opts());
        assert!(r.html.contains("rel=\"noopener noreferrer\""), "{}", r.html);
        assert!(r.html.contains("target=\"_blank\""), "{}", r.html);
    }

    #[test]
    fn a_refused_scheme_loses_the_link_and_keeps_the_words() {
        let r = render_html("[clique](javascript:alert(1))", &opts());
        assert!(!r.html.contains("javascript"), "{}", r.html);
        assert!(r.html.contains("clique"), "{}", r.html);
        assert!(!r.html.contains("<a "), "{}", r.html);
    }

    #[test]
    fn the_outline_numbers_a_repeated_heading() {
        let doc = parse("# A\n\n# A\n");
        assert_eq!(doc.headings[0].slug, "a");
        assert_eq!(doc.headings[1].slug, "a-1");
    }

    #[test]
    fn a_link_inside_code_is_reported_but_not_rendered() {
        let doc = parse("`[x](outra.md)`");
        assert_eq!(doc.links.len(), 1);
        assert!(doc.links[0].in_code);
        assert_eq!(doc.links[0].target, "outra.md");
        let r = render_html("`[x](outra.md)`", &opts());
        assert!(!r.html.contains("<a "), "{}", r.html);
    }

    #[test]
    fn a_blocked_remote_image_is_named_rather_than_fetched() {
        let r = render_html("![a](https://example.invalid/t.png)", &opts());
        assert_eq!(r.blocked_remote, vec!["https://example.invalid/t.png"]);
        assert!(!r.html.contains("<img"), "{}", r.html);
        assert!(r.html.contains("example.invalid/t.png"), "{}", r.html);
    }

    #[test]
    fn a_relative_image_becomes_an_asset_url() {
        let r = render_html("![a](sub/f.png)", &opts());
        assert!(
            r.html
                .contains("src=\"notes-asset://00000000-0000-0000-0000-000000000000/sub/f.png\""),
            "{}",
            r.html
        );
    }

    #[test]
    fn a_table_keeps_alignment_and_nothing_else() {
        let r = render_html("| a |\n|:-:|\n| b |\n", &opts());
        assert!(r.html.contains("text-align: center"), "{}", r.html);
    }

    #[test]
    fn a_note_link_keeps_its_section() {
        let r = render_html("[x](outra.md#uma-secao)", &opts());
        assert!(r.html.contains("data-note-path=\"outra.md\""), "{}", r.html);
        assert!(
            r.html.contains("data-note-anchor=\"uma-secao\""),
            "{}",
            r.html
        );
    }

    #[test]
    fn an_email_autolink_is_not_a_note_called_alguem_at_example_com() {
        let r = render_html("<alguem@example.com>", &opts());
        assert!(!r.html.contains("data-note-path"), "{}", r.html);
        assert!(!r.html.contains("<a "), "{}", r.html);
        assert!(r.html.contains("alguem@example.com"), "{}", r.html);
    }

    #[test]
    fn a_refused_image_still_shows_its_alt_text() {
        let r = render_html("![uma figura](file:///etc/passwd)", &opts());
        assert!(!r.html.contains("<img"), "{}", r.html);
        assert!(r.html.contains("uma figura"), "{}", r.html);
    }

    #[test]
    fn a_bare_url_in_prose_becomes_a_link_and_one_in_code_does_not() {
        let r = render_html(
            "Veja https://example.com/a e `https://example.com/b`.",
            &opts(),
        );
        assert!(
            r.html.contains("<a href=\"https://example.com/a\""),
            "{}",
            r.html
        );
        assert_eq!(r.html.matches("<a ").count(), 1, "{}", r.html);

        let fenced = render_html("```\nhttps://example.com/c\n```\n", &opts());
        assert!(!fenced.html.contains("<a "), "{}", fenced.html);
    }

    #[test]
    fn an_autolink_ends_where_the_sentence_does() {
        assert_eq!(
            autolinks("Veja https://example.com/a."),
            vec![5..26],
            "the full stop belongs to the sentence"
        );
        assert_eq!(
            autolinks("(https://example.com/a)"),
            vec![1..22],
            "an unbalanced closing parenthesis is not part of the URL"
        );
        assert_eq!(
            autolinks("https://en.wikipedia.org/wiki/A_(b)"),
            vec![0..35],
            "a balanced one is"
        );
        assert!(
            autolinks("nothttps://example.com").is_empty(),
            "a URL has to begin a word"
        );
        assert!(
            autolinks("http://").is_empty(),
            "a scheme alone is not a URL"
        );
    }

    #[test]
    fn raw_html_is_still_sanitized_when_it_is_turned_on() {
        let mut o = opts();
        o.raw_html = true;
        let r = render_html("<b>ok</b><script>window.x=1</script>", &o);
        assert!(r.html.contains("<b>ok</b>"), "{}", r.html);
        assert!(!r.html.contains("script"), "{}", r.html);
    }
}
