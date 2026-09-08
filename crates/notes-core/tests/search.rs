//! Milestone 0.1c: quick open and global search.
//!
//! The timed criterion needs `fixtures/large` and is `#[ignore]`d; everything
//! else runs in the ordinary suite.

use notes_core::search::{QuickMatch, SearchMode, SearchOpts};
use notes_core::WorkspaceService;
use notes_model::{CoreError, RelPath};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

struct Fixture {
    _data: tempfile::TempDir,
    work: tempfile::TempDir,
    svc: WorkspaceService,
}

fn setup() -> Fixture {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let w = work.path();
    std::fs::write(
        w.join("servidor.md"),
        b"# Servidor\n\nconfigurar o firewall\nlinha dois\n",
    )
    .unwrap();
    std::fs::write(w.join("outra.md"), b"# Outra\n\nnada aqui\n").unwrap();
    std::fs::create_dir(w.join("infra")).unwrap();
    std::fs::write(
        w.join("infra/rede.md"),
        b"# Rede\n\no firewall do cliente\n",
    )
    .unwrap();
    std::fs::write(w.join("leiame.txt"), b"firewall em um nao-nota\n").unwrap();
    std::fs::create_dir(w.join(".git")).unwrap();
    std::fs::write(w.join(".git/config.md"), b"firewall dentro do .git\n").unwrap();

    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(w).unwrap();
    Fixture {
        _data: data,
        work,
        svc,
    }
}

