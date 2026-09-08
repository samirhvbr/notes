//! Workspace search, milestone 0.1c.
//!
//! Two different things that stay different, because scope §10 says so and
//! because conflating them is how a search box stops being predictable:
//!
//! * **Quick open** matches *paths* and must feel instant. It is fuzzy, it works
//!   from a cached list held in memory, and it never reads a file.
//! * **Global search** matches *content* by scanning the workspace — the `ignore`
//!   crate for the walk and `regex` for the match. Results stream, and it is
//!   cancellable. FTS5 arrives at 0.2 and takes over word search; **this
//!   scanner does not go away then**, because literal and regex are what it is
//!   for, and §10 requires the three semantics to keep their names rather than
//!   swapping underneath the user.
//!
//! It searches **what is on disk**. A dirty buffer has not been searched, and
//! the interface has to say so rather than let the user infer it.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use notes_model::{CoreError, RelPath};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ignore::is_hidden_name;

/// How the query is read. `Words` belongs to 0.2 and FTS5; it is named here so
/// the interface can show three stable semantics from the start (§10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SearchMode {
    Literal,
    Regex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SearchOpts {
    pub mode: SearchMode,
    pub case_sensitive: bool,
}

impl Default for SearchOpts {
    fn default() -> Self {
        Self {
            mode: SearchMode::Literal,
            case_sensitive: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(transparent)]
#[ts(export, type = "number")]
pub struct SearchId(#[ts(type = "number")] pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SearchHit {
    pub path: RelPath,
    /// One-based, because it is shown to a person.
    pub line: u32,
    pub col: u32,
    /// The matching line, trimmed. Never the whole file.
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SearchProgress {
    pub id: SearchId,
    /// Hits since the last poll. Drained, so a caller never sees one twice.
    pub hits: Vec<SearchHit>,
    pub files_scanned: usize,
    pub total_hits: usize,
    pub done: bool,
    pub cancelled: bool,
    /// True when the cap was reached and the walk stopped early, so the
    /// interface can say "showing the first N" instead of implying there are no
    /// more.
    pub truncated: bool,
}

/// A ceiling on hits. A query of `e` over ten thousand notes is a request to
/// stream a million lines into a WebView, and the honest answer is the first
/// few thousand plus the fact that it was cut.
const MAX_HITS: usize = 2_000;

/// Files larger than this are skipped by content search. The corpus has a 5 MB
/// note on purpose; a 200 MB one is not a note and scanning it stalls the walk.
const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;

const CONTEXT_MAX: usize = 200;

struct Shared {
    hits: Mutex<Vec<SearchHit>>,
    cancel: AtomicBool,
    done: AtomicBool,
    scanned: AtomicUsize,
    total: AtomicUsize,
    truncated: AtomicBool,
}

pub struct Search {
    id: SearchId,
    shared: Arc<Shared>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl Search {
    pub fn id(&self) -> SearchId {
        self.id
    }

    /// Start scanning `root` in the background.
    ///
    /// The walk is parallel and every worker checks the cancel flag before each
    /// file, so cancelling is bounded by one file rather than by the workspace.
    pub fn start(root: &std::path::Path, query: &str, opts: SearchOpts) -> Result<Self, CoreError> {
        let pattern = match opts.mode {
            SearchMode::Literal => regex::escape(query),
            SearchMode::Regex => query.to_string(),
        };
        let re = regex::RegexBuilder::new(&pattern)
            .case_insensitive(!opts.case_sensitive)
            .size_limit(1 << 20)
            .build()
            .map_err(|e| CoreError::InvalidPath {
                path: query.to_string(),
                reason: format!("invalid search pattern: {e}"),
            })?;

        let shared = Arc::new(Shared {
            hits: Mutex::new(Vec::new()),
            cancel: AtomicBool::new(false),
            done: AtomicBool::new(false),
            scanned: AtomicUsize::new(0),
            total: AtomicUsize::new(0),
            truncated: AtomicBool::new(false),
        });
        let id = SearchId(NEXT_ID.fetch_add(1, Ordering::Relaxed));

        let root = root.to_path_buf();
        let bg = Arc::clone(&shared);
        std::thread::spawn(move || {
            scan(&root, &re, &bg);
            bg.done.store(true, Ordering::Release);
        });

        Ok(Self { id, shared })
    }

    /// Take everything found since the last call.
    pub fn poll(&self) -> SearchProgress {
        let hits = std::mem::take(&mut *self.shared.hits.lock().unwrap());
        SearchProgress {
            id: self.id,
            hits,
            files_scanned: self.shared.scanned.load(Ordering::Relaxed),
            total_hits: self.shared.total.load(Ordering::Relaxed),
            done: self.shared.done.load(Ordering::Acquire),
            cancelled: self.shared.cancel.load(Ordering::Relaxed),
            truncated: self.shared.truncated.load(Ordering::Relaxed),
        }
    }

    pub fn cancel(&self) {
        self.shared.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for Search {
    /// Dropping a search stops it. Without this, replacing one search with the
    /// next would leave the first walking a 197 MiB corpus for nobody.
    fn drop(&mut self) {
        self.cancel();
    }
}

fn scan(root: &std::path::Path, re: &regex::Regex, shared: &Arc<Shared>) {
    // Every ignore source of the `ignore` crate is off: scope §7.6 says the
    // user's `.gitignore` is not a visibility policy, and the list that governs
    // this workspace is `IGNORE_DEFAULT`.
    let walker = ignore::WalkBuilder::new(root)
        .standard_filters(false)
        .hidden(false)
        .follow_links(false)
        .threads(
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
        )
        .filter_entry(|e| {
            e.file_name()
                .to_str()
                .map(|n| !is_hidden_name(n))
                .unwrap_or(false)
        })
        .build_parallel();

    walker.run(|| {
        let shared = Arc::clone(shared);
        let re = re.clone();
        let root = root.to_path_buf();
        Box::new(move |entry| {
            if shared.cancel.load(Ordering::Relaxed) || shared.done.load(Ordering::Acquire) {
                return ignore::WalkState::Quit;
            }
            let Ok(entry) = entry else {
                return ignore::WalkState::Continue;
            };
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                return ignore::WalkState::Continue;
            }
            let Ok(rel_str) = entry
                .path()
                .strip_prefix(&root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
            else {
                return ignore::WalkState::Continue;
            };
            let Ok(rel) = RelPath::parse(&rel_str) else {
                return ignore::WalkState::Continue;
            };
            if !rel.is_note() {
                return ignore::WalkState::Continue;
            }
            if entry
                .metadata()
                .map(|m| m.len() > MAX_FILE_BYTES)
                .unwrap_or(true)
            {
                return ignore::WalkState::Continue;
            }

            shared.scanned.fetch_add(1, Ordering::Relaxed);
            let Ok(bytes) = std::fs::read(entry.path()) else {
                return ignore::WalkState::Continue;
            };
            // Not valid UTF-8 is not an error here: it opens read-only in the
            // editor and it is not searched.
            let Ok(text) = std::str::from_utf8(&bytes) else {
                return ignore::WalkState::Continue;
            };

            let mut found = Vec::new();
            for (i, line) in text.lines().enumerate() {
                if let Some(m) = re.find(line) {
                    found.push(SearchHit {
                        path: rel.clone(),
                        line: i as u32 + 1,
                        col: line[..m.start()].chars().count() as u32 + 1,
                        context: trim_context(line),
                    });
                }
            }
            if found.is_empty() {
                return ignore::WalkState::Continue;
            }

            let total = shared.total.fetch_add(found.len(), Ordering::Relaxed) + found.len();
            shared.hits.lock().unwrap().extend(found);
            if total >= MAX_HITS {
                shared.truncated.store(true, Ordering::Relaxed);
                return ignore::WalkState::Quit;
            }
            ignore::WalkState::Continue
        })
    });
}

fn trim_context(line: &str) -> String {
    let line = line.trim_end();
    if line.chars().count() <= CONTEXT_MAX {
        return line.to_string();
    }
    let cut: String = line.chars().take(CONTEXT_MAX).collect();
    format!("{cut}…")
}

// ---------------------------------------------------------------------------
// Quick open
// ---------------------------------------------------------------------------

/// What quick open answers with.
///
/// `building` is not decoration: the list is walked on a thread, so a call made
/// while it fills matches a **partial** workspace, and an interface that showed
/// that as "no such note" would be lying. `indexed` and `unreadable` give it the
/// numbers to say so.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct QuickOpen {
    pub matches: Vec<QuickMatch>,
    #[ts(type = "number")]
    pub indexed: usize,
    pub building: bool,
    #[ts(type = "number")]
    pub unreadable: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct QuickMatch {
    pub path: RelPath,
    pub name: String,
    #[ts(type = "number")]
    pub score: i32,
}

/// Fuzzy-match `query` against a list of paths already in memory.
///
/// The scoring is deliberately small and explainable rather than clever: a
/// subsequence match, with a bonus for consecutive characters and for landing at
/// the start of a path segment, and a penalty for how far in the match begins.
/// The file name is scored ahead of the directory, because `Ctrl+P` is how a
/// person reaches for a file they can name.
pub fn quick_match(paths: &[RelPath], query: &str, limit: usize) -> Vec<QuickMatch> {
    let q = query.trim();
    if q.is_empty() {
        let mut out: Vec<_> = paths
            .iter()
            .take(limit)
            .map(|p| QuickMatch {
                name: p.file_name().to_string(),
                path: p.clone(),
                score: 0,
            })
            .collect();
        out.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));
        return out;
    }
    let needle: Vec<char> = q.to_lowercase().chars().collect();

    let mut scored: Vec<QuickMatch> = paths
        .iter()
        .filter_map(|p| {
            let name = p.file_name();
            let by_name = score(name, &needle).map(|s| s + 40);
            let by_path = score(p.as_str(), &needle);
            let best = match (by_name, by_path) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b),
            }?;
            Some(QuickMatch {
                path: p.clone(),
                name: name.to_string(),
                score: best,
            })
        })
        .collect();

    scored.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.path.as_str().len().cmp(&b.path.as_str().len()))
            .then_with(|| a.path.as_str().cmp(b.path.as_str()))
    });
    scored.truncate(limit);
    scored
}

