//! Source-preserving destination edits for reviewed moves. No filesystem access.
use notes_model::RelPath;
use pulldown_cmark::{Event, LinkType, Parser, Tag};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Range};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LinkEdit {
    pub start: usize,
    pub end: usize,
    pub before: String,
    pub after: String,
}

pub fn repath(path: &RelPath, from: &RelPath, to: &RelPath) -> RelPath {
    if path == from {
        return to.clone();
    }
    path.as_str()
        .strip_prefix(&format!("{}/", from.as_str()))
        .and_then(|tail| RelPath::parse(&format!("{}/{tail}", to.as_str())).ok())
        .unwrap_or_else(|| path.clone())
}

pub fn edits(source: &str, note: &RelPath, from: &RelPath, to: &RelPath) -> Vec<LinkEdit> {
    let next_note = repath(note, from, to);
    let base = note.parent().unwrap_or_else(RelPath::root);
    let next_base = next_note.parent().unwrap_or_else(RelPath::root);
    let parser = Parser::new_ext(source, super::options(source));
    let mut candidates: Vec<(Range<usize>, String, bool)> = Vec::new();
    let mut events = parser.into_offset_iter();
    for (event, span) in events.by_ref() {
        match event {
            Event::Start(Tag::Link {
                dest_url,
                link_type: LinkType::Inline,
                ..
            })
            | Event::Start(Tag::Image {
                dest_url,
                link_type: LinkType::Inline,
                ..
            }) => candidates.push((span, dest_url.to_string(), false)),
            _ => {}
        }
    }
    candidates.extend(
        events
            .reference_definitions()
            .iter()
            .map(|(_, d)| (d.span.clone(), d.dest.to_string(), true)),
    );
    let mut changes = BTreeMap::new();
    for (span, dest, definition) in candidates {
        if dest.starts_with('#') || dest.starts_with('/') || super::url::scheme_of(&dest).is_some()
        {
            continue;
        }
        let suffix = dest.find(['#', '?']).unwrap_or(dest.len());
        let Some(target) = super::url::resolve_relative(&base, &dest[..suffix]) else {
            continue;
        };
        let next_target = repath(&target, from, to);
        if next_target == target && next_base == base {
            continue;
        }
        let Some(local) = destination(&source[span.clone()], definition) else {
            continue;
        };
        let start = span.start + local.start;
        let end = span.start + local.end;
        // Escaped/entity destinations are left for explicit manual review.
        if source[start..end] != dest {
            continue;
        }
        let after = format!("{}{}", relative(&next_base, &next_target), &dest[suffix..]);
        changes.insert(
            start,
            LinkEdit {
                start,
                end,
                before: dest,
                after,
            },
        );
    }
    changes.into_values().collect()
}

fn destination(s: &str, definition: bool) -> Option<Range<usize>> {
    let b = s.as_bytes();
    let mut i = usize::from(b.first() == Some(&b'!'));
    if b.get(i) != Some(&b'[') {
        return None;
    }
    let mut depth = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    i += 1;
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    if b.get(i) != Some(&if definition { b':' } else { b'(' }) {
        return None;
    }
    i += 1;
    while b.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    let angle = b.get(i) == Some(&b'<');
    if angle {
        i += 1;
    }
    let start = i;
    let mut parens = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'\\' {
            i += 2;
            continue;
        }
        if angle && c == b'>' {
            return Some(start..i);
        }
        if !angle {
            if c.is_ascii_whitespace() || (c == b')' && parens == 0) {
                return Some(start..i);
            }
            if c == b'(' {
                parens += 1;
            } else if c == b')' {
                parens -= 1;
            }
        }
        i += 1;
    }
    (!angle).then_some(start..i)
}
pub fn relative(base: &RelPath, target: &RelPath) -> String {
    let a: Vec<_> = base.as_str().split('/').filter(|s| !s.is_empty()).collect();
    let b: Vec<_> = target.as_str().split('/').collect();
    let common = a.iter().zip(&b).take_while(|(a, b)| a == b).count();
    let mut pieces = vec!["..".to_string(); a.len() - common];
    pieces.extend(b[common..].iter().map(|p| {
        p.bytes()
            .map(|c| {
                if c.is_ascii_alphanumeric() || b"-._~".contains(&c) {
                    (c as char).to_string()
                } else {
                    format!("%{c:02X}")
                }
            })
            .collect::<String>()
    }));
    pieces.join("/")
}
pub fn apply(text: &str, edits: &[LinkEdit]) -> String {
    let mut result = text.to_string();
    for e in edits.iter().rev() {
        result.replace_range(e.start..e.end, &e.after);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(s: &str) -> RelPath {
        RelPath::parse(s).unwrap()
    }
    #[test]
    fn preserves_labels_titles_code_and_fragments() {
        let s = "[old.md](old.md#part \"old.md\") ` [x](old.md) `\n```\n[x](old.md)\n```\n";
        let e = edits(s, &p("a.md"), &p("old.md"), &p("new name.md"));
        assert_eq!(e.len(), 1);
        assert_eq!(
            apply(s, &e),
            s.replacen("(old.md#part", "(new%20name.md#part", 1)
        );
    }
    #[test]
    fn moves_outgoing_incoming_images_and_definitions() {
        let from = p("folder/a.md");
        let to = p("elsewhere/a.md");
        let s =
            "[a](../folder/a.md)\n![pic](img.png)\n[x][ref]\n\n[ref]: ../folder/a.md \"title\"\n";
        let e = edits(s, &from, &from, &to);
        let out = apply(s, &e);
        assert!(out.contains("[a](a.md)"));
        assert!(out.contains("![pic](../folder/img.png)"));
        assert!(out.contains("[ref]: a.md \"title\""));
    }
    #[test]
    fn directory_boundaries_and_external_links() {
        let s = "[x](folder2/a.md) [web](https://a/folder/a.md) [self](#h)";
        assert!(edits(s, &p("index.md"), &p("folder"), &p("next")).is_empty());
    }
}
