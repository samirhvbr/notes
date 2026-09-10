//! An explicit pairing preview. This command never copies or deletes notes.
use notes_sync::{pair, PairingMode};
use std::path::PathBuf;
fn main() {
    if run().is_err() {
        eprintln!("notes-sync-plan: invalid input or inaccessible workspace; use --help");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.as_slice() == ["--help"] {
        println!("notes-sync-plan MODE LOCAL_FOLDER REMOTE_FOLDER STATE_DIRECTORY\nMODE: upload | download | reconcile\nPrint a read-only pairing preview for two mounted folders. This is not remote synchronization.\nSTATE_DIRECTORY must be absolute and outside both workspaces; it holds identities only.");
        return Ok(());
    }
    if args.len() != 4 {
        return Err("invalid arguments".into());
    }
    let mode = match args[0].as_str() {
        "upload" => PairingMode::Upload,
        "download" => PairingMode::Download,
        "reconcile" => PairingMode::Reconcile,
        _ => return Err("invalid mode".into()),
    };
    let local = std::fs::canonicalize(&args[1])?;
    let remote = std::fs::canonicalize(&args[2])?;
    if local.starts_with(&remote) || remote.starts_with(&local) {
        return Err("folders must be distinct and disjoint".into());
    }
    let data = PathBuf::from(&args[3]);
    if !data.is_absolute() {
        return Err("state must be absolute".into());
    }
    notes_core::sync::validate_state_location(&[&local, &remote], &data)?;
    std::fs::create_dir_all(&data)?;
    let data = std::fs::canonicalize(data)?;
    if data.starts_with(&local) || data.starts_with(&remote) {
        return Err("state must be outside both folders".into());
    }
    let a = notes_core::sync::inventory(&local, &data.join("local"))?;
    let b = notes_core::sync::inventory(&remote, &data.join("remote"))?;
    println!("{}", serde_json::to_string_pretty(&pair(mode, &a, &b)?)?);
    Ok(())
}
