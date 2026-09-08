//! A strict reader for the HTML this crate produces.
//!
//! Shared by the golden corpus and the sanitizer suite. It is **not** a general
//! HTML parser and must never be used on anything but `ammonia`'s output, where
//! two guarantees make a scanner this small correct:
//!
//! - a `<` in text has been written `&lt;`, so every `<` starts a tag;
//! - a `"` in an attribute value has been written `&quot;`, so the first
//!   closing quote is the end of the value.
//!
//! Assertions are made against *tags and attributes* rather than against
//! substrings for one reason: `fixtures/xss/safe-in-code.md` must render the
//! string `javascript:alert(1)` as **text**, so a suite that greps the output
//! for `javascript:` asserts the opposite of the requirement.

#![allow(dead_code)] // each test binary uses a different part of this

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    pub closing: bool,
    pub attrs: Vec<(String, String)>,
}

pub fn tags(html: &str) -> Vec<Tag> {
    let bytes = html.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let Some(end) = bytes[i..].iter().position(|b| *b == b'>').map(|p| p + i) else {
            break;
        };
        let inner = &html[i + 1..end];
        i = end + 1;
        let (closing, inner) = match inner.strip_prefix('/') {
            Some(rest) => (true, rest),
            None => (false, inner),
        };
        let name: String = inner
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '!' || *c == '-')
            .collect::<String>()
            .to_ascii_lowercase();
        if name.is_empty() {
            continue;
        }
        out.push(Tag {
            attrs: attributes(&inner[name.len()..]),
            name,
            closing,
        });
    }
    out
}

fn attributes(mut rest: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    loop {
        rest = rest.trim_start();
        if rest.is_empty() || rest == "/" {
            return out;
        }
        let name_len = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':')))
            .unwrap_or(rest.len());
        if name_len == 0 {
            // Not an attribute name — stop rather than guess.
            return out;
        }
        let name = rest[..name_len].to_ascii_lowercase();
        rest = &rest[name_len..];
        let value = match rest.strip_prefix('=').map(str::trim_start) {
            Some(after) => match after.strip_prefix('"') {
                Some(quoted) => {
                    let end = quoted.find('"').unwrap_or(quoted.len());
                    let v = &quoted[..end];
                    rest = &quoted[(end + 1).min(quoted.len())..];
                    unescape(v)
                }
                None => {
                    let end = after.find(char::is_whitespace).unwrap_or(after.len());
                    let v = &after[..end];
                    rest = &after[end..];
                    unescape(v)
                }
            },
            None => String::new(),
        };
        out.push((name, value));
    }
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

impl Tag {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
    pub fn has(&self, name: &str) -> bool {
        self.attrs.iter().any(|(n, _)| n == name)
    }
}

/// The scheme of a URL as it appears in the finished HTML, lowercased.
pub fn scheme(url: &str) -> Option<String> {
    let compact: String = url.chars().filter(|c| !c.is_whitespace()).collect();
    let colon = compact.find(':')?;
    if compact[..colon].contains(['/', '?', '#']) {
        return None;
    }
    Some(compact[..colon].to_ascii_lowercase())
}

#[test]
fn the_scanner_reads_what_ammonia_writes() {
    let t = tags(r#"<a href="a b.md" data-note-path="a b.md" target="_blank">x</a>"#);
    assert_eq!(t.len(), 2);
    assert_eq!(t[0].name, "a");
    assert_eq!(t[0].attr("href"), Some("a b.md"));
    assert_eq!(t[0].attr("target"), Some("_blank"));
    assert!(t[1].closing);
}

#[test]
fn an_escaped_payload_in_text_is_not_a_tag() {
    let t = tags("<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>");
    assert_eq!(
        t.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        vec!["p", "p"]
    );
}

#[test]
fn a_quote_inside_an_attribute_value_is_escaped_so_the_value_ends_where_it_should() {
    let t = tags(r#"<img src="a&quot;b.png" alt="x">"#);
    assert_eq!(t[0].attr("src"), Some("a\"b.png"));
    assert_eq!(t[0].attr("alt"), Some("x"));
}
