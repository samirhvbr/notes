use notes_model::Entry;

/// Entries hidden from the tree on every workspace, whether or not `.notes/`
/// exists.
///
/// Scope §7.6 requires this list *by default*, and `.notes/config.json` is off
/// by default — so it cannot live there. A default that only exists once the
/// user opts in is not a default (`ARCHITECTURE.md` §16).
pub const IGNORE_DEFAULT: &[&str] = &[".notes", ".git", ".obsidian", ".trash"];

/// Should this entry be shown?
///
/// `extra` comes from `.notes/config.json` when the user turns portable
/// settings on. **It extends the list and can never shrink it** — a config file
/// that can unhide `.git/` is a foot-gun with no use case.
pub fn is_hidden(entry: &Entry, show_hidden: bool, extra: &[String]) -> bool {
    let name = entry.name.as_str();
    if IGNORE_DEFAULT.contains(&name) || extra.iter().any(|e| e == name) {
        return true;
    }
    if name.starts_with('.') && !show_hidden {
        return true;
    }
    // Our own temporary files: `.<name>.tmp`. Always hidden, even with
    // `show_hidden`, because a half-written note appearing in the tree is a
    // note the user will try to open — and a `SIGKILL` can leave one behind.
    if name.starts_with('.') && name.ends_with(".tmp") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use notes_model::{EntryKind, RelPath};

    fn entry(name: &str) -> Entry {
        Entry {
            path: RelPath::parse(name).unwrap(),
            name: name.to_string(),
            kind: EntryKind::File,
            size: Some(0),
            is_note: RelPath::parse(name).unwrap().is_note(),
        }
    }

    #[test]
    fn the_default_list_applies_without_any_config() {
        for n in [".notes", ".git", ".obsidian", ".trash"] {
            assert!(
                is_hidden(&entry(n), false, &[]),
                "{n} must be hidden by default"
            );
            assert!(
                is_hidden(&entry(n), true, &[]),
                "{n} stays hidden even with show_hidden"
            );
        }
    }

    #[test]
    fn dot_files_follow_show_hidden_but_the_default_list_does_not() {
        assert!(is_hidden(&entry(".oculto.md"), false, &[]));
        assert!(!is_hidden(&entry(".oculto.md"), true, &[]));
    }

    #[test]
    fn extra_entries_extend_and_never_replace() {
        let extra = vec!["build".to_string()];
        assert!(is_hidden(&entry("build"), true, &extra));
        assert!(
            is_hidden(&entry(".git"), true, &extra),
            "config cannot unhide .git"
        );
    }

    #[test]
    fn our_own_temporary_files_are_never_shown() {
        assert!(is_hidden(&entry(".nota.md.tmp"), true, &[]));
    }

    #[test]
    fn ordinary_notes_are_shown() {
        assert!(!is_hidden(&entry("nota.md"), false, &[]));
        assert!(!is_hidden(&entry("Notas Gerais.md"), false, &[]));
    }
}
