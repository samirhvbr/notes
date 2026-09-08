//! Where the time goes when a workspace has directories instead of notes.
//!
//! `fixtures/large` measures notes — 10 000 of them, flat, all notes. It says
//! nothing about the axis that froze the application: **directories**. This
//! measures that axis against `fixtures/deep`, and it exists because the owner
//! opened `~/x` — around 160 repositories with `node_modules/`, `target/` and
//! `.git/` — and the Welcome screen stayed on screen for over two minutes.
//!
//!     tools/gen-deep.sh
//!     cargo test -p notes-core --test deep -- --ignored --nocapture

use notes_core::WorkspaceService;
use notes_model::RelPath;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn deep() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/deep")
}

fn ms(d: Duration) -> String {
    format!("{:>9.2} ms", d.as_secs_f64() * 1000.0)
}

#[test]
#[ignore = "needs fixtures/deep — run tools/gen-deep.sh first"]
fn where_the_time_goes_opening_a_workspace_full_of_directories() {
    let root = deep();
    assert!(root.is_dir(), "run tools/gen-deep.sh first");

    let dirs = count_dirs(&root);
    let data = tempfile::tempdir().unwrap();
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();

    let t = Instant::now();
    svc.open_workspace(&root).unwrap();
    let opened = t.elapsed();

    let t = Instant::now();
    let top = svc.list_dir(&RelPath::root()).unwrap();
    let listed = t.elapsed();

    let t = Instant::now();
    let watch = svc.start_watch().unwrap();
    let watched = t.elapsed();

    let t = Instant::now();
    let quick = svc.quick_open("readme", 20);
    let indexed = t.elapsed();

    println!("directories:        {dirs}");
    println!("open_workspace:   {}", ms(opened));
    println!("list root:        {}   ({} entries)", ms(listed), top.len());
    println!("start_watch:      {}   (degraded: {watch:?})", ms(watched));
    match &quick {
        Ok(m) => println!(
            "quick_open first: {}   ({} matches, building: {}, unreadable: {})",
            ms(indexed),
            m.matches.len(),
            m.building,
            m.unreadable
        ),
        Err(e) => println!("quick_open first: {}   FAILED: {e:?}", ms(indexed)),
    }
    println!("--------------------------------");
    println!("to a usable tree: {}", ms(opened + listed));
    println!(
        "everything:       {}",
        ms(opened + listed + watched + indexed)
    );

    // The rule this milestone adds: a tree in under a second, at any size.
    // Everything that needs the whole tree belongs in the background.
    assert!(
        (opened + listed).as_secs_f64() < 1.0,
        "opening and listing took {}, over the one-second rule",
        ms(opened + listed)
    );
}

fn count_dirs(root: &Path) -> usize {
    let mut n = 0;
    let Ok(rd) = std::fs::read_dir(root) else {
        return 0;
    };
    for e in rd.flatten() {
        if e.file_type().map(|t| t.is_dir()).unwrap_or(false)
            && !e.file_type().map(|t| t.is_symlink()).unwrap_or(true)
        {
            n += 1 + count_dirs(&e.path());
        }
    }
    n
}

// ---------------------------------------------------------------------------
// The criteria. These build their own corpus and run in CI, unlike the
// measurement above, which needs `tools/gen-deep.sh` and a lot of disk.
// ---------------------------------------------------------------------------

/// A workspace shaped like the one that froze the application: repositories
/// with a `node_modules` tree, a `target/` and a `.git/`, plus the two hazards
/// that turned a slow open into a broken one — a directory nobody can read and
/// a symlink loop.
///
/// `repos = 120` gives roughly 3 000 directories, which is enough to catch a
/// synchronous whole-tree walk (it takes ~80 ms at this size, and the budget is
/// a second) without costing CI a minute of `mkdir`.
struct DeepTree {
    dir: tempfile::TempDir,
    dirs: usize,
}

impl DeepTree {
    fn build(repos: usize) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut dirs = 0;

        for r in 0..repos {
            let repo = root.join(format!("repo-{r:03}"));
            for d in [".git/objects/pack", "target/debug/deps", "docs"] {
                std::fs::create_dir_all(repo.join(d)).unwrap();
                dirs += 2;
            }
            std::fs::write(repo.join("README.md"), b"# readme\n").unwrap();
            std::fs::write(repo.join("docs/guia.md"), b"# guia\n").unwrap();
            for p in 0..6 {
                let pkg = repo.join("node_modules").join(format!("pkg-{p:02}"));
                std::fs::create_dir_all(pkg.join("dist")).unwrap();
                dirs += 2;
                // `node_modules` is full of Markdown, which is why hiding it is
                // a product decision and not an obvious one (D-08).
                std::fs::write(pkg.join("README.md"), b"# dep\n").unwrap();
            }
        }
        Self { dir, dirs }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    /// The hazards, added separately so a test can say which one it is about.
    #[cfg(unix)]
    fn add_hazards(&self) {
        use std::os::unix::fs::PermissionsExt;
        let denied = self.path().join("repo-000/docs/ead");
        std::fs::create_dir_all(&denied).unwrap();
        std::fs::write(denied.join("segredo.md"), b"# nope\n").unwrap();
        std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o000)).unwrap();

        let loopy = self.path().join("repo-001/loop");
        std::fs::create_dir_all(&loopy).unwrap();
        std::os::unix::fs::symlink(self.path(), loopy.join("up")).unwrap();
    }

    /// A `tempdir` cannot delete a directory it cannot enter, so the mode is
    /// put back before the test ends.
    #[cfg(unix)]
    fn release_hazards(&self) {
        use std::os::unix::fs::PermissionsExt;
        let denied = self.path().join("repo-000/docs/ead");
        let _ = std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o755));
    }
}

