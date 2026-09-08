//! What the preview IR costs, measured rather than guessed.
//!
//! `docs/ARCHITECTURE.md` §10 chooses sanitized HTML over an AST, and the
//! obvious objection is that HTML is bulkier on the wire. The instruction for
//! this milestone was explicit: *if the serialisation cost shows up in the
//! profile, record the number — do not optimise on intuition*. This is the
//! measurement, and `docs/ACCEPTANCE-0.1b.md` carries what it said.
//!
//! Ignored by default: it is a measurement, not an assertion, and a timing
//! threshold on shared CI hardware is a flake generator (the same reasoning as
//! `tests/performance.rs`).
//!
//! ```bash
//! cargo test -p notes-core --test cost -- --ignored --nocapture
//! ```

use notes_core::WorkspaceService;
use notes_model::RelPath;
use std::time::Instant;

fn fixtures() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn setup(note: &str) -> (tempfile::TempDir, tempfile::TempDir, WorkspaceService) {
    let data = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join(note), b"").unwrap();
    let mut svc = WorkspaceService::with_data_dir(data.path()).unwrap();
    svc.open_workspace(work.path()).unwrap();
    (data, work, svc)
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn measure(label: &str, svc: &WorkspaceService, src: &str) {
    let path = RelPath::parse("nota.md").unwrap();
    let rounds = if src.len() > 1_000_000 { 5 } else { 50 };

    let mut render = Vec::new();
    let mut serialise = Vec::new();
    let mut outline = Vec::new();
    let mut html_bytes = 0usize;
    let mut json_bytes = 0usize;

    for _ in 0..rounds {
        let t = Instant::now();
        let r = svc.render_markdown(&path, src).unwrap();
        render.push(t.elapsed().as_secs_f64() * 1000.0);
        html_bytes = r.html.len();

        let t = Instant::now();
        let json = serde_json::to_string(&r).unwrap();
        serialise.push(t.elapsed().as_secs_f64() * 1000.0);
        json_bytes = json.len();

        let t = Instant::now();
        let _ = svc.outline(src).unwrap();
        outline.push(t.elapsed().as_secs_f64() * 1000.0);
    }

    let (render, serialise, outline) = (median(render), median(serialise), median(outline));
    println!(
        "{label:22} source {:>9} · html {:>9} · json {:>9} | render {render:>8.3} ms · serialise {serialise:>8.3} ms ({:>4.1}% of the two) · outline {outline:>8.3} ms",
        bytes(src.len()),
        bytes(html_bytes),
        bytes(json_bytes),
        100.0 * serialise / (render + serialise).max(f64::MIN_POSITIVE),
    );
}

fn bytes(n: usize) -> String {
    if n >= 1024 * 1024 {
        format!("{:.1} MiB", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{:.1} KiB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

/// The question §10 leaves open: is turning `Rendered` into JSON a cost worth
/// engineering around, or noise beside the parse?
#[test]
#[ignore = "a measurement, not an assertion"]
fn what_the_preview_ir_costs_on_the_wire() {
    let (_d, _w, svc) = setup("nota.md");

    println!();
    // A note of the size people actually write.
    let typical = std::fs::read_to_string(fixtures().join("basic/nota-000.md"))
        .unwrap_or_else(|_| "# nota\n\ncorpo\n".to_string());
    measure("typical note", &svc, &typical);

    // The corpus's own worst case for structure rather than size.
    let mut everything = String::new();
    if let Ok(rd) = std::fs::read_dir(fixtures().join("markdown")) {
        let mut names: Vec<_> = rd
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".md") && n != "README.md")
            .collect();
        names.sort();
        for n in names {
            everything
                .push_str(&std::fs::read_to_string(fixtures().join("markdown").join(&n)).unwrap());
            everything.push_str("\n\n");
        }
    }
    measure("the whole corpus", &svc, &everything);

    // 200 copies of it: a long note, still plausible.
    let long = everything.repeat(200);
    measure("×200", &svc, &long);

    // The 5 MB edge case, which is the size at which a debounce stops hiding
    // anything.
    if let Ok(huge) = std::fs::read_to_string(fixtures().join("edge-cases/large-5mb.md")) {
        measure("large-5mb.md", &svc, &huge);
    }
    println!();
}
