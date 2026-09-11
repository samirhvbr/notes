use notes_sync_client::{
    remote::{Endpoint, Page, Remote, Transport},
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
        println!("notes-sync-client init-upload|init-receive STATE ROOT ORIGIN WORKSPACE TOKEN_FILE [--allow-private]\nnotes-sync-client init-subfolder STATE ROOT ORIGIN WORKSPACE SCOPE TOKEN_FILE [--allow-private]\nnotes-sync-client pair-preview STATE APP_DATA_DIRECTORY\nnotes-sync-client pair-confirm STATE APP_DATA_DIRECTORY TOKEN_FILE CONFIRMATION\nnotes-sync-client stage-delete STATE NOTE_UUID EXPECTED_HEAD_UUID\nnotes-sync-client apply-bundle STATE APP_DATA_DIRECTORY\nnotes-sync-client stage|stage-receiver|stage-receiver-new|stage-receiver-renames|confirm-receiver|status|received|conflicts STATE\nnotes-sync-client transfer|fetch|acknowledge|recover-server STATE TOKEN_FILE\nnotes-sync-client resolve STATE LOCAL_UUID REMOTE_UUID RESULT_FILE\nnotes-sync-client resolve-to STATE LOCAL_UUID REMOTE_UUID NOTE_PATH RESULT_FILE\nnotes-sync-client resolve-delete STATE LOCAL_UUID REMOTE_UUID NOTE_PATH\nnotes-sync-client export-attachment STATE REVISION_UUID ATTACHMENT_PATH\nnotes-sync-client export STATE REVISION_UUID\nnotes-sync-client capture-conflict STATE APP_DATA_DIRECTORY NOTE_UUID\nnotes-sync-client recapture-conflict STATE APP_DATA_DIRECTORY\nnotes-sync-client apply-resolution STATE APP_DATA_DIRECTORY RESOLUTION_UUID\nnotes-sync-client apply STATE APP_DATA_DIRECTORY\nSTATE and TOKEN_FILE must be absolute; STATE stays outside ROOT.\nTransfer only stores revisions. Explicit apply writes creations/updates while the workspace is closed and draft-free.\nUpload starts with an empty server inbox. Subfolder pairing requires a matching credential scope.\nHTTPS is required; --allow-private also permits HTTP at a literal loopback address.\nRecovery requires paused publishers and an unscoped retained queue; repeat until zero publications are replayed.\nOptional NOTES_SYNC_CA_FILE adds an operator-selected PEM trust anchor.");
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
                scope: None,
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
        "init-subfolder"
            if args.len() == 7 || (args.len() == 8 && args[7] == "--allow-private") =>
        {
            let endpoint = Endpoint {
                origin: args[3].clone(),
                name: args[4].clone(),
                allow_private: args.len() == 8,
                scope: Some(notes_model::RelPath::parse(&args[5]).map_err(|_| Error::Invalid)?),
            };
            let mut remote = Remote::connect(&endpoint, Path::new(&args[6]), ca.as_deref())?;
            store.initialize(Path::new(&args[2]), endpoint, Mode::Receive, &mut remote)?;
        }
        "pair-preview" if args.len() == 3 => {
            println!(
                "{}",
                serde_json::to_string_pretty(&store.preview_pairing(Path::new(&args[2]))?)
                    .map_err(|_| Error::Invalid)?
            );
            return Ok(());
        }
        "pair-confirm" if args.len() == 5 => {
            let mut remote =
                Remote::connect(&store.endpoint()?, Path::new(&args[3]), ca.as_deref())?;
            store.confirm_pairing(Path::new(&args[2]), &args[4], &mut remote)?;
        }
        "stage-delete" if args.len() == 4 => {
            let id = store.stage_delete(
                args[2].parse().map_err(|_| Error::Invalid)?,
                args[3].parse().map_err(|_| Error::Invalid)?,
            )?;
            println!(
                "{}",
                serde_json::json!({"staged_deletion": id, "source_written": false})
            );
        }
        "apply-bundle" if args.len() == 3 => {
            let count = store.apply_effects(Path::new(&args[2]))?;
            println!("{}", serde_json::json!({"newly_applied": count}));
        }
        "stage" if args.len() == 2 => {
            let missing = store.stage()?;
            println!(
                "{}",
                serde_json::json!({"missing_tracked_notes":missing,"deletions_queued":false})
            );
        }
        "stage-receiver" if args.len() == 2 => {
            let count = store.stage_receiver_edits()?;
            println!(
                "{}",
                serde_json::json!({"captured_edits": count, "source_written": false})
            );
        }
        "stage-receiver-new" | "stage-receiver-renames" if args.len() == 2 => {
            let count = store.stage_receiver_changes(
                false,
                command == "stage-receiver-new",
                command == "stage-receiver-renames",
            )?;
            println!(
                "{}",
                serde_json::json!({"captured_changes": count, "source_written": false})
            );
        }
        "confirm-receiver" if args.len() == 2 => {
            let count = store.confirm_receiver_edit()?;
            println!(
                "{}",
                serde_json::json!({"confirmed_edits": count, "source_written": false})
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
        "conflicts" if args.len() == 2 => {
            println!(
                "{}",
                serde_json::to_string_pretty(&store.conflicts()?).map_err(|_| Error::Invalid)?
            );
            return Ok(());
        }
        "resolve" if args.len() == 5 => {
            let id = store.resolve(
                args[2].parse().map_err(|_| Error::Invalid)?,
                args[3].parse().map_err(|_| Error::Invalid)?,
                Path::new(&args[4]),
            )?;
            println!(
                "{}",
                serde_json::json!({"staged_resolution":id, "source_written":false})
            );
        }
        "resolve-to" | "resolve-delete"
            if (command == "resolve-to" && args.len() == 6)
                || (command == "resolve-delete" && args.len() == 5) =>
        {
            let local = args[2].parse().map_err(|_| Error::Invalid)?;
            let remote = args[3].parse().map_err(|_| Error::Invalid)?;
            let path = notes_model::RelPath::parse(&args[4]).map_err(|_| Error::Invalid)?;
            let id = if command == "resolve-to" {
                store.resolve_to(local, remote, path, Path::new(&args[5]))?
            } else {
                store.resolve_delete(local, remote, path)?
            };
            println!(
                "{}",
                serde_json::json!({"staged_resolution":id, "source_written":false})
            );
        }
        "transfer" | "fetch" if args.len() == 3 => {
            let mut remote =
                Remote::connect(&store.endpoint()?, Path::new(&args[2]), ca.as_deref())?;
            if command == "fetch" {
                store.fetch(&mut remote)?;
            } else {
                store.transfer(&mut remote)?;
            }
        }
        "recover-server" if args.len() == 3 => {
            let mut remote =
                Remote::connect(&store.endpoint()?, Path::new(&args[2]), ca.as_deref())?;
            let count = store.recover_server(&mut RecoveryRemote(&mut remote))?;
            println!(
                "{}",
                serde_json::json!({"replayed_publications": count, "source_written": false})
            );
        }
        "acknowledge" if args.len() == 3 => {
            let mut remote =
                Remote::connect(&store.endpoint()?, Path::new(&args[2]), ca.as_deref())?;
            let count = store.acknowledge(&mut remote)?;
            println!("{}", serde_json::json!({"newly_acknowledged":count}));
        }
        "capture-conflict" if args.len() == 4 => {
            let id = store.capture_receiver_conflict(
                Path::new(&args[2]),
                args[3].parse().map_err(|_| Error::Invalid)?,
            )?;
            println!(
                "{}",
                serde_json::json!({"captured_branch":id, "source_written":false})
            );
        }
        "recapture-conflict" if args.len() == 3 => {
            let id = store.recapture_receiver_conflict(&PathBuf::from(&args[2]))?;
            println!(
                "{}",
                serde_json::json!({"captured_branch": id, "source_written": false})
            );
        }
        "apply-resolution" if args.len() == 4 => {
            let count = store.apply_resolution(
                Path::new(&args[2]),
                args[3].parse().map_err(|_| Error::Invalid)?,
            )?;
            println!("{}", serde_json::json!({"newly_applied":count}));
        }
        "apply" if args.len() == 3 => {
            let count = store.apply(Path::new(&args[2]))?;
            println!("{}", serde_json::json!({"newly_applied":count}));
        }
        "export-attachment" if args.len() == 4 => {
            let path = notes_model::RelPath::parse(&args[3]).map_err(|_| Error::Invalid)?;
            println!(
                "{}",
                store
                    .export_attachment(args[2].parse().map_err(|_| Error::Invalid)?, &path)?
                    .display()
            );
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

// Full-history audits can cross the server's normal credential request window.
// Retry only admission/rate-limit failures, keeping the same operation and IDs.
struct RecoveryRemote<'a>(&'a mut Remote);
fn recovery_request<T>(mut request: impl FnMut() -> Result<T>) -> Result<T> {
    for _ in 0..2 {
        match request() {
            Err(Error::Busy) => {
                eprintln!("notes-sync-client: recovery paused by server; retrying in 61 seconds");
                std::thread::sleep(std::time::Duration::from_secs(61));
            }
            result => return result,
        }
    }
    request()
}
impl Transport for RecoveryRemote<'_> {
    fn page(&mut self, cursor: usize) -> Result<Page> {
        recovery_request(|| self.0.page(cursor))
    }
    fn fetch(&mut self, id: uuid::Uuid) -> Result<notes_sync::transfer::Publication> {
        recovery_request(|| self.0.fetch(id))
    }
    fn publish(&mut self, publication: &notes_sync::transfer::Publication) -> Result<()> {
        recovery_request(|| self.0.publish(publication))
    }
    fn acknowledge(
        &mut self,
        receipt: &notes_sync::transfer::ApplicationAcknowledgment,
    ) -> Result<()> {
        recovery_request(|| self.0.acknowledge(receipt))
    }
}