fn opened(root: &Path) -> (WorkspaceService, tempfile::TempDir) {
    let data = tempfile::tempdir().unwrap();
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(root).unwrap();
    (svc, data)
}

/// **ADR-034, the rule itself.** The tree appears in under a second at any
/// size. Measured on `fixtures/deep` (20 962 directories) it is 1.13 ms; here it
/// is a smaller corpus in CI, and the budget is the same one second.
#[test]
fn the_tree_appears_in_well_under_a_second() {
    let tree = DeepTree::build(120);
    let data = tempfile::tempdir().unwrap();
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();

    let t = Instant::now();
    svc.open_workspace(tree.path()).unwrap();
    let top = svc.list_dir(&RelPath::root()).unwrap();
    let elapsed = t.elapsed();

    assert_eq!(top.len(), 120, "the root listed");
    assert!(
        elapsed.as_secs_f64() < 1.0,
        "{} directories took {} to a usable tree, over the one-second rule",
        tree.dirs,
        ms(elapsed)
    );
}

/// **0.1b's criterion.** Starting the watcher returns immediately, because the
/// walk that installs one watch per directory is background work.
///
/// The assertion is a **ratio against the walk this test then waits for**,
/// rather than a fixed millisecond count, because a fixed count passes on a
/// fast disk for the wrong reason: at this corpus size a synchronous walk costs
/// ~30 ms, which any absolute budget worth writing would let through. What the
/// rule actually says is that returning does not include walking, and that is
/// what a ratio can state. Measured on `fixtures/deep`, where the walk is
/// 503 ms, `start_watch` still returns in well under a millisecond.
#[test]
fn starting_the_watcher_returns_immediately_and_walks_behind() {
    let tree = DeepTree::build(400);
    let (mut svc, _data) = opened(tree.path());

    let t = Instant::now();
    let degraded = svc.start_watch().unwrap();
    let returned = t.elapsed();

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut status = svc.watch_status();
    while status.walking && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
        status = svc.watch_status();
    }
    let walked = t.elapsed();

    assert!(
        returned.as_millis() < 100 && returned * 5 < walked,
        "start_watch returned in {} of a {} walk over {} directories — it is \
         walking the tree inline",
        ms(returned),
        ms(walked),
        tree.dirs
    );

    if degraded.is_none() {
        assert!(!status.walking, "the walk finished: {status:?}");
        assert!(status.dirs > 100, "the walk installed watches: {status:?}");
        // `node_modules/` and `target/` are skipped by the watcher and only by
        // the watcher (D-08), so the count is far below the directory total.
        assert!(
            status.dirs < tree.dirs / 2,
            "and skipped the machine-generated trees: {} of {}",
            status.dirs,
            tree.dirs
        );
    }
}

/// **0.1b's criterion, the second half.** One unreadable directory is counted
/// and skipped. It used to demote the whole workspace to polling, because
/// `notify`'s recursive add fails whole on the first `read_dir` it cannot do.
#[cfg(unix)]
#[test]
fn an_unreadable_directory_does_not_demote_the_workspace() {
    let tree = DeepTree::build(20);
    tree.add_hazards();
    let (mut svc, _data) = opened(tree.path());

    let degraded = svc.start_watch().unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut status = svc.watch_status();
    while status.walking && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
        status = svc.watch_status();
    }
    tree.release_hazards();

    // A machine with no inotify budget left is a real skip, not a failure.
    if degraded.is_some() {
        eprintln!("skipped: this machine cannot watch ({degraded:?})");
        return;
    }
    assert!(
        !status.walking,
        "the walk returned — the symlink loop is not followed"
    );
    assert_eq!(status.degraded, None, "the workspace is still watched");
    assert_eq!(
        status.unreadable, 1,
        "and the one directory is counted: {status:?}"
    );
    assert!(
        status.dirs > 20,
        "the rest of the tree is watched: {status:?}"
    );
}

/// **0.1c's criterion.** Quick open answers immediately on a deep workspace,
/// from a partial index, and says that it is partial.
///
/// Same shape of assertion as the watcher's, and for the same reason: the first
/// call is compared against the index build it used to contain. On
/// `fixtures/deep` that build was 550 ms, inside the command, holding the
/// service mutex. With an unreadable directory anywhere under the root it was
/// not slow but *fatal* — `Err(PermissionDenied)` for the whole workspace — so
/// the hazards are in place here.
#[test]
fn quick_open_answers_immediately_and_admits_it_is_still_indexing() {
    let tree = DeepTree::build(400);
    #[cfg(unix)]
    tree.add_hazards();
    let (svc, _data) = opened(tree.path());

    let t = Instant::now();
    let first = svc
        .quick_open("readme", 50)
        .expect("never fails on an unreadable directory");
    let returned = t.elapsed();

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut done = first.clone();
    while done.building && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
        done = svc.quick_open("readme", 50).unwrap();
    }
    let built = t.elapsed();
    #[cfg(unix)]
    tree.release_hazards();

    assert!(
        returned.as_millis() < 100 && returned * 5 < built,
        "the first quick_open took {} of a {} index build over {} directories — \
         it is walking the tree inline",
        ms(returned),
        ms(built),
        tree.dirs
    );

    assert!(!done.building, "the index finished: {done:?}");
    assert_eq!(done.matches.len(), 50, "and it matches the whole workspace");
    assert!(
        done.indexed >= 400 * 8,
        "every README under node_modules is indexed — the watcher's skip list is \
         not the visibility list (D-08): {}",
        done.indexed
    );
    #[cfg(unix)]
    assert_eq!(
        done.unreadable, 1,
        "the unreadable directory is counted, not fatal"
    );
}
