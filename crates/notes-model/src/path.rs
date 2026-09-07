use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    #[error("path is empty")]
    Empty,
    #[error("path is absolute")]
    Absolute,
    #[error("path contains a `.` or `..` segment")]
    DotSegment,
    #[error("path contains a backslash")]
    Backslash,
    #[error("path contains a control character")]
    Control,
    #[error("path has an empty segment")]
    EmptySegment,
    #[error("path ends with a separator")]
    TrailingSeparator,
}

/// A path relative to the workspace root, `/`-separated, **exactly as the name
/// is on disk**.
///
/// Never normalised and never rewritten: normalising Unicode here would produce
/// a string that does not open the file the user actually has, on any
/// filesystem that stores NFD (`ARCHITECTURE.md` §3). Comparison is
/// [`CompareKey`]'s job, not this type's.
///
/// The parser is the **first** half of the root jail; the second half is
/// `notes-fs` re-resolving and re-checking at the moment of use, because a
/// symlink turns a valid-looking relative path into an escape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(try_from = "String", into = "String")]
#[ts(export, type = "string")]
pub struct RelPath(String);

impl RelPath {
    pub fn parse(s: &str) -> Result<Self, PathError> {
        if s.is_empty() {
            return Err(PathError::Empty);
        }
        if s.contains('\\') {
            return Err(PathError::Backslash);
        }
        if s.chars().any(|c| c.is_control()) {
            return Err(PathError::Control);
        }
        if s.starts_with('/') || Self::has_drive_letter(s) {
            return Err(PathError::Absolute);
        }
        if s.ends_with('/') {
            return Err(PathError::TrailingSeparator);
        }
        for seg in s.split('/') {
            match seg {
                "" => return Err(PathError::EmptySegment),
                "." | ".." => return Err(PathError::DotSegment),
                _ => {}
            }
        }
        Ok(Self(s.to_string()))
    }

    fn has_drive_letter(s: &str) -> bool {
        let b = s.as_bytes();
        b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
    }

    /// The workspace root itself, for listing. Not a valid note path.
    pub fn root() -> Self {
        Self(String::new())
    }
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn file_name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or(&self.0)
    }

    pub fn parent(&self) -> Option<RelPath> {
        match self.0.rfind('/') {
            Some(i) => Some(RelPath(self.0[..i].to_string())),
            None if self.0.is_empty() => None,
            None => Some(RelPath::root()),
        }
    }

    /// Append one already-validated segment.
    pub fn join(&self, name: &str) -> Result<RelPath, PathError> {
        let candidate = if self.0.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", self.0, name)
        };
        RelPath::parse(&candidate)
    }

    /// The extension, lowercased, without the dot.
    pub fn extension(&self) -> Option<String> {
        let name = self.file_name();
        let i = name.rfind('.')?;
        if i == 0 {
            return None; // a dot-file is not an extension
        }
        Some(name[i + 1..].to_lowercase())
    }

    /// A file the app treats as a note. `.md` and `.markdown` (scope §7.6).
    pub fn is_note(&self) -> bool {
        matches!(self.extension().as_deref(), Some("md") | Some("markdown"))
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for RelPath {
    type Error = PathError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        RelPath::parse(&s)
    }
}

impl From<RelPath> for String {
    fn from(p: RelPath) -> String {
        p.0
    }
}

/// A key for *comparing* paths — never for opening one, never shown, never
/// written to disk.
///
/// NFC always, plus Unicode case-folding when the root's filesystem does not
/// distinguish case. Two files whose `CompareKey`s are equal collide on that
/// root even when their `RelPath`s differ.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CompareKey(String);

impl CompareKey {
    pub fn new(path: &RelPath, case_insensitive: bool) -> Self {
        let nfc = nfc(path.as_str());
        Self(if case_insensitive { case_fold(&nfc) } else { nfc })
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Case-folding, approximated by `to_lowercase`.
///
/// Full Unicode case-folding differs from lowercasing in a handful of scripts
/// (final sigma, dotted/dotless I). Those differences matter for correctness of
/// *collision refusal*, and refusing a collision too eagerly is the safe
/// direction — the same direction the unknown-case default takes. Replacing this
/// with `unicode-case-mapping` is a dependency decision, not a design one.
fn case_fold(s: &str) -> String {
    s.to_lowercase()
}

/// NFC, restricted to what a note path realistically contains.
///
/// A full normaliser is `unicode-normalization`, which this crate deliberately
/// does not depend on (`ARCHITECTURE.md` §2 pins the dependency list). What is
/// implemented is the case that actually occurs: macOS hands out NFD, so a
/// combining mark following a Latin letter is recomposed. Anything else is left
/// alone, which can only make two different paths compare unequal — the safe
/// direction again.
fn nfc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if let Some(prev) = out.chars().last() {
            if let Some(composed) = compose(prev, c) {
                out.pop();
                out.push(composed);
                continue;
            }
        }
        out.push(c);
    }
    out
}

