//! Heading slugs, by GitHub's algorithm.
//!
//! Written here rather than pulled in for the same reason `notes-model` folds
//! NFC by hand (`docs/DECISIONS-0.1a.md` D-07): the rule is forty lines and a
//! dependency that changes it silently changes every anchor a user has ever
//! written down.
//!
//! The algorithm, as GitHub applies it: lowercase; drop everything that is not
//! a letter, a digit, a space or a hyphen — **Unicode letters count**, so
//! `Acentuação` keeps its `ç` and `ã`; turn spaces into hyphens; and when the
//! same slug appears twice, append `-1`, `-2`, and so on.

use std::collections::HashMap;

/// Slugs for one document, remembering what it has already emitted.
#[derive(Default)]
pub struct Slugger {
    seen: HashMap<String, u32>,
}

impl Slugger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn slug(&mut self, text: &str) -> String {
        let base = base_slug(text);
        match self.seen.get_mut(&base) {
            Some(n) => {
                *n += 1;
                format!("{base}-{n}")
            }
            None => {
                self.seen.insert(base.clone(), 0);
                base
            }
        }
    }
}

fn base_slug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if matches!(c, '-' | ' ' | '\t' | '\n') {
            // A hyphen stays a hyphen and whitespace becomes one; they end up
            // as the same character, which is what makes `A: B` and `A B` the
            // same anchor.
            out.push('-');
        }
        // Everything else — punctuation, emoji, symbols — is dropped, which is
        // what GitHub does and what every anchor already written expects.
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn punctuation_goes_and_letters_stay() {
        assert_eq!(
            base_slug("Um título com Acentuação e Pontuação!"),
            "um-título-com-acentuação-e-pontuação"
        );
    }

    #[test]
    fn a_repeated_heading_is_numbered_from_one() {
        let mut s = Slugger::new();
        assert_eq!(s.slug("Repetido"), "repetido");
        assert_eq!(s.slug("Repetido"), "repetido-1");
        assert_eq!(s.slug("Repetido"), "repetido-2");
    }

    #[test]
    fn different_headings_that_slug_the_same_still_collide() {
        // "A: B" and "A B" are one anchor, and the second must move. Anything
        // else would give two headings the same `id`.
        let mut s = Slugger::new();
        assert_eq!(s.slug("A: B"), "a-b");
        assert_eq!(s.slug("A B"), "a-b-1");
    }

    #[test]
    fn an_empty_heading_still_gets_a_usable_id() {
        let mut s = Slugger::new();
        assert_eq!(s.slug("🎈"), "");
        assert_eq!(s.slug("!"), "-1");
    }
}
