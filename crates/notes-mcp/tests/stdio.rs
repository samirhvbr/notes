use notes_core::{SaveResult, WorkspaceService};
use notes_model::RelPath;
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
struct Client {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    id: u64,
}
impl Client {
    fn start(config: &std::path::Path, data: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_notes-mcp"))
            .arg("--config")
            .arg(config)
            .env("NOTES_DATA_DIR", data)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut c = Self {
            child,
            input,
            output,
            id: 0,
        };
        let r=c.rpc("initialize",json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}));
        assert_eq!(r["result"]["protocolVersion"], "2025-11-25");
        writeln!(
            c.input,
            "{}",
            json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .unwrap();
        c.input.flush().unwrap();
        c
    }
    fn rpc(&mut self, method: &str, params: Value) -> Value {
        self.id += 1;
        writeln!(
            self.input,
            "{}",
            json!({"jsonrpc":"2.0","id":self.id,"method":method,"params":params})
        )
        .unwrap();
        self.input.flush().unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }
    fn call(&mut self, name: &str, args: Value) -> Value {
        self.rpc("tools/call", json!({"name":name,"arguments":args}))
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn body(response: &Value) -> Value {
    serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}
fn config(
    root: &std::path::Path,
    permissions: Value,
    scope: &str,
    review: bool,
) -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        file.path(),
        json!({"workspace":root,"permissions":permissions,"scope":scope,"review":review})
            .to_string(),
    )
    .unwrap();
    file
}
#[test]
fn headless_read_scope_and_separate_delete_permission() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("allowed")).unwrap();
    std::fs::write(root.path().join("allowed/a.md"), "needle allowed").unwrap();
    std::fs::write(root.path().join("secret.md"), "needle secret").unwrap();
    let cfg = config(root.path(), json!(["read", "search"]), "allowed", false);
    let mut c = Client::start(cfg.path(), data.path());
    let listed = c.rpc("tools/list", json!({}));
    assert!(!listed.to_string().contains("notes_delete"));
    let read = c.call("notes_read", json!({"path":"allowed/a.md"}));
    assert_eq!(body(&read)["text"], "needle allowed");
    let denied = c.call("notes_read", json!({"path":"secret.md"}));
    assert_eq!(denied["result"]["isError"], true);
    assert!(!denied.to_string().contains("needle secret"));
    let search = c.call("notes_search", json!({"query":"needle"}));
    assert!(!search.to_string().contains("secret"));
    assert_eq!(body(&search)["hits"].as_array().unwrap().len(), 1);
    assert!(c
        .call("notes_delete", json!({"path":"allowed/a.md"}))
        .get("error")
        .is_some());
}
#[test]
fn app_core_and_mcp_process_refuse_stale_writes_and_append_replays() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.md"), "initial").unwrap();
    let mut app = WorkspaceService::with_data_dir(data.path()).unwrap();
    app.open_workspace(root.path()).unwrap();
    let path = RelPath::parse("a.md").unwrap();
    let note = app.open_note(&path).unwrap();
    let cfg = config(root.path(), json!(["read", "update"]), "", false);
    let mut c = Client::start(cfg.path(), data.path());
    let first = body(&c.call("notes_read", json!({"path":"a.md"})));
    assert_eq!(first["note_id"], json!(note.note_id));
    let write = c.call(
        "notes_update",
        json!({"path":"a.md","text":"agent","base_rev":first["base_rev"]}),
    );
    assert_eq!(write["result"]["isError"], false);
    assert!(matches!(
        app.save_note(note.note_id, "app", 1, &note.base_rev)
            .unwrap(),
        SaveResult::Conflict { .. }
    ));
    let current = body(&c.call("notes_read", json!({"path":"a.md"})));
    let args = json!({"path":"a.md","text":" appended","base_rev":current["base_rev"]});
    assert_eq!(
        c.call("notes_append", args.clone())["result"]["isError"],
        false
    );
    drop(c);
    let mut c = Client::start(cfg.path(), data.path());
    let retry = c.call("notes_append", args);
    assert_eq!(body(&retry)["replayed"], true);
    assert_eq!(
        std::fs::read_to_string(root.path().join("a.md")).unwrap(),
        "agent appended"
    );
    let stale = c.call(
        "notes_update",
        json!({"path":"a.md","text":"stale","base_rev":first["base_rev"]}),
    );
    assert_eq!(body(&stale)["code"], "conflict");
}
#[test]
fn review_mode_only_writes_proposals_and_creation_never_overwrites() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("proposals")).unwrap();
    std::fs::write(root.path().join("a.md"), "original").unwrap();
    let cfg = config(
        root.path(),
        json!(["read", "create", "update", "move"]),
        "",
        true,
    );
    let mut c = Client::start(cfg.path(), data.path());
    assert_eq!(
        c.call("notes_create", json!({"path":"outside.md","text":"bad"}))["result"]["isError"],
        true
    );
    assert_eq!(
        c.call(
            "notes_create",
            json!({"path":"proposals/new.md","text":"proposal"})
        )["result"]["isError"],
        false
    );
    assert_eq!(
        c.call(
            "notes_create",
            json!({"path":"proposals/new.md","text":"replace"})
        )["result"]["isError"],
        true
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("proposals/new.md")).unwrap(),
        "proposal"
    );
    let note = body(&c.call("notes_read", json!({"path":"proposals/new.md"})));
    assert_eq!(
        c.call(
            "notes_move",
            json!({"path":"proposals/new.md","to":"published.md","base_rev":note["base_rev"]})
        )["result"]["isError"],
        true
    );
}

