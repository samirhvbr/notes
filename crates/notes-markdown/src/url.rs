//! What a URL in a note is allowed to become.
//!
//! **This module is the security policy of the preview in code.** `ammonia`
//! runs after it as a second, independent allowlist, and the two are meant to
//! agree — but the decision about which scheme is admissible, and about which
//! relative path stays inside the workspace, is taken here, once, on the way
//! through the parser (`docs/ARCHITECTURE.md` §10).
//!
//! Scope §8.4: *"Nota é conteúdo não confiável, mesmo local."* Everything below
//! follows from that sentence.

use notes_model::RelPath;

/// What kind of destination a link had, for `Document.links`.
///
/// Named separately from the policy because the outline reports what the user
/// *wrote*, and the renderer decides what to *emit*; 0.2's link rewriter needs
/// the former and would be wrong to read the latter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LinkKind {
    /// A path relative to the note, resolving inside the workspace.
    RelativePath,
    /// `http(s)`.
    Url,
    /// `#section`, within this note.
    Anchor,
    /// `[[wiki]]` — resolved by the core with explicit ambiguity handling.
    Wiki,
    /// Anything else: a scheme we refuse, or a relative path that escapes the
    /// workspace. Reported so the outline is honest; never rendered as a link.
    Refused,
}

/// What the renderer does with a link destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkPolicy {
    /// Resolved by the core; ambiguous names require a user choice.
    Wiki(String),
    /// Kept verbatim; scrolls within the rendered note.
    Anchor(String),
    /// `http(s)` — opened by the shell, never by the WebView.
    External(String),
    /// Inside the workspace: the frontend opens it in-app. The second field is
    /// the `#fragment`, without its `#`, when the link named one.
    Note(RelPath, Option<String>),
    /// Dropped. The link text survives; the destination does not.
    Refused,
}

/// What the renderer does with an image destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImagePolicy {
    /// Served by the `notes-asset://` handler, which applies the same root jail.
    Asset(RelPath),
    /// A remote image the user has opted into.
    Remote(String),
    /// A remote image with `remote_images: false`. The URL is shown as text —
    /// blocking a resource never stops the rest of the note rendering
    /// (scope §8.4).
    Blocked(String),
    /// An inline `data:` image in the narrow allowlist below.
    Data(String),
    Refused,
}

/// The scheme of a URL, lowercased, or `None` when it has none.
///
/// **Whitespace and control characters are removed before the colon is looked
/// for**, which is the whole point: CommonMark decodes entity references in a
/// link destination, so `jav&#x09;ascript:alert(1)` reaches us as
/// `jav\tascript:alert(1)` and a naive `starts_with("javascript:")` lets it
/// through. `fixtures/xss/mixed-case-and-entities.md` is that exact case.
pub fn scheme_of(url: &str) -> Option<String> {
    let stripped: String = url
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect();
    let colon = stripped.find(':')?;
    // A colon after a `/`, `?` or `#` belongs to the path, not to a scheme.
    if stripped[..colon].contains(['/', '?', '#']) {
        return None;
    }
    let scheme = &stripped[..colon];
    let mut chars = scheme.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) {
        return None;
    }
    Some(scheme.to_ascii_lowercase())
}

/// Resolve a relative destination against the directory the note is in.
///
/// Returns `None` when it leaves the workspace — `../../../../etc/passwd` is
/// `fixtures/xss/escaping-image.md`, and it is a perfectly well-formed relative
/// path, which is why refusing it is this function's job rather than
/// [`RelPath`]'s.
pub fn resolve_relative(base: &RelPath, dest: &str) -> Option<RelPath> {
    if dest.is_empty() {
        return None;
    }
    // An absolute destination is not "the workspace root": in a note that is
    // read outside this application it means the filesystem root, and honouring
    // it here would make the preview disagree with every other Markdown tool.
    if dest.starts_with('/') || dest.starts_with('\\') {
        return None;
    }
    let mut segments: Vec<String> = if base.is_root() {
        Vec::new()
    } else {
        base.as_str().split('/').map(str::to_string).collect()
    };
    for raw in dest.split('/') {
        match raw {
            "" | "." => continue,
            ".." => {
                segments.pop()?;
            }
            seg => segments.push(percent_decode(seg)),
        }
    }
    RelPath::parse(&segments.join("/")).ok()
}

