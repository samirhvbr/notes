use notes_model::{Entry, EntryKind};
use std::path::Path;

/// Decide whether a root's filesystem distinguishes case — **by reading only**.
///
/// `None` means inconclusive: an empty root, or one where no entry has a cased
/// letter. The caller resolves that to *insensitive*, which is the safe
/// direction — treating a case-sensitive filesystem as insensitive refuses a
/// legitimate name, while the reverse lets a create pass its collision check and
/// overwrite a note (`docs/DECISIONS-0.1a.md` D-01).
///
/// The alternative, writing a temporary file and testing the fold, is refused by
/// scope §2.3: opening a folder must not create anything in it, and 0.1a has an
/// acceptance criterion that tests exactly that.
pub fn probe_case_insensitive(root: &Path, entries: &[Entry]) -> Option<bool> {
    for e in entries {
        if e.kind == EntryKind::Symlink {
            continue;
        }
        let flipped = flip_case(&e.name)?;
        if flipped == e.name {
            continue;
        }
        let original = root.join(&e.name);
        let other = root.join(&flipped);

        let Ok(a) = std::fs::symlink_metadata(&original) else { continue };
        match std::fs::symlink_metadata(&other) {
            // The flipped name resolves. It is the same file if the filesystem
            // folds case; a genuinely distinct file that differs only by case
            // proves the opposite.
            Ok(b) => return Some(same_file(&a, &b)),
            Err(_) => return Some(false),
        }
    }
    None
}

/// Invert the case of the first cased character, leaving the rest alone.
///
/// Flipping only one character keeps the probe honest on filesystems that fold
/// some scripts and not others, and avoids a name whose length changes under
/// case mapping (ß → SS), which would test the wrong thing.
fn flip_case(name: &str) -> Option<String> {
    let idx = name.char_indices().find(|(_, c)| c.is_alphabetic() && (c.is_lowercase() || c.is_uppercase()))?;
    let (i, c) = idx;
    let flipped: String = if c.is_lowercase() {
        c.to_uppercase().collect()
    } else {
        c.to_lowercase().collect()
    };
    if flipped.chars().count() != 1 {
        return None;
    }
    let mut out = String::with_capacity(name.len());
    out.push_str(&name[..i]);
    out.push_str(&flipped);
    out.push_str(&name[i + c.len_utf8()..]);
    Some(out)
}

#[cfg(unix)]
fn same_file(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.dev() == b.dev() && a.ino() == b.ino()
}

#[cfg(not(unix))]
fn same_file(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    // Without a stable file index, "both names resolve" is the best available
    // evidence, and Windows filesystems are case-insensitive in practice.
    a.len() == b.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flips_exactly_one_character() {
        assert_eq!(flip_case("Nota.md").as_deref(), Some("nota.md"));
        assert_eq!(flip_case("nota.md").as_deref(), Some("Nota.md"));
        assert_eq!(flip_case("ação.md").as_deref(), Some("Ação.md"));
        // The first cased character may be in the extension.
        assert_eq!(flip_case("123.md").as_deref(), Some("123.Md"));
    }

    #[test]
    fn no_cased_character_is_inconclusive() {
        assert_eq!(flip_case("123"), None);
        assert_eq!(flip_case(""), None);
    }
}
