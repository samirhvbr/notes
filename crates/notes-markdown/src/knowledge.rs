//! Read-only metadata interpretation; source bytes are never serialized back.
use crate::{Span, Spanned};
use pulldown_cmark::{Event, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use ts_rs::TS;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Metadata {
    pub title: Option<String>,
    pub properties: Vec<Property>,
    pub tags: Vec<String>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Property {
    pub name: String,
    pub value: String,
}

pub fn metadata(src: &str) -> Metadata {
    let (doc, _) = crate::analyse(src);
    let mut result = front_matter(src, doc.front_matter);
    result.tags = doc.tags;
    result
}

pub(crate) fn front_matter(src: &str, span: Option<Span>) -> Metadata {
    let mut result = Metadata::default();
    let Some(span) = span else { return result };
    let raw = &src[span.start..span.end];
    if raw.len() > 65536 {
        result.error = Some("Front matter exceeds 64 KiB".into());
        return result;
    }
    let body = raw
        .lines()
        .skip(1)
        .take_while(|l| l.trim_end() != "---" && l.trim_end() != "...")
        .collect::<Vec<_>>()
        .join("\n");
    let value = match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&body) {
        Ok(v) => v,
        Err(_) => {
            result.error = Some("Invalid YAML front matter".into());
            return result;
        }
    };
    if value.is_null() {
        return result;
    }
    let Some(map) = value.as_mapping() else {
        result.error = Some("Front matter must be a mapping".into());
        return result;
    };
    for (key, value) in map {
        let Some(name) = key.as_str() else { continue };
        if name == "title" {
            result.title = value.as_str().map(str::to_owned);
        }
        if name == "tags" {
            if let Some(tag) = value.as_str() {
                result
                    .tags
                    .extend(tag.split([',', ' ']).filter_map(normalize_tag));
            }
            if let Some(tags) = value.as_sequence() {
                result.tags.extend(
                    tags.iter()
                        .filter_map(|v| v.as_str())
                        .filter_map(normalize_tag),
                );
            }
        }
        let text = value.as_str().map(str::to_owned).unwrap_or_else(|| {
            serde_yaml_ng::to_string(value)
                .unwrap_or_default()
                .trim()
                .to_owned()
        });
        result.properties.push(Property {
            name: name.into(),
            value: text,
        });
    }
    result
}
fn normalize_tag(raw: &str) -> Option<String> {
    let tag = raw.trim().trim_start_matches('#').trim_end_matches('/');
    if tag.is_empty()
        || !tag.chars().any(char::is_alphabetic)
        || !tag
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/'))
    {
        return None;
    }
    Some(tag.to_lowercase())
}
pub(crate) fn tags(src: &str, events: &[Spanned<'_>], front: Option<Span>) -> Vec<String> {
    let mut found: BTreeSet<String> = front_matter(src, front).tags.into_iter().collect();
    let mut blocked = 0usize;
    for (event, range) in events {
        match event {
            Event::Start(
                Tag::CodeBlock(_)
                | Tag::MetadataBlock(_)
                | Tag::Link { .. }
                | Tag::Image { .. }
                | Tag::HtmlBlock,
            ) => blocked += 1,
            Event::End(
                TagEnd::CodeBlock
                | TagEnd::MetadataBlock(_)
                | TagEnd::Link
                | TagEnd::Image
                | TagEnd::HtmlBlock,
            ) => blocked = blocked.saturating_sub(1),
            Event::Text(text) if blocked == 0 && src.get(range.clone()) == Some(text.as_ref()) => {
                let chars: Vec<(usize, char)> = text.char_indices().collect();
                for (i, (offset, c)) in chars.iter().enumerate() {
                    if *c != '#'
                        || (i > 0
                            && !chars[i - 1].1.is_whitespace()
                            && !matches!(chars[i - 1].1, '(' | '['))
                    {
                        continue;
                    }
                    if range.start + offset > 0 && src.as_bytes()[range.start + offset - 1] == b'\\'
                    {
                        continue;
                    }
                    let tail = &text[offset + 1..];
                    let end = tail
                        .find(|c: char| !c.is_alphanumeric() && !matches!(c, '_' | '-' | '/'))
                        .unwrap_or(tail.len());
                    if let Some(tag) = normalize_tag(&tail[..end]) {
                        found.insert(tag);
                    }
                }
            }
            _ => {}
        }
    }
    found.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_and_tags_exclude_code_and_destinations() {
        let src="---\ntitle: Café\ntags: [Rust, 'projeto/ação']\ncount: 3\n---\n# Heading\n#Visible `#inline` [link](#anchor) \\#escaped\n```md\n#hidden\n```\n";
        let m = metadata(src);
        assert_eq!(m.title.as_deref(), Some("Café"));
        assert_eq!(m.tags, vec!["projeto/ação", "rust", "visible"]);
        assert!(m.error.is_none());
    }
    #[test]
    fn indented_fences_are_yaml_scalar_content_not_metadata_delimiters() {
        let src =
            "---\nsummary: |\n  before\n  ---\n  middle\n  ...\n  after\ntags: [kept]\n---\nbody\n";
        let m = metadata(src);
        assert!(m.error.is_none());
        assert_eq!(m.tags, vec!["kept"]);
        assert_eq!(m.properties[0].value, "before\n---\nmiddle\n...\nafter\n");
    }

    #[test]
    fn invalid_yaml_does_not_hide_body_tags() {
        let m = metadata("---\ntags: [broken\n---\n#visible\n");
        assert!(m.error.is_some());
        assert_eq!(m.tags, vec!["visible"]);
    }
    #[test]
    fn wiki_links_are_parser_nodes_but_code_is_not() {
        let d = crate::parse("[[Note|label]] `[[hidden]]`\n");
        assert_eq!(
            d.links
                .iter()
                .filter(|l| l.kind == crate::LinkKind::Wiki)
                .count(),
            1
        );
        let r = crate::render_html(
            "[[Note|label]]",
            &crate::RenderOpts::for_note(
                &notes_model::RelPath::parse("a.md").unwrap(),
                notes_model::WorkspaceId::new(),
            ),
        );
        assert!(r.html.contains("data-wiki-target=\"Note\""));
        assert!(r.html.contains("label"));
    }
}