/// Decode `%XX` in one path segment. Public because the `notes-asset://`
/// handler in `src-tauri` has to undo exactly what the renderer wrote.
///
/// A destination written `com%20espaco.md` names a file with a space in it, and
/// the path that reaches `notes-fs` has to be the name on disk. Invalid escapes
/// are left alone rather than dropped — a literal `%` in a filename is legal.
pub fn percent_decode(s: &str) -> String {
    if !s.contains('%') {
        return s.to_string();
    }
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(v) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    // A percent escape that does not spell UTF-8 is not a reason to lose the
    // name; the lossy form still names something, and the filesystem will say
    // whether it exists.
    String::from_utf8_lossy(&out).into_owned()
}

/// The `data:` prefixes an image may carry.
///
/// **SVG is not on it, and that is the whole reason there is a list.** An
/// `image/svg+xml` data URI is a document with scripting, not a picture; every
/// other entry here decodes to pixels and nothing else.
const DATA_IMAGE_ALLOWLIST: &[&str] = &[
    "data:image/png;base64,",
    "data:image/jpeg;base64,",
    "data:image/gif;base64,",
    "data:image/webp;base64,",
];

pub fn classify_link(base: &RelPath, dest: &str) -> (LinkKind, LinkPolicy) {
    if dest.starts_with('#') {
        return (LinkKind::Anchor, LinkPolicy::Anchor(dest.to_string()));
    }
    match scheme_of(dest).as_deref() {
        Some("http") | Some("https") => (LinkKind::Url, LinkPolicy::External(dest.to_string())),
        // **`mailto:` is refused, and the scope says so.** §8.4: *"Links
        // externos `http(s)` abrem no navegador do SO por clique. Outros
        // esquemas recusados."* The capability file agrees — `shell:allow-open`
        // is restricted to `http` and `https` — so a rendered `mailto:` link
        // would be one that does nothing when clicked, which is worse than
        // text. Widening the capability is the owner's act, not this crate's
        // (CLAUDE.md golden rule 7); `docs/DECISIONS-0.1b.md` D-06.
        Some("mailto") => (LinkKind::Refused, LinkPolicy::Refused),
        // `//host/path` is protocol-relative: it is a remote URL wearing the
        // shape of a path, and inside a WebView with a `tauri://` origin it
        // resolves to something nobody intended.
        Some(_) => (LinkKind::Refused, LinkPolicy::Refused),
        None if dest.starts_with("//") => (LinkKind::Refused, LinkPolicy::Refused),
        None => {
            // `outra.md#uma-secao` is one link with two halves, and the second
            // half is the whole point of scope §8.3's
            // `[texto](caminho/relativo.md#titulo-opcional)`: the frontend opens
            // the note *and* scrolls. Dropping it silently was the defect the
            // golden corpus caught on its first reading.
            let (path, anchor) = match dest.split_once('#') {
                Some((p, a)) => (p, (!a.is_empty()).then(|| a.to_string())),
                None => (dest, None),
            };
            match resolve_relative(base, path) {
                Some(p) => (LinkKind::RelativePath, LinkPolicy::Note(p, anchor)),
                None => (LinkKind::Refused, LinkPolicy::Refused),
            }
        }
    }
}

