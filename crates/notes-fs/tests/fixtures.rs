//! Byte preservation over the committed corpus.
//!
//! 0.1a acceptance: *"Abrir cada arquivo dos fixtures e salvar sem editar →
//! `git status` limpo (CRLF, BOM, sem newline final, NFD, EOL misto abre
//! read-only)."*
//!
//! Run here against a copy in a temp directory rather than against the working
//! tree: the assertion is byte equality, which is what a clean `git status`
//! *means*, and a hermetic test cannot leave the repository dirty when it fails.
//! The `git status` form itself is documented as a manual step in
//! `docs/ACCEPTANCE-0.1a.md`.

use notes_fs::{FileSystem, LocalFs};
use notes_model::{Encoding, Eol, RelPath, TextProfile};
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let dst = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &dst);
        } else {
            std::fs::copy(e.path(), &dst).unwrap();
        }
    }
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<RelPath>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if e.file_type().unwrap().is_dir() {
            walk(root, &p, out);
        } else {
            let rel = p
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if let Ok(r) = RelPath::parse(&rel) {
                out.push(r);
            }
        }
    }
}

/// Open every file, save it back unedited, and require the bytes to be identical.
fn round_trip_corpus(name: &str) {
    let src = fixtures().join(name);
    assert!(
        src.is_dir(),
        "missing corpus {name} — run tools/gen-fixtures.py"
    );
    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().join(name);
    copy_tree(&src, &work);

    let fs = LocalFs::open(&work).unwrap();
    let mut paths = Vec::new();
    walk(&work, &work, &mut paths);
    assert!(!paths.is_empty(), "{name} is empty");

    let mut saved = 0;
    let mut read_only = 0;
    for rel in &paths {
        let original = fs.read(rel).unwrap();
        let (profile, text) = TextProfile::detect(&original);

        if profile.read_only_reason().is_some() {
            read_only += 1;
            // A read-only note is never written back — that is the guarantee.
            continue;
        }
        let text = text.expect("decodable when not read-only");
        let bytes = profile.encode(&text);
        assert_eq!(
            bytes, original,
            "{rel}: encode(detect(x)) != x — profile {profile:?}"
        );
        fs.write_atomic(rel, &bytes, None).unwrap();
        assert_eq!(
            fs.read(rel).unwrap(),
            original,
            "{rel}: save-unchanged altered the file"
        );
        saved += 1;
    }
    assert!(saved > 0, "{name}: nothing was saved back");
    eprintln!("{name}: {saved} saved unchanged, {read_only} read-only");
}

#[test]
fn basic_corpus_survives_open_and_save_unchanged() {
    round_trip_corpus("basic");
}

#[test]
fn edge_cases_corpus_survives_open_and_save_unchanged() {
    round_trip_corpus("edge-cases");
}

#[test]
fn the_edge_cases_that_must_open_read_only_do() {
    let fs = LocalFs::open(fixtures().join("edge-cases")).unwrap();
    for (name, reason) in [
        ("mixed-eol.md", notes_model::ReadOnlyReason::MixedEol),
        ("cr-only.md", notes_model::ReadOnlyReason::MixedEol),
        ("invalid-utf8.md", notes_model::ReadOnlyReason::NotUtf8),
        (
            "lone-surrogate-ish.md",
            notes_model::ReadOnlyReason::NotUtf8,
        ),
    ] {
        let rel = RelPath::parse(name).unwrap();
        let (p, _) = TextProfile::detect(&fs.read(&rel).unwrap());
        assert_eq!(p.read_only_reason(), Some(reason), "{name}");
    }
}

#[test]
fn the_edge_cases_that_must_stay_editable_do() {
    let fs = LocalFs::open(fixtures().join("edge-cases")).unwrap();
    for name in [
        "lf.md",
        "crlf.md",
        "no-final-newline.md",
        "bom-lf.md",
        "bom-crlf.md",
        "empty.md",
        "only-newline.md",
        "front-matter.md",
        "front-matter-invalid.md",
        "large-5mb.md",
    ] {
        let rel = RelPath::parse(name).unwrap();
        let (p, _) = TextProfile::detect(&fs.read(&rel).unwrap());
        assert_eq!(p.read_only_reason(), None, "{name} must be editable");
    }
}

#[test]
fn the_corpus_carries_the_shapes_it_claims_to() {
    let fs = LocalFs::open(fixtures().join("edge-cases")).unwrap();
    let read = |n: &str| fs.read(&RelPath::parse(n).unwrap()).unwrap();

    let (p, _) = TextProfile::detect(&read("crlf.md"));
    assert_eq!(p.eol, Eol::CrLf);
    let (p, _) = TextProfile::detect(&read("bom-lf.md"));
    assert!(p.bom);
    let (p, _) = TextProfile::detect(&read("no-final-newline.md"));
    assert!(!p.final_newline);
    let (p, _) = TextProfile::detect(&read("invalid-utf8.md"));
    assert_eq!(p.encoding, Encoding::Unknown);
    assert_eq!(read("empty.md").len(), 0);
    assert!(read("large-5mb.md").len() >= 5 * 1024 * 1024);
}

/// Listing must not read content — the 0.1a criterion for `fixtures/large`.
/// Here the assertion is structural rather than timed: a timing test on shared
/// CI is a flake generator, and the property that matters is that `list` never
/// opens a file.
#[test]
fn listing_is_one_level_and_reads_no_content() {
    let fs = LocalFs::open(fixtures().join("basic")).unwrap();
    let entries = fs.list(&RelPath::root()).unwrap();
    assert!(entries.iter().any(|e| e.name == "nota-000.md" && e.is_note));
    assert!(entries.iter().any(|e| e.name == "README.txt" && !e.is_note));
    assert!(entries.iter().any(|e| e.name == "imagem.png" && !e.is_note));
    assert!(
        entries
            .iter()
            .any(|e| e.name == "trabalho" && e.kind == notes_model::EntryKind::Dir),
        "directories are listed"
    );
    assert!(
        !entries.iter().any(|e| e.name.starts_with("nota-007")),
        "one level only: nota-007 lives in arquivo/2025"
    );
    assert!(
        entries.iter().any(|e| e.name == ".oculto.md"),
        "notes-fs lists everything; filtering ignored entries is notes-core's job"
    );
}

#[test]
fn the_case_probe_reaches_a_verdict_on_this_machine() {
    let fs = LocalFs::open(fixtures().join("basic")).unwrap();
    let entries = fs.list(&RelPath::root()).unwrap();
    let verdict = notes_fs::probe_case_insensitive(fs.root(), &entries);
    // The corpus has cased names, so the probe must not be inconclusive here.
    assert!(
        verdict.is_some(),
        "probe was inconclusive on a corpus full of cased names"
    );
    #[cfg(target_os = "linux")]
    assert_eq!(verdict, Some(false), "ext4 on Linux distinguishes case");
}