/// Run a search to completion, with a ceiling so a bug cannot hang the suite.
fn run(
    svc: &mut WorkspaceService,
    query: &str,
    opts: SearchOpts,
) -> Vec<notes_core::search::SearchHit> {
    let id = svc.search_start(query, opts).unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut all = Vec::new();
    loop {
        let p = svc.search_poll(id).unwrap();
        all.extend(p.hits);
        if p.done || Instant::now() > deadline {
            assert!(p.done, "search did not finish within the ceiling");
            return all;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

// ---------------------------------------------------------------------------
// Global search
// ---------------------------------------------------------------------------

#[test]
fn a_literal_search_reports_path_line_column_and_the_line() {
    let mut f = setup();
    let hits = run(&mut f.svc, "firewall", SearchOpts::default());
    let mut paths: Vec<_> = hits.iter().map(|h| h.path.as_str().to_string()).collect();
    paths.sort();
    assert_eq!(paths, vec!["infra/rede.md", "servidor.md"]);

    let s = hits
        .iter()
        .find(|h| h.path.as_str() == "servidor.md")
        .unwrap();
    assert_eq!(s.line, 3, "one-based, because a person reads it");
    assert_eq!(s.col, 14);
    assert_eq!(s.context, "configurar o firewall");
}

#[test]
fn only_notes_are_searched_and_the_ignore_list_is_honoured() {
    let mut f = setup();
    let hits = run(&mut f.svc, "firewall", SearchOpts::default());
    assert!(
        !hits.iter().any(|h| h.path.as_str().ends_with(".txt")),
        "a .txt is not a note"
    );
    assert!(
        !hits.iter().any(|h| h.path.as_str().contains(".git")),
        "a hit inside .git/ is never what was being looked for"
    );
}

#[test]
fn search_is_case_insensitive_unless_asked() {
    let mut f = setup();
    assert_eq!(run(&mut f.svc, "SERVIDOR", SearchOpts::default()).len(), 1);
    let strict = SearchOpts {
        mode: SearchMode::Literal,
        case_sensitive: true,
    };
    assert!(run(&mut f.svc, "SERVIDOR", strict).is_empty());
}

#[test]
fn a_literal_query_is_not_read_as_a_pattern() {
    let mut f = setup();
    std::fs::write(f.work.path().join("meta.md"), b"custa R$ 1.50 (a.b)\n").unwrap();
    // `.` and `(` would match far more as a regex than as text.
    let hits = run(&mut f.svc, "(a.b)", SearchOpts::default());
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path.as_str(), "meta.md");
}

#[test]
fn regex_mode_is_available_and_named() {
    let mut f = setup();
    let opts = SearchOpts {
        mode: SearchMode::Regex,
        case_sensitive: false,
    };
    let hits = run(&mut f.svc, r"fire\w+", opts);
    assert_eq!(hits.len(), 2);
}

#[test]
fn an_invalid_pattern_is_refused_rather_than_scanning_for_nothing() {
    let mut f = setup();
    let opts = SearchOpts {
        mode: SearchMode::Regex,
        case_sensitive: false,
    };
    let err = f.svc.search_start("[unclosed", opts).unwrap_err();
    assert!(matches!(err, CoreError::InvalidPath { .. }), "got {err:?}");
}

#[test]
fn starting_a_search_cancels_the_previous_one() {
    let mut f = setup();
    let first = f
        .svc
        .search_start("firewall", SearchOpts::default())
        .unwrap();
    let second = f
        .svc
        .search_start("servidor", SearchOpts::default())
        .unwrap();
    assert_ne!(first, second);

    // A poll for the old id says "done, cancelled" rather than failing: by now
    // the user has typed again, and that is not an error.
    let stale = f.svc.search_poll(first).unwrap();
    assert!(stale.done && stale.cancelled);
    assert!(stale.hits.is_empty());
}

#[test]
fn cancelling_stops_the_walk() {
    let mut f = setup();
    let id = f
        .svc
        .search_start("firewall", SearchOpts::default())
        .unwrap();
    f.svc.search_cancel(id).unwrap();
    let p = f.svc.search_poll(id).unwrap();
    assert!(p.cancelled);
}

#[test]
fn a_query_with_no_match_finishes_cleanly() {
    let mut f = setup();
    let hits = run(&mut f.svc, "zzzz-nao-existe", SearchOpts::default());
    assert!(hits.is_empty());
}

#[test]
fn searching_with_no_workspace_open_is_an_error_not_a_panic() {
    let data = tempfile::tempdir().unwrap();
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    assert!(matches!(
        svc.search_start("x", SearchOpts::default()),
        Err(CoreError::NoWorkspace)
    ));
}

/// Scope §10: search reads **what is on disk**. A dirty buffer has not been
/// searched, and the interface has to say so — this is the fact it says.
#[test]
fn search_reads_the_disk_not_the_open_buffer() {
    let mut f = setup();
    let opened = f
        .svc
        .open_note(&RelPath::parse("outra.md").unwrap())
        .unwrap();
    // Typed, never saved.
    f.svc
        .write_draft(
            opened.note_id,
            "# Outra\n\nagora com firewall\n",
            1,
            &opened.base_rev,
            notes_core::DraftReason::Stale,
        )
        .unwrap();

    let hits = run(&mut f.svc, "firewall", SearchOpts::default());
    assert!(
        !hits.iter().any(|h| h.path.as_str() == "outra.md"),
        "an unsaved buffer is not on disk and must not be reported as found"
    );
}

// ---------------------------------------------------------------------------
// Quick open
// ---------------------------------------------------------------------------

/// Quick open's answer once its index has finished filling.
///
/// The path list is built on a background thread (ADR-034), so a call made
/// immediately after the tree changed matches a **partial** workspace and says
/// so with `building: true`. That is the right answer for a palette on a
/// keystroke and the wrong one for a test about *what is in the list*, so these
/// tests wait — which is a wait of microseconds on a five-note fixture, and is
/// the only thing about quick open that this change altered.
fn quick(svc: &notes_core::WorkspaceService, query: &str, limit: usize) -> Vec<QuickMatch> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let r = svc.quick_open(query, limit).unwrap();
        if !r.building || std::time::Instant::now() > deadline {
            return r.matches;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn quick_open_matches_paths_without_reading_files() {
    let f = setup();
    let hits = quick(&f.svc, "rede", 10);
    assert_eq!(hits[0].path.as_str(), "infra/rede.md");
    assert_eq!(hits[0].name, "rede.md");
}

#[test]
fn quick_open_lists_only_notes() {
    let f = setup();
    let all = quick(&f.svc, "", 100);
    let paths: Vec<_> = all.iter().map(|m| m.path.as_str()).collect();
    assert!(paths.contains(&"servidor.md"));
    assert!(paths.contains(&"infra/rede.md"));
    assert!(!paths.iter().any(|p| p.ends_with(".txt")));
    assert!(!paths.iter().any(|p| p.contains(".git")));
}

#[test]
fn a_note_created_after_the_cache_was_built_is_still_offered() {
    let mut f = setup();
    // Build the cache…
    assert!(quick(&f.svc, "terceira", 10).is_empty());
    // …then change the tree through the application.
    f.svc.create_note(&RelPath::root(), "terceira").unwrap();
    let hits = quick(&f.svc, "terceira", 10);
    assert_eq!(
        hits.len(),
        1,
        "the cache must not outlive the tree it described"
    );
}

#[test]
fn a_note_renamed_is_no_longer_offered_under_its_old_name() {
    let mut f = setup();
    assert!(!quick(&f.svc, "servidor", 10).is_empty());
    f.svc
        .rename_entry(&RelPath::parse("servidor.md").unwrap(), "maquina.md")
        .unwrap();
    let old = quick(&f.svc, "servidor", 10);
    assert!(old.is_empty(), "got {old:?}");
    assert!(!quick(&f.svc, "maquina", 10).is_empty());
}

// ---------------------------------------------------------------------------
// The timed criterion
// ---------------------------------------------------------------------------

fn large() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/large")
}

/// "Busca global em `fixtures/large` entrega o primeiro resultado em <500ms e é
/// cancelável."
#[test]
#[ignore = "needs fixtures/large — run tools/gen-large.sh first"]
fn the_first_result_arrives_in_under_500ms_and_the_search_can_be_cancelled() {
    let root = large();
    assert!(root.is_dir(), "run tools/gen-large.sh first");
    let data = tempfile::tempdir().unwrap();
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(&root).unwrap();

    // A word the generator puts in every note, so the corpus is not the variable
    // under test — the time to the *first* hit is.
    let t0 = Instant::now();
    let id = svc.search_start("firewall", SearchOpts::default()).unwrap();
    let mut first: Option<Duration> = None;
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut seen = 0usize;
    while Instant::now() < deadline {
        let p = svc.search_poll(id).unwrap();
        if first.is_none() && !p.hits.is_empty() {
            first = Some(t0.elapsed());
        }
        seen += p.hits.len();
        if p.done {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let first = first.expect("no hit at all");
    println!("first result: {first:?} · hits seen: {seen}");
    assert!(
        first < Duration::from_millis(500),
        "first result took {first:?}"
    );

    // Cancellable: start another and stop it while it is still walking.
    let id2 = svc.search_start("servidor", SearchOpts::default()).unwrap();
    let t1 = Instant::now();
    svc.search_cancel(id2).unwrap();
    let stopped = t1.elapsed();
    println!("cancel returned in {stopped:?}");
    assert!(
        stopped < Duration::from_millis(500),
        "cancel blocked for {stopped:?}"
    );
    assert!(svc.search_poll(id2).unwrap().cancelled);
}