/// Latin-1 Supplement and Latin Extended-A compositions, which is what a
/// Portuguese, Spanish, French or German filename needs.
fn compose(base: char, mark: char) -> Option<char> {
    let table: &[(char, char, char)] = &[
        ('a', '\u{300}', 'à'), ('a', '\u{301}', 'á'), ('a', '\u{302}', 'â'),
        ('a', '\u{303}', 'ã'), ('a', '\u{308}', 'ä'), ('a', '\u{30a}', 'å'),
        ('c', '\u{327}', 'ç'),
        ('e', '\u{300}', 'è'), ('e', '\u{301}', 'é'), ('e', '\u{302}', 'ê'),
        ('e', '\u{308}', 'ë'),
        ('i', '\u{300}', 'ì'), ('i', '\u{301}', 'í'), ('i', '\u{302}', 'î'),
        ('i', '\u{308}', 'ï'),
        ('n', '\u{303}', 'ñ'),
        ('o', '\u{300}', 'ò'), ('o', '\u{301}', 'ó'), ('o', '\u{302}', 'ô'),
        ('o', '\u{303}', 'õ'), ('o', '\u{308}', 'ö'),
        ('u', '\u{300}', 'ù'), ('u', '\u{301}', 'ú'), ('u', '\u{302}', 'û'),
        ('u', '\u{308}', 'ü'),
        ('y', '\u{301}', 'ý'), ('y', '\u{308}', 'ÿ'),
    ];
    let lower = base.to_lowercase().next()?;
    let (_, _, composed) = table.iter().find(|(b, m, _)| *b == lower && *m == mark)?;
    if base.is_uppercase() {
        composed.to_uppercase().next()
    } else {
        Some(*composed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_every_escape_shape() {
        for bad in ["", "/etc/passwd", "../x.md", "a/../../x.md", "./x.md",
                    "a\\b.md", "a//b.md", "dir/", "C:/x.md", "a\u{0}b.md"] {
            assert!(RelPath::parse(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn accepts_names_the_user_actually_has() {
        for good in ["a.md", "dir/a.md", "com espaço.md", "acentuação.md",
                     "a.b/c.d.md", ".oculto.md", "dir/sub/deep.md"] {
            assert!(RelPath::parse(good).is_ok(), "should accept {good:?}");
        }
    }

    #[test]
    fn keeps_the_name_exactly_as_given() {
        let nfd = "cafe\u{301}.md";
        let p = RelPath::parse(nfd).unwrap();
        assert_eq!(p.as_str(), nfd, "RelPath must not normalise");
    }

    #[test]
    fn extension_and_note_detection() {
        assert!(RelPath::parse("a.md").unwrap().is_note());
        assert!(RelPath::parse("a.MD").unwrap().is_note());
        assert!(RelPath::parse("a.markdown").unwrap().is_note());
        assert!(!RelPath::parse("a.txt").unwrap().is_note());
        assert!(!RelPath::parse("a").unwrap().is_note());
        assert!(!RelPath::parse(".md").unwrap().is_note(), "a dot-file is not an extension");
    }

    #[test]
    fn parent_and_join() {
        let p = RelPath::parse("a/b/c.md").unwrap();
        assert_eq!(p.parent().unwrap().as_str(), "a/b");
        assert_eq!(p.file_name(), "c.md");
        assert_eq!(RelPath::root().join("x.md").unwrap().as_str(), "x.md");
        assert_eq!(
            RelPath::parse("a").unwrap().join("b.md").unwrap().as_str(),
            "a/b.md"
        );
        assert!(RelPath::parse("a").unwrap().join("../b.md").is_err());
    }

    #[test]
    fn compare_key_folds_nfd_onto_nfc() {
        let nfd = RelPath::parse("cafe\u{301}.md").unwrap();
        let nfc = RelPath::parse("café.md").unwrap();
        assert_ne!(nfd, nfc, "distinct paths on disk");
        assert_eq!(
            CompareKey::new(&nfd, false),
            CompareKey::new(&nfc, false),
            "but the same file to compare against"
        );
    }

    #[test]
    fn compare_key_respects_case_only_when_asked() {
        let a = RelPath::parse("Nota.md").unwrap();
        let b = RelPath::parse("nota.md").unwrap();
        assert_ne!(CompareKey::new(&a, false), CompareKey::new(&b, false));
        assert_eq!(CompareKey::new(&a, true), CompareKey::new(&b, true));
    }
}
