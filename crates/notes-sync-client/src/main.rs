use notes_sync_client::{
    remote::{Endpoint, Remote},
    state::{Mode, Store},
    Error, Result,
};
use std::path::{Path, PathBuf};
fn main() {
    if let Err(error) = run() {
        eprintln!("notes-sync-client: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.as_slice() == ["--help"] {
        println!("notes-sync-client init-upload|init-receive STATE ROOT ORIGIN WORKSPACE TOKEN_FILE [--allow-private]\nnotes-sync-client stage|status|received STATE\nnotes-sync-client transfer|acknowledge STATE TOKEN_FILE\nnotes-sync-client export STATE REVISION_UUID\nnotes-sync-client apply STATE APP_DATA_DIRECTORY\nSTATE and TOKEN_FILE must be absolute; STATE stays outside ROOT.\nTransfer only stores revisions. Explicit apply writes creations/updates while the workspace is closed and draft-free.\nUpload starts with an empty server inbox. Whole-workspace credentials only.\nHTTPS is required; --allow-private also permits HTTP at a literal loopback address.\nOptional NOTES_SYNC_CA_FILE adds an operator-selected PEM trust anchor.");
        return Ok(());
    }
    let ca = std::env::var_os("NOTES_SYNC_CA_FILE").map(PathBuf::from);
    let command = args.first().ok_or(Error::Invalid)?.as_str();
    let state = args.get(1).ok_or(Error::Invalid)?;
    let store = Store::open(Path::new(state))?;
    match command {
        "init-upload" | "init-receive"
            if args.len() == 6 || (args.len() == 7 && args[6] == "--allow-private") =>
        {
            let endpoint = Endpoint {
                origin: args[3].clone(),
                name: args[4].clone(),
                allow_private: args.len() == 7,
            };
            let mut remote = Remote::connect(&endpoint, Path::new(&args[5]), ca.as_deref())?;
            store.initialize(
                Path::new(&args[2]),
                endpoint,
                if command == "init-upload" {
                    Mode::Upload
                } else {
                    Mode::Receive
                },
                &mut remote,
            )?;
        }
        "stage" if args.len() == 2 => {
            let missing = store.stage()?;
            println!(
                "{}",
                serde_json::json!({"missing_tracked_notes":missing,"deletions_queued":false})
            );
        }
        "status" if args.len() == 2 => {}
        "received" if args.len() == 2 => {
            println!(
                "{}",
                serde_json::to_string_pretty(&store.received()?).map_err(|_| Error::Invalid)?
            );
            return Ok(());
        }
        "transfer" if args.len() == 3 => {
            let mut remote =
                Remote::connect(&store.endpoint()?, Path::new(&args[2]), ca.as_deref())?;
            store.transfer(&mut remote)?;
        }
        "acknowledge" if args.len() == 3 => {
            let mut remote =
                Remote::connect(&store.endpoint()?, Path::new(&args[2]), ca.as_deref())?;
            let count = store.acknowledge(&mut remote)?;
            println!("{}", serde_json::json!({"newly_acknowledged":count}));
        }
        "apply" if args.len() == 3 => {
            let count = store.apply(Path::new(&args[2]))?;
            println!("{}", serde_json::json!({"newly_applied":count}));
        }
        "export" if args.len() == 3 => {
            println!(
                "{}",
                store
                    .export(args[2].parse().map_err(|_| Error::Invalid)?)?
                    .display()
            );
        }
        _ => return Err(Error::Invalid),
    }
    println!(
        "{}",
        serde_json::to_string(&store.status()?).map_err(|_| Error::Invalid)?
    );
    Ok(())
}
