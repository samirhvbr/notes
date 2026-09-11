use notes_server::{admin, api, backup, sync, Result};
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

#[tokio::main]
async fn main() {
    // A panic must not print request-derived filesystem paths or content.
    std::panic::set_hook(Box::new(|_| eprintln!("notes-server: operation panicked")));
    if let Err(error) = run().await {
        // CLI errors are operator-facing, but never contain a credential value.
        eprintln!("notes-server: {error}");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = PathBuf::from(std::env::var("NOTES_SERVER_DATA").unwrap_or_else(|_| "/data".into()));
    if args.first().is_some_and(|a| a == "restore") && (args.len() == 3 || args.len() == 4) {
        return backup::restore_as(
            &PathBuf::from(&args[1]),
            &PathBuf::from(&args[2]),
            args.get(3).map(std::path::Path::new),
        );
    }
    if args.is_empty() || args[0] == "help" {
        println!("notes-server serve\nnotes-server workspace create NAME\nnotes-server token create LABEL WORKSPACE SCOPE PERMISSIONS OUTPUT [review]\nnotes-server token list\nnotes-server token revoke UUID\nnotes-server sync-prune WORKSPACE\nnotes-server backup ARCHIVE\nnotes-server restore ARCHIVE NEW_DIRECTORY [FINAL_DATA_ROOT]\nSet NOTES_SERVER_DATA to an absolute directory. Permissions: comma-separated read,create,update,move,delete,search. Use '-' for no permissions; scope '.' means the workspace root. Token secrets are written only to a new private OUTPUT file. Stop the server and back up its data before sync-prune.");
        return Ok(());
    }
    let data = admin::data_root(&path)?;
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["serve"] => {
            admin::load(&data)?;
            let bind: SocketAddr = std::env::var("NOTES_SERVER_BIND")
                .unwrap_or_else(|_| "127.0.0.1:8787".into())
                .parse()?;
            let proxy: Option<IpAddr> = std::env::var("NOTES_SERVER_TRUSTED_PROXY")
                .ok()
                .map(|s| s.parse())
                .transpose()?;
            if !bind.ip().is_loopback() {
                let private = match bind.ip() {
                    IpAddr::V4(ip) => ip.is_private(),
                    IpAddr::V6(ip) => ip.is_unique_local(),
                };
                if !private || proxy.is_none() {
                    return Err("non-loopback serving requires a private bind address and a trusted TLS proxy".into());
                }
            }
            let mut lock = backup::instance_lock(&data)?;
            let _guard = lock
                .try_write()
                .map_err(|_| "server or backup already running")?;
            let listener = tokio::net::TcpListener::bind(bind).await?;
            let app = api::router(api::Server::new(data, proxy));
            eprintln!("notes-server: ready");
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown())
            .await?;
        }
        ["workspace", "create", name] => {
            if !admin::valid_name(name) {
                return Err("invalid workspace name".into());
            }
            let mut lock = admin::lock(&data)?;
            let _guard = lock.write()?;
            std::fs::create_dir(data.join("workspaces").join(name))?;
            admin::audit(
                &data,
                "operator",
                "local",
                "workspace_create",
                "ok",
                &uuid::Uuid::new_v4().to_string(),
            )?;
        }
        ["token", "create", label, name, scope, permissions, output, rest @ ..]
            if rest.is_empty() || *rest == ["review"] =>
        {
            let permissions = if *permissions == "-" {
                Default::default()
            } else {
                permissions
                    .split(',')
                    .map(|p| serde_json::from_value(serde_json::Value::String(p.into())))
                    .collect::<std::result::Result<_, _>>()?
            };
            let scope = if *scope == "." {
                notes_model::RelPath::root()
            } else {
                notes_model::RelPath::parse(scope)?
            };
            let id = admin::create_token(
                &data,
                (*label).into(),
                (*name).into(),
                scope,
                permissions,
                !rest.is_empty(),
                &PathBuf::from(output),
            )?;
            println!("{id}");
        }
        ["token", "list"] => {
            let lock = admin::lock(&data)?;
            let _guard = lock.read()?;
            let rows: Vec<_> = admin::load(&data)?.credentials.into_iter().map(|c| serde_json::json!({"id":c.id,"label":c.label,"workspace":c.workspace,"scope":c.scope,"permissions":c.permissions,"review":c.review,"revoked":c.revoked})).collect();
            println!("{}", serde_json::to_string_pretty(&rows)?);
        }
        ["token", "revoke", id] => admin::revoke(&data, id.parse()?)?,
        ["sync-prune", workspace] => {
            let mut lock = backup::instance_lock(&data)?;
            let _guard = lock
                .try_write()
                .map_err(|_| "server or backup already running")?;
            let report = sync::prune_resolved(&data, workspace)
                .map_err(|_| "sync history could not be pruned")?;
            admin::audit(
                &data,
                "operator",
                "local",
                "sync_prune",
                "ok",
                &uuid::Uuid::new_v4().to_string(),
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        ["backup", output] => backup::backup(&data, &PathBuf::from(output))?,
        _ => return Err("invalid command; run notes-server help".into()),
    }
    Ok(())
}

async fn shutdown() {
    #[cfg(unix)]
    {
        if let Ok(mut term) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! { _ = term.recv() => {}, _ = tokio::signal::ctrl_c() => {} }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}