pub fn classify_image(base: &RelPath, dest: &str, remote_images: bool) -> ImagePolicy {
    match scheme_of(dest).as_deref() {
        Some("http") | Some("https") => {
            if remote_images {
                ImagePolicy::Remote(dest.to_string())
            } else {
                ImagePolicy::Blocked(dest.to_string())
            }
        }
        Some("data") => {
            let compact: String = dest.chars().filter(|c| !c.is_whitespace()).collect();
            let lower = compact.to_ascii_lowercase();
            if DATA_IMAGE_ALLOWLIST.iter().any(|p| lower.starts_with(p)) {
                ImagePolicy::Data(compact)
            } else {
                ImagePolicy::Refused
            }
        }
        Some(_) => ImagePolicy::Refused,
        None if dest.starts_with("//") => {
            // Protocol-relative, so remote. Treated as the remote it is rather
            // than as the path it looks like.
            let url = format!("https:{dest}");
            if remote_images {
                ImagePolicy::Remote(url)
            } else {
                ImagePolicy::Blocked(url)
            }
        }
        None => match resolve_relative(base, dest) {
            Some(p) => ImagePolicy::Asset(p),
            None => ImagePolicy::Refused,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(s: &str) -> RelPath {
        RelPath::parse(s).unwrap()
    }

    #[test]
    fn a_scheme_survives_a_tab_in_the_middle_of_it() {
        assert_eq!(
            scheme_of("jav\tascript:alert(1)").as_deref(),
            Some("javascript")
        );
        assert_eq!(scheme_of("JaVaScRiPt:x").as_deref(), Some("javascript"));
        assert_eq!(scheme_of(" java\nscript :x").as_deref(), Some("javascript"));
    }

    #[test]
    fn a_colon_in_a_path_is_not_a_scheme() {
        assert_eq!(scheme_of("pasta/a:b.md"), None);
        assert_eq!(scheme_of("#uma:secao"), None);
        assert_eq!(scheme_of("?q=a:b"), None);
        // A digit cannot start a scheme, so a port-looking prefix is not one.
        assert_eq!(scheme_of("8080:x"), None);
    }

    #[test]
    fn a_relative_path_may_climb_but_not_out() {
        let base = rel("a/b");
        assert_eq!(resolve_relative(&base, "c.md"), Some(rel("a/b/c.md")));
        assert_eq!(resolve_relative(&base, "../c.md"), Some(rel("a/c.md")));
        assert_eq!(resolve_relative(&base, "../../c.md"), Some(rel("c.md")));
        assert_eq!(resolve_relative(&base, "../../../c.md"), None);
        assert_eq!(resolve_relative(&base, "./c.md"), Some(rel("a/b/c.md")));
        assert_eq!(
            resolve_relative(&RelPath::root(), "../../../../etc/passwd"),
            None
        );
        assert_eq!(resolve_relative(&RelPath::root(), "/etc/passwd"), None);
    }

    #[test]
    fn a_percent_escape_names_the_file_on_disk() {
        assert_eq!(
            resolve_relative(&RelPath::root(), "com%20espaco.md"),
            Some(rel("com espaco.md"))
        );
        assert_eq!(
            resolve_relative(&RelPath::root(), "acentua%C3%A7%C3%A3o.md"),
            Some(rel("acentuação.md"))
        );
        // A lone percent is a legal filename character, not a broken escape to
        // be swallowed.
        assert_eq!(
            resolve_relative(&RelPath::root(), "100%.md"),
            Some(rel("100%.md"))
        );
    }

    #[test]
    fn every_scheme_but_the_three_is_refused() {
        for dest in [
            "javascript:alert(1)",
            "mailto:alguem@example.com",
            "data:text/html;base64,PHNjcmlwdD4=",
            "file:///etc/passwd",
            "vbscript:x",
            "notes-asset://elsewhere/x.md",
            "//example.invalid/tracker.png",
        ] {
            let (kind, policy) = classify_link(&RelPath::root(), dest);
            assert_eq!(kind, LinkKind::Refused, "{dest} must be refused");
            assert_eq!(policy, LinkPolicy::Refused, "{dest} must be refused");
        }
    }

    #[test]
    fn only_raster_data_images_are_admissible() {
        let base = RelPath::root();
        assert!(matches!(
            classify_image(&base, "data:image/png;base64,iVBORw0KGgo=", false),
            ImagePolicy::Data(_)
        ));
        // The one that matters: an SVG data URI is a scriptable document.
        assert_eq!(
            classify_image(&base, "data:image/svg+xml;base64,PHN2Zz4=", false),
            ImagePolicy::Refused
        );
        assert_eq!(
            classify_image(&base, "data:text/html;base64,PHNjcmlwdD4=", false),
            ImagePolicy::Refused
        );
    }

    #[test]
    fn a_remote_image_is_blocked_by_default_and_kept_when_opted_in() {
        let base = RelPath::root();
        let url = "https://example.invalid/tracker.png";
        assert_eq!(
            classify_image(&base, url, false),
            ImagePolicy::Blocked(url.into())
        );
        assert_eq!(
            classify_image(&base, url, true),
            ImagePolicy::Remote(url.into())
        );
    }
}