fn score(haystack: &str, needle: &[char]) -> Option<i32> {
    let hay: Vec<char> = haystack.to_lowercase().chars().collect();
    let raw: Vec<char> = haystack.chars().collect();
    let mut score = 0i32;
    let mut hi = 0usize;
    let mut previous_hit: Option<usize> = None;

    for &want in needle {
        let found = hay[hi..].iter().position(|&c| c == want)? + hi;
        score += match previous_hit {
            Some(prev) if found == prev + 1 => 15, // consecutive
            _ => 0,
        };
        let at_boundary = found == 0
            || matches!(
                raw.get(found - 1),
                Some('/') | Some('-') | Some('_') | Some(' ') | Some('.')
            );
        if at_boundary {
            score += 10;
        }
        score += 5;
        previous_hit = Some(found);
        hi = found + 1;
    }
    // Prefer a match that starts early and a shorter haystack around it.
    score -= (hi as i32 - needle.len() as i32).min(20);
    Some(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> RelPath {
        RelPath::parse(s).unwrap()
    }

    #[test]
    fn quick_open_finds_a_subsequence_anywhere_in_the_path() {
        let paths = vec![
            p("infra/servidor.md"),
            p("pessoal/ideias.md"),
            p("trabalho/projetos.md"),
        ];
        let hits = quick_match(&paths, "serv", 10);
        assert_eq!(hits[0].path, p("infra/servidor.md"));
    }

    #[test]
    fn the_file_name_outranks_the_directory() {
        // "nota" is in the directory of the first and the name of the second.
        let paths = vec![p("nota/outra-coisa.md"), p("arquivo/nota.md")];
        let hits = quick_match(&paths, "nota", 10);
        assert_eq!(hits[0].path, p("arquivo/nota.md"), "got {hits:#?}");
    }

    #[test]
    fn consecutive_characters_beat_scattered_ones() {
        let paths = vec![p("s-e-r-v-i-d-o-r.md"), p("servidor.md")];
        let hits = quick_match(&paths, "servidor", 10);
        assert_eq!(hits[0].path, p("servidor.md"));
    }

    #[test]
    fn a_query_that_does_not_match_returns_nothing() {
        let paths = vec![p("infra/servidor.md")];
        assert!(quick_match(&paths, "zzz", 10).is_empty());
    }

    #[test]
    fn an_empty_query_lists_the_start_of_the_workspace() {
        let paths = vec![p("b.md"), p("a.md")];
        let hits = quick_match(&paths, "  ", 10);
        assert_eq!(hits.len(), 2);
        assert_eq!(
            hits[0].path,
            p("a.md"),
            "sorted, so the list does not jump around"
        );
    }

    #[test]
    fn matching_ignores_case_but_the_name_shown_does_not() {
        let paths = vec![p("Infra/Servidor.md")];
        let hits = quick_match(&paths, "servidor", 10);
        assert_eq!(hits[0].name, "Servidor.md");
    }

    #[test]
    fn the_limit_is_respected() {
        let paths: Vec<_> = (0..50).map(|i| p(&format!("nota-{i:02}.md"))).collect();
        assert_eq!(quick_match(&paths, "nota", 7).len(), 7);
    }

    #[test]
    fn context_is_trimmed_rather_than_streamed_whole() {
        let long = "x".repeat(5_000);
        let out = trim_context(&long);
        assert!(out.chars().count() <= CONTEXT_MAX + 1);
        assert!(out.ends_with('…'));
    }
}
