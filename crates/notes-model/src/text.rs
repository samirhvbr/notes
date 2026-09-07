use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Encoding {
    Utf8,
    /// Not decodable as UTF-8. The note opens read-only; nothing is converted
    /// silently (scope §7.5).
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Eol {
    Lf,
    CrLf,
    /// Both in the same file, or a lone CR. Read-only until the user converts
    /// explicitly: CodeMirror normalises line breaks internally, so claiming to
    /// round-trip a mixed file would be a lie (scope §7.5).
    Mixed,
}

/// What a file's bytes look like, detected on open and **re-applied on save**.
///
/// This is the whole of front-matter preservation at 0.1a: nothing rewrites the
/// buffer, so YAML survives because no one touches it — not because a parser
/// puts it back (`ARCHITECTURE.md` §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TextProfile {
    pub encoding: Encoding,
    pub bom: bool,
    pub eol: Eol,
    pub final_newline: bool,
}

const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

impl TextProfile {
    /// Detect the profile and decode. Returns the text with the BOM stripped
    /// and every line ending normalised to `\n` — the editor only ever sees
    /// `\n`, and [`Self::encode`] puts the file's own shape back.
    pub fn detect(bytes: &[u8]) -> (TextProfile, Option<String>) {
        let bom = bytes.starts_with(&BOM);
        let body = if bom { &bytes[3..] } else { bytes };

        let Ok(text) = std::str::from_utf8(body) else {
            return (
                TextProfile {
                    encoding: Encoding::Unknown,
                    bom,
                    eol: Eol::Lf,
                    final_newline: false,
                },
                None,
            );
        };

        let crlf = text.matches("\r\n").count();
        let total_lf = text.matches('\n').count();
        let lone_lf = total_lf - crlf;
        let lone_cr = text.matches('\r').count() - crlf;

        let eol = match (crlf, lone_lf, lone_cr) {
            (_, _, c) if c > 0 => Eol::Mixed,
            (0, _, _) => Eol::Lf,
            (_, 0, _) => Eol::CrLf,
            _ => Eol::Mixed,
        };

        let final_newline = text.ends_with('\n');
        let normalised = if eol == Eol::CrLf {
            text.replace("\r\n", "\n")
        } else {
            text.to_string()
        };

        (
            TextProfile {
                encoding: Encoding::Utf8,
                bom,
                eol,
                final_newline,
            },
            Some(normalised),
        )
    }

    /// Turn editor text back into the file's own bytes.
    ///
    /// `final_newline` is applied as the file had it: a file that ended without
    /// one still ends without one after an unchanged save, which is what makes
    /// `git status` clean.
    pub fn encode(&self, text: &str) -> Vec<u8> {
        let mut s = if self.eol == Eol::CrLf {
            text.replace('\n', "\r\n")
        } else {
            text.to_string()
        };
        let ends = if self.eol == Eol::CrLf {
            s.ends_with("\r\n")
        } else {
            s.ends_with('\n')
        };
        if self.final_newline && !ends && !s.is_empty() {
            s.push_str(if self.eol == Eol::CrLf { "\r\n" } else { "\n" });
        } else if !self.final_newline && ends {
            if self.eol == Eol::CrLf {
                s.truncate(s.len() - 2)
            } else {
                s.truncate(s.len() - 1)
            }
        }
        let mut out = Vec::with_capacity(s.len() + 3);
        if self.bom {
            out.extend_from_slice(&BOM);
        }
        out.extend_from_slice(s.as_bytes());
        out
    }

    /// Why the editor is disabled, if it is.
    pub fn read_only_reason(&self) -> Option<crate::ReadOnlyReason> {
        match (self.encoding, self.eol) {
            (Encoding::Unknown, _) => Some(crate::ReadOnlyReason::NotUtf8),
            (_, Eol::Mixed) => Some(crate::ReadOnlyReason::MixedEol),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The acceptance criterion, as a unit test: detect then encode must be the
    /// identity on bytes, for every shape in `fixtures/edge-cases`.
    #[track_caller]
    fn round_trips(bytes: &[u8]) {
        let (profile, text) = TextProfile::detect(bytes);
        let text = text.expect("valid utf-8 for this case");
        assert_eq!(
            profile.encode(&text),
            bytes,
            "profile {profile:?} lost bytes"
        );
    }

    #[test]
    fn round_trips_every_line_ending_shape() {
        round_trips(b"# lf\n\nlinha um\nlinha dois\n");
        round_trips(b"# crlf\r\n\r\nlinha um\r\nlinha dois\r\n");
        round_trips(b"# sem newline final\n\nultima linha");
        round_trips(b"# crlf sem newline final\r\n\r\nultima");
        round_trips(b"");
        round_trips(b"\n");
        round_trips(b"   \n\t\n");
        round_trips("acentua\u{e7}\u{e3}o e emoji \u{1f331}\n".as_bytes());
    }

    #[test]
    fn round_trips_with_a_bom() {
        round_trips(b"\xef\xbb\xbf# bom + lf\n\ncorpo\n");
        round_trips(b"\xef\xbb\xbf# bom + crlf\r\n\r\ncorpo\r\n");
    }

    #[test]
    fn bom_is_hidden_from_the_editor_and_restored_on_save() {
        let (p, text) = TextProfile::detect(b"\xef\xbb\xbf# t\n");
        assert!(p.bom);
        assert_eq!(
            text.as_deref(),
            Some("# t\n"),
            "the editor never sees a BOM"
        );
        assert_eq!(p.encode("# t\n"), b"\xef\xbb\xbf# t\n");
    }

    #[test]
    fn crlf_is_hidden_from_the_editor_and_restored_on_save() {
        let (p, text) = TextProfile::detect(b"a\r\nb\r\n");
        assert_eq!(p.eol, Eol::CrLf);
        assert_eq!(text.as_deref(), Some("a\nb\n"), "the editor only sees \\n");
        assert_eq!(p.encode("a\nb\n"), b"a\r\nb\r\n");
    }

    #[test]
    fn mixed_and_cr_only_are_read_only() {
        let (p, _) = TextProfile::detect(b"a\nb\r\nc\n");
        assert_eq!(p.eol, Eol::Mixed);
        assert_eq!(p.read_only_reason(), Some(crate::ReadOnlyReason::MixedEol));

        let (p, _) = TextProfile::detect(b"a\rb\rc\r");
        assert_eq!(
            p.eol,
            Eol::Mixed,
            "a lone CR is not a line ending we round-trip"
        );
    }

    #[test]
    fn invalid_utf8_is_read_only_and_yields_no_text() {
        let (p, text) = TextProfile::detect(b"# x\n\n\xff\xfe\n");
        assert_eq!(p.encoding, Encoding::Unknown);
        assert!(
            text.is_none(),
            "no lossy decode; the bytes are not shown as text"
        );
        assert_eq!(p.read_only_reason(), Some(crate::ReadOnlyReason::NotUtf8));
    }

    #[test]
    fn an_edit_keeps_the_files_own_shape() {
        let (p, text) = TextProfile::detect(b"\xef\xbb\xbfa\r\nb");
        let edited = text.unwrap() + "\nc";
        assert_eq!(p.encode(&edited), b"\xef\xbb\xbfa\r\nb\r\nc");
    }
}