#[test]
fn equal_size_and_mtime_cannot_hide_a_write_from_another_process() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let path = root.path().join("a.md");
    std::fs::write(&path, "initial").unwrap();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    let mut app = WorkspaceService::with_data_dir(data.path()).unwrap();
    app.open_workspace(root.path()).unwrap();
    let note = app.open_note(&RelPath::parse("a.md").unwrap()).unwrap();
    let cfg = config(root.path(), json!(["read", "update"]), "", false);
    let mut c = Client::start(cfg.path(), data.path());
    let first = body(&c.call("notes_read", json!({"path":"a.md"})));
    let written = c.call(
        "notes_update",
        json!({"path":"a.md","text":"agent!!","base_rev":first["base_rev"]}),
    );
    assert_eq!(written["result"]["isError"], false);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    assert!(matches!(
        app.save_note(note.note_id, "local!!", 1, &note.base_rev)
            .unwrap(),
        SaveResult::Conflict { .. }
    ));
    assert_eq!(std::fs::read_to_string(path).unwrap(), "agent!!");
}

#[test]
fn simultaneous_first_clients_share_workspace_and_note_identity() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.md"), "initial").unwrap();
    let cfg = config(root.path(), json!(["read"]), "", false);
    let barrier = std::sync::Barrier::new(2);
    let ids = std::thread::scope(|scope| {
        let run = || {
            barrier.wait();
            let mut c = Client::start(cfg.path(), data.path());
            body(&c.call("notes_read", json!({"path":"a.md"})))["note_id"].clone()
        };
        let a = scope.spawn(run);
        let b = scope.spawn(run);
        (a.join().unwrap(), b.join().unwrap())
    });
    assert!(ids.0.is_string());
    assert_eq!(ids.0, ids.1);
    let mut app = WorkspaceService::with_data_dir(data.path()).unwrap();
    assert!(app.restore_last_workspace().unwrap().is_none());
    app.open_workspace(root.path()).unwrap();
    let note = app.open_note(&RelPath::parse("a.md").unwrap()).unwrap();
    assert_eq!(ids.0.as_str().unwrap(), note.note_id.to_string());
}

#[test]
fn concurrent_app_and_mcp_writers_have_one_winner() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.md"), "initial").unwrap();
    let cfg = config(root.path(), json!(["read", "update"]), "", false);
    let mut c = Client::start(cfg.path(), data.path());
    let mut app = WorkspaceService::with_data_dir(data.path()).unwrap();
    app.open_workspace(root.path()).unwrap();
    for round in 0..8 {
        let note = app.open_note(&RelPath::parse("a.md").unwrap()).unwrap();
        let base = serde_json::to_value(&note.base_rev).unwrap();
        let app_text = format!("app {round}");
        let agent_text = format!("mcp {round}");
        let barrier = std::sync::Barrier::new(2);
        let (saved, reply) = std::thread::scope(|scope| {
            let writer = scope.spawn(|| {
                barrier.wait();
                app.save_note(note.note_id, &app_text, round + 1, &note.base_rev)
                    .unwrap()
            });
            barrier.wait();
            let reply = c.call(
                "notes_update",
                json!({"path":"a.md","text":agent_text,"base_rev":base}),
            );
            (writer.join().unwrap(), reply)
        });
        let app_won = matches!(saved, SaveResult::Saved { .. });
        let agent_won = reply["result"]["isError"] == false;
        assert_ne!(app_won, agent_won, "{saved:?}; {reply}");
        assert_eq!(
            std::fs::read_to_string(root.path().join("a.md")).unwrap(),
            if app_won { app_text } else { agent_text }
        );
    }
}
