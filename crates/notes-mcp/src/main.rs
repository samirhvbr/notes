//! Bounded newline-delimited JSON-RPC over stdio. No socket or app dependency.
use notes_core::agent::{AgentArgs, AgentConfig, AgentService};
use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};
const PROTOCOL: &str = "2025-11-25";
const MAX_MESSAGE: u64 = 32 * 1024 * 1024;
fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
fn tools(config: &AgentConfig) -> Value {
    let specs = [
        (
            "notes_list",
            "List Markdown paths in the authorized subtree.",
            vec![],
        ),
        (
            "notes_search",
            "Search saved notes literally within the authorized subtree.",
            vec!["query"],
        ),
        (
            "notes_read",
            "Read a note and the base_rev required for a later write.",
            vec!["path"],
        ),
        (
            "notes_create",
            "Create a new note; refuse an existing destination.",
            vec!["path", "text"],
        ),
        (
            "notes_update",
            "Replace note text only if base_rev still matches.",
            vec!["path", "text", "base_rev"],
        ),
        (
            "notes_append",
            "Append once per note and base_rev; retrying does not duplicate text.",
            vec!["path", "text", "base_rev"],
        ),
        (
            "notes_move",
            "Move a note within scope. References are not rewritten.",
            vec!["path", "to", "base_rev"],
        ),
        (
            "notes_delete",
            "Delete a note if separately permitted and base_rev matches.",
            vec!["path", "base_rev"],
        ),
    ];
    Value::Array(specs.into_iter().filter(|(name,_,_)|AgentService::permission(name).is_some_and(|p|config.permissions.contains(&p))).map(|(name,description,required)|{
        let mut properties=serde_json::Map::new();
        for field in &required {properties.insert((*field).into(),if *field=="base_rev"{json!({"type":"object","properties":{"size":{"type":"integer","minimum":0},"mtime_ns":{"type":"number"},"hash":{"type":"string"}},"required":["size","mtime_ns","hash"],"additionalProperties":false})}else{json!({"type":"string"})});}
        if matches!(name,"notes_list"|"notes_search"){properties.insert("limit".into(),json!({"type":"integer","minimum":1,"maximum":200}));}
        json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"annotations":{"readOnlyHint":matches!(name,"notes_list"|"notes_search"|"notes_read"),"destructiveHint":matches!(name,"notes_update"|"notes_move"|"notes_delete"),"openWorldHint":false}})
    }).collect())
}
fn main() {
    if let Err(message) = run() {
        eprintln!("notes-mcp: {message}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 || args[0] != "--config" {
        return Err("usage: notes-mcp --config /absolute/path/agent.json".into());
    }
    let mut config_bytes = vec![];
    std::fs::File::open(&args[1])?
        .take(65537)
        .read_to_end(&mut config_bytes)?;
    if config_bytes.len() > 65536 {
        return Err("configuration exceeds 64 KiB".into());
    }
    let mut config: AgentConfig = serde_json::from_slice(&config_bytes)?;
    if !config.workspace.is_absolute() {
        return Err("workspace must be absolute".into());
    }
    config.workspace = std::fs::canonicalize(&config.workspace)?;
    // Validate operator configuration once, without exposing failures on stdout.
    drop(AgentService::new(config.clone())?);
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    let mut initialized = false;
    let mut ready = false;
    loop {
        let mut line = vec![];
        if (&mut input)
            .take(MAX_MESSAGE + 1)
            .read_until(b'\n', &mut line)?
            == 0
        {
            break;
        }
        if line.len() as u64 > MAX_MESSAGE {
            return Err("message exceeds 32 MiB".into());
        }
        let request: Value = match serde_json::from_slice(&line) {
            Ok(v) => v,
            Err(_) => {
                writeln!(output, "{}", error(Value::Null, -32700, "Invalid JSON"))?;
                output.flush()?;
                continue;
            }
        };
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        if id.is_none() {
            if initialized && method == "notifications/initialized" {
                ready = true;
            }
            continue;
        }
        let id = id.unwrap();
        let response = if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
            || !id.is_string() && !id.is_number()
        {
            error(id, -32600, "Invalid JSON-RPC request")
        } else if method == "initialize" && !initialized {
            initialized = true;
            let requested = request
                .pointer("/params/protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL);
            let version = if matches!(requested, "2025-03-26" | "2025-06-18" | PROTOCOL) {
                requested
            } else {
                PROTOCOL
            };
            json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":version,"capabilities":{"tools":{}},"serverInfo":{"name":"notes-mcp","version":include_str!("../../../version.md").trim()}}})
        } else if method == "ping" {
            json!({"jsonrpc":"2.0","id":id,"result":{}})
        } else if !ready {
            error(id, -32002, "Initialize the session first")
        } else if method == "tools/list" {
            json!({"jsonrpc":"2.0","id":id,"result":{"tools":tools(&config)}})
        } else if method == "tools/call" {
            let name = request
                .pointer("/params/name")
                .and_then(Value::as_str)
                .unwrap_or("");
            let arguments = request
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or(json!({}));
            let definition = tools(&config)
                .as_array()
                .unwrap()
                .iter()
                .find(|tool| tool["name"] == name)
                .cloned();
            if let Some(definition) = definition {
                let fields = definition["inputSchema"]["properties"].as_object().unwrap();
                let required = definition["inputSchema"]["required"].as_array().unwrap();
                let valid = arguments.as_object().is_some_and(|a| {
                    required.iter().all(|k| a.contains_key(k.as_str().unwrap()))
                        && a.keys().all(|k| fields.contains_key(k))
                });
                if !valid {
                    error(id, -32602, "Invalid tool arguments")
                } else {
                    match serde_json::from_value::<AgentArgs>(arguments) {
                        Err(_) => error(id, -32602, "Invalid tool argument types"),
                        Ok(args) => {
                            // Fresh snapshots per request avoid stale cross-process identity state.
                            let result = AgentService::new(config.clone())
                                .and_then(|mut service| service.call(name, args));
                            let (value, failed) = match result {
                                Ok(v) => (v, false),
                                Err(e) => (serde_json::to_value(e)?, true),
                            };
                            json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":value.to_string()}],"isError":failed}})
                        }
                    }
                }
            } else {
                error(id, -32602, "Unknown or unauthorized tool")
            }
        } else {
            error(id, -32601, "Method not found")
        };
        writeln!(output, "{response}")?;
        output.flush()?;
    }
    Ok(())
}
