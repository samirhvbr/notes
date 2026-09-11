use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{Request, StatusCode},
    Router,
};
use notes_core::agent::Permission;
use notes_model::RelPath;
use notes_server::{admin, api, backup};
use serde_json::{json, Value};
use std::{fs, net::SocketAddr};
use tower::ServiceExt;

struct Fixture {
    _dir: tempfile::TempDir,
    data: std::path::PathBuf,
    token: String,
    id: uuid::Uuid,
    app: Router,
}
impl Fixture {
    fn new(permissions: &[Permission]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let data = admin::data_root(&dir.path().join("data")).unwrap();
        fs::create_dir(data.join("workspaces/home")).unwrap();
        fs::create_dir(data.join("workspaces/home/allowed")).unwrap();
        fs::write(data.join("workspaces/home/secret.md"), "SECRET_MARKER").unwrap();
        let output = dir.path().join("token.secret");
        let id = admin::create_token(
            &data,
            "test".into(),
            "home".into(),
            RelPath::parse("allowed").unwrap(),
            permissions.iter().copied().collect(),
            false,
            &output,
        )
        .unwrap();
        let token = fs::read_to_string(output).unwrap();
        let app = api::router(api::Server::new(data.clone(), None));
        Self {
            _dir: dir,
            data,
            token,
            id,
            app,
        }
    }
    async fn request(
        &self,
        method: &str,
        path: &str,
        value: Option<Value>,
        headers: &[(&str, &str)],
    ) -> (StatusCode, axum::http::HeaderMap, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header("authorization", format!("Bearer {}", self.token))
            .header("content-type", "application/json");
        for (key, value) in headers {
            request = request.header(*key, *value);
        }
        let mut request = request
            .body(Body::from(value.map(|v| v.to_string()).unwrap_or_default()))
            .unwrap();
        request.extensions_mut().insert(ConnectInfo(
            "127.0.0.1:12345".parse::<SocketAddr>().unwrap(),
        ));
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), 32 * 1024 * 1024)
            .await
            .unwrap();
        (status, headers, serde_json::from_slice(&bytes).unwrap())
    }
}
fn all() -> Vec<Permission> {
    vec![
        Permission::Read,
        Permission::Create,
        Permission::Update,
        Permission::Move,
        Permission::Delete,
        Permission::Search,
    ]
}
const COLLECTION: &str = "/v1/workspaces/home/notes";
const NOTE: &str = "/v1/workspaces/home/notes/allowed/test.md";

#[tokio::test]
async fn conditional_lifecycle_and_append_retry_preserve_original_bytes() {
    let f = Fixture::new(&all());
    assert_eq!(
        f.request(
            "POST",
            COLLECTION,
            Some(json!({"path":"allowed/test.md","text":"hello\r\n"})),
            &[]
        )
        .await
        .0,
        StatusCode::PRECONDITION_REQUIRED
    );
    let (status, headers, _) = f
        .request(
            "POST",
            COLLECTION,
            Some(json!({"path":"allowed/test.md","text":"hello\r\n"})),
            &[("if-none-match", "*")],
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let base = headers["etag"].to_str().unwrap();
    assert_eq!(
        f.request("PUT", NOTE, Some(json!({"text":"new\n"})), &[])
            .await
            .0,
        StatusCode::PRECONDITION_REQUIRED
    );
    let update = f
        .request(
            "PUT",
            NOTE,
            Some(json!({"text":"new\n"})),
            &[("if-match", base)],
        )
        .await;
    assert_eq!(update.0, StatusCode::OK);
    assert_eq!(
        fs::read(f.data.join("workspaces/home/allowed/test.md")).unwrap(),
        b"new\r\n"
    );
    assert_eq!(
        f.request(
            "PUT",
            NOTE,
            Some(json!({"text":"stale"})),
            &[("if-match", base)]
        )
        .await
        .0,
        StatusCode::PRECONDITION_FAILED
    );
    let base = update.1["etag"].to_str().unwrap();
    for _ in 0..2 {
        assert_eq!(
            f.request(
                "PATCH",
                NOTE,
                Some(json!({"text":"end\n"})),
                &[("if-match", base)]
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let read = f.request("GET", NOTE, None, &[]).await;
    assert_eq!(read.2["text"], "new\nend\n");
    assert!(read.2.get("note_id").is_none());
    let moved = f
        .request(
            "POST",
            "/v1/workspaces/home/moves",
            Some(json!({"from":"allowed/test.md","to":"allowed/moved.md"})),
            &[("if-match", read.1["etag"].to_str().unwrap())],
        )
        .await;
    assert_eq!(moved.0, StatusCode::OK);
    let path = "/v1/workspaces/home/notes/allowed/moved.md";
    let moved_read = f.request("GET", path, None, &[]).await;
    assert_eq!(
        f.request(
            "DELETE",
            path,
            None,
            &[("if-match", moved_read.1["etag"].to_str().unwrap())]
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        f.request("GET", path, None, &[]).await.0,
        StatusCode::NOT_FOUND
    );
}
#[tokio::test]
async fn scopes_permissions_revocation_and_logs_do_not_leak() {
    let f = Fixture::new(&[Permission::Read, Permission::Search]);
    assert_eq!(
        f.request("GET", "/v1/workspaces/home/notes/secret.md", None, &[])
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let search = f
        .request(
            "GET",
            "/v1/workspaces/home/search?q=SECRET_MARKER",
            None,
            &[],
        )
        .await;
    assert_eq!(search.2["hits"], json!([]));
    assert_eq!(
        f.request("GET", "/v1/workspaces/else/notes", None, &[])
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        f.request(
            "POST",
            COLLECTION,
            Some(json!({"path":"allowed/x.md","text":"PRIVATE_BODY_MARKER"})),
            &[("if-none-match", "*")]
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    admin::revoke(&f.data, f.id).unwrap();
    assert_eq!(
        f.request("GET", COLLECTION, None, &[]).await.0,
        StatusCode::UNAUTHORIZED
    );
    let log = fs::read_to_string(f.data.join("audit/events.jsonl")).unwrap();
    for secret in [
        &f.token,
        "SECRET_MARKER",
        "PRIVATE_BODY_MARKER",
        "secret.md",
        f.data.to_str().unwrap(),
    ] {
        assert!(!log.contains(secret));
    }
    assert!(log.contains("token_revoke"));
    assert!(log.contains("forbidden"));
}
#[tokio::test]
async fn pagination_is_scoped_and_search_can_resume_within_a_note() {
    let f = Fixture::new(&[Permission::Read, Permission::Search]);
    fs::write(
        f.data.join("workspaces/home/allowed/a.md"),
        "match\nmatch\nmatch\n",
    )
    .unwrap();
    fs::write(f.data.join("workspaces/home/allowed/b.md"), "match\n").unwrap();
    let first = f
        .request("GET", &format!("{COLLECTION}?limit=1"), None, &[])
        .await;
    assert_eq!(first.2["paths"], json!(["allowed/a.md"]));
    assert_eq!(first.2["next_cursor"], 1);
    let next = f
        .request("GET", &format!("{COLLECTION}?limit=1&cursor=1"), None, &[])
        .await;
    assert_eq!(next.2["paths"], json!(["allowed/b.md"]));
    assert!(next.2["next_cursor"].is_null());
    let search = f
        .request(
            "GET",
            "/v1/workspaces/home/search?q=match&limit=2&cursor=2",
            None,
            &[],
        )
        .await;
    assert_eq!(search.2["hits"][0]["line"], 3);
    assert_eq!(search.2["hits"][1]["path"], "allowed/b.md");
    assert!(search.2["next_cursor"].is_null());
}
#[tokio::test]
async fn reject_invalid_inputs_and_browser_credentials() {
    let f = Fixture::new(&all());
    assert_eq!(
        f.request("GET", "/missing", None, &[]).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        f.request(
            "GET",
            COLLECTION,
            None,
            &[("origin", "https://example.com")]
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        f.request("GET", &format!("{COLLECTION}?extra=yes"), None, &[])
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        f.request("GET", &format!("{COLLECTION}?limit=201"), None, &[])
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        f.request(
            "POST",
            COLLECTION,
            Some(json!({"path":"allowed/x.md","text":"ok","admin":true})),
            &[("if-none-match", "*")]
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        f.request(
            "GET",
            "/v1/workspaces/home/notes/allowed/%2e%2e/secret.md",
            None,
            &[]
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let response = f.request("GET", "/healthz", None, &[]).await;
    assert_eq!(response.1["cache-control"], "no-store");
    assert_eq!(response.2, json!({"status":"ok"}));
}
#[tokio::test]
async fn rate_limit_bounds_authenticated_requests() {
    let f = Fixture::new(&[Permission::Read]);
    for _ in 0..60 {
        assert_eq!(
            f.request("GET", COLLECTION, None, &[]).await.0,
            StatusCode::OK
        );
    }
    let denied = f.request("GET", COLLECTION, None, &[]).await;
    assert_eq!(denied.0, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(denied.1["retry-after"], "60");
}
#[tokio::test]
async fn proxy_checks_actual_peer_and_https_header() {
    let mut f = Fixture::new(&[Permission::Read]);
    f.app = api::router(api::Server::new(
        f.data.clone(),
        Some("127.0.0.1".parse().unwrap()),
    ));
    assert_eq!(
        f.request("GET", COLLECTION, None, &[]).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        f.request("GET", COLLECTION, None, &[("x-forwarded-proto", "https")])
            .await
            .0,
        StatusCode::OK
    );
    f.app = api::router(api::Server::new(
        f.data.clone(),
        Some("192.168.1.2".parse().unwrap()),
    ));
    assert_eq!(
        f.request("GET", COLLECTION, None, &[("x-forwarded-proto", "https")])
            .await
            .0,
        StatusCode::FORBIDDEN
    );
}
#[test]
fn backup_restore_keeps_bytes_and_revocation_and_refuses_live_or_existing_data() {
    let f = Fixture::new(&all());
    let original = b"\xef\xbb\xbfuntouched\r\n";
    fs::write(f.data.join("workspaces/home/allowed/bom.md"), original).unwrap();
    admin::revoke(&f.data, f.id).unwrap();
    let archive = f._dir.path().join("backup.tar.gz");
    let mut lock = backup::instance_lock(&f.data).unwrap();
    let guard = lock.try_write().unwrap();
    assert!(backup::backup(&f.data, &archive).is_err());
    drop(guard);
    backup::backup(&f.data, &archive).unwrap();
    assert!(backup::backup(&f.data, &archive).is_err());
    let restored = f._dir.path().join("restored");
    backup::restore(&archive, &restored).unwrap();
    assert_eq!(
        fs::read(restored.join("workspaces/home/allowed/bom.md")).unwrap(),
        original
    );
    assert!(admin::authenticate(&admin::load(&restored).unwrap(), &f.token).is_none());
    assert!(backup::restore(&archive, &restored).is_err());
}
#[test]
fn future_schema_is_never_overwritten() {
    let f = Fixture::new(&all());
    let path = f.data.join("admin/tokens.json");
    let raw = b"{\"schema\":999,\"credentials\":[]}";
    fs::write(&path, raw).unwrap();
    assert!(admin::load(&f.data).is_err());
    assert!(admin::revoke(&f.data, f.id).is_err());
    assert_eq!(fs::read(path).unwrap(), raw);
}
#[cfg(unix)]
#[test]
fn backup_refuses_symlinks() {
    let f = Fixture::new(&all());
    std::os::unix::fs::symlink("/etc/passwd", f.data.join("workspaces/home/link.md")).unwrap();
    let archive = f._dir.path().join("backup.tar.gz");
    assert!(backup::backup(&f.data, &archive).is_err());
    assert!(!archive.exists());
}

#[tokio::test]
async fn concurrent_writers_cannot_both_consume_one_revision() {
    let f = Fixture::new(&all());
    let created = f
        .request(
            "POST",
            COLLECTION,
            Some(json!({"path":"allowed/test.md","text":"base"})),
            &[("if-none-match", "*")],
        )
        .await;
    let etag = created.1["etag"].to_str().unwrap();
    let headers = [("if-match", etag)];
    let (a, b) = tokio::join!(
        f.request("PUT", NOTE, Some(json!({"text":"first"})), &headers),
        f.request("PUT", NOTE, Some(json!({"text":"second"})), &headers)
    );
    let mut results = [a.0.as_u16(), b.0.as_u16()];
    results.sort();
    assert_eq!(results, [200, 412]);
}
#[tokio::test]
async fn review_mode_only_writes_proposals_and_large_bodies_are_rejected() {
    let f = Fixture::new(&all());
    fs::create_dir(f.data.join("workspaces/home/allowed/proposals")).unwrap();
    let mut store = admin::load(&f.data).unwrap();
    store.credentials[0].review = true;
    admin::save(&f.data, &store).unwrap();
    assert_eq!(
        f.request(
            "POST",
            COLLECTION,
            Some(json!({"path":"allowed/x.md","text":"no"})),
            &[("if-none-match", "*")]
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        f.request(
            "POST",
            COLLECTION,
            Some(json!({"path":"allowed/proposals/x.md","text":"yes"})),
            &[("if-none-match", "*")]
        )
        .await
        .0,
        StatusCode::CREATED
    );
    assert_eq!(
        f.request(
            "POST",
            COLLECTION,
            Some(json!({"path":"allowed/proposals/large.md","text":"x".repeat(16*1024*1024)})),
            &[("if-none-match", "*")]
        )
        .await
        .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert!(!f
        .data
        .join("workspaces/home/allowed/proposals/large.md")
        .exists());
}
#[test]
fn restore_rejects_link_entries_without_creating_destination() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("bad.tar.gz");
    let gzip = flate2::write::GzEncoder::new(
        fs::File::create(&archive).unwrap(),
        flate2::Compression::default(),
    );
    let mut tar = tar::Builder::new(gzip);
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_size(0);
    header.set_mode(0o777);
    header.set_link_name("/tmp").unwrap();
    header.set_cksum();
    tar.append_data(&mut header, "data/link", std::io::empty())
        .unwrap();
    tar.into_inner().unwrap().finish().unwrap();
    let destination = dir.path().join("restored");
    assert!(backup::restore(&archive, &destination).is_err());
    assert!(!destination.exists());
}

#[test]
fn restore_to_another_directory_preserves_workspace_and_note_identity() {
    let f = Fixture::new(&all());
    let path = RelPath::parse("allowed/identity.md").unwrap();
    fs::write(
        f.data.join("workspaces/home/allowed/identity.md"),
        "identity",
    )
    .unwrap();
    let mut original = notes_core::WorkspaceService::with_data_dir(f.data.join("state")).unwrap();
    let workspace = original
        .open_workspace(&f.data.join("workspaces/home"))
        .unwrap();
    let note = original.open_note(&path).unwrap();
    drop(original);
    let archive = f._dir.path().join("identity.tar.gz");
    backup::backup(&f.data, &archive).unwrap();
    let restored = f._dir.path().join("restored-identity");
    backup::restore(&archive, &restored).unwrap();
    let mut service = notes_core::WorkspaceService::with_data_dir(restored.join("state")).unwrap();
    assert_eq!(
        service
            .open_workspace(&restored.join("workspaces/home"))
            .unwrap()
            .id,
        workspace.id
    );
    assert_eq!(service.open_note(&path).unwrap().note_id, note.note_id);
}

#[test]
fn backup_omits_only_operational_locks_and_keeps_user_files_with_similar_names() {
    let f = Fixture::new(&all());
    fs::write(f.data.join("workspaces/home/server.lock"), "user-owned").unwrap();
    let archive = f._dir.path().join("locks.tar.gz");
    backup::backup(&f.data, &archive).unwrap();
    let gzip = flate2::read::GzDecoder::new(fs::File::open(archive).unwrap());
    let mut tar = tar::Archive::new(gzip);
    let paths: Vec<_> = tar
        .entries()
        .unwrap()
        .map(|e| e.unwrap().path().unwrap().into_owned())
        .collect();
    assert!(!paths.contains(&std::path::PathBuf::from("data/server.lock")));
    assert!(!paths.contains(&std::path::PathBuf::from("data/admin/lock")));
    assert!(!paths.contains(&std::path::PathBuf::from("data/audit/lock")));
    assert!(paths.contains(&std::path::PathBuf::from(
        "data/workspaces/home/server.lock"
    )));
    assert!(paths.contains(&std::path::PathBuf::from("data/admin/tokens.json")));
}

const SYNC: &str = "/v1/workspaces/home/sync/revisions";
fn publication(
    workspace: uuid::Uuid,
    prior: Option<&notes_server::sync::Publication>,
    path: &str,
    bytes: Option<&[u8]>,
) -> notes_server::sync::Publication {
    use base64::Engine;
    let expected = prior.map(|p| p.revision.id);
    notes_server::sync::Publication {
        attachments: vec![],
        branches: vec![],
        history: vec![],
        workspace,
        expected,
        revision: notes_sync::Revision::new(
            prior.map(|p| p.revision.note).unwrap_or_default(),
            expected.into_iter().collect(),
            uuid::Uuid::new_v4(),
            RelPath::parse(path).unwrap(),
            bytes.map(|b| notes_model::ContentHash::from_bytes(*blake3::hash(b).as_bytes())),
        ),
        content_base64: bytes.map(|b| base64::engine::general_purpose::STANDARD.encode(b)),
    }
}
async fn sync_workspace(f: &Fixture) -> uuid::Uuid {
    let (status, _, page) = f.request("GET", SYNC, None, &[]).await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_value(page["workspace"].clone()).unwrap()
}
async fn post_revision(f: &Fixture, p: &notes_server::sync::Publication) -> StatusCode {
    f.request("POST", SYNC, Some(serde_json::to_value(p).unwrap()), &[])
        .await
        .0
}

#[tokio::test]
async fn sync_exact_bytes_retry_stale_head_tombstone_and_restart() {
    let mut f = Fixture::new(&all());
    let workspace = sync_workspace(&f).await;
    let first = publication(
        workspace,
        None,
        "allowed/byte.md",
        Some(b"\xef\xbb\xbfhello\r\n\xff"),
    );
    assert_eq!(post_revision(&f, &first).await, StatusCode::OK);
    let state = f.data.join("sync/home/vault.json");
    let original = fs::read(&state).unwrap();
    assert_eq!(post_revision(&f, &first).await, StatusCode::OK);
    assert_eq!(fs::read(&state).unwrap(), original);
    let second = publication(
        workspace,
        Some(&first),
        "allowed/renamed.md",
        Some(b"new\r\n"),
    );
    assert_eq!(post_revision(&f, &second).await, StatusCode::OK);
    let accepted = fs::read(&state).unwrap();
    let stale = publication(workspace, Some(&first), "allowed/byte.md", Some(b"stale"));
    assert_eq!(post_revision(&f, &stale).await, StatusCode::CONFLICT);
    assert_eq!(fs::read(&state).unwrap(), accepted);
    // Retrying an older accepted publication must not roll the current head back.
    assert_eq!(post_revision(&f, &first).await, StatusCode::OK);
    assert_eq!(fs::read(&state).unwrap(), accepted);
    let deletion = publication(workspace, Some(&second), "allowed/renamed.md", None);
    assert_eq!(post_revision(&f, &deletion).await, StatusCode::OK);
    f.app = api::router(api::Server::new(f.data.clone(), None));
    let (status, _, fetched) = f
        .request("GET", &format!("{SYNC}/{}", first.revision.id), None, &[])
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched, serde_json::to_value(&first).unwrap());
    let (_, _, page) = f
        .request("GET", &format!("{SYNC}?limit=1&cursor=1"), None, &[])
        .await;
    assert_eq!(page["revisions"][0]["id"], second.revision.id.to_string());
    assert_eq!(page["next_cursor"], 2);
    assert_eq!(page["has_more"], true);
    assert_eq!(
        page["heads"][first.revision.note.to_string()],
        deletion.revision.id.to_string()
    );
    assert!(fs::read_dir(f.data.join("workspaces/home/allowed"))
        .unwrap()
        .next()
        .is_none());
    let logs = fs::read_to_string(f.data.join("audit/events.jsonl")).unwrap();
    assert!(!logs.contains("byte.md"));
    assert!(!logs.contains(first.content_base64.as_ref().unwrap()));
}

#[tokio::test]
async fn sync_permissions_scope_and_historical_paths_are_enforced() {
    let mut f = Fixture::new(&all());
    let workspace = sync_workspace(&f).await;
    let secret = publication(workspace, None, "secret.md", Some(b"PRIVATE"));
    assert_eq!(post_revision(&f, &secret).await, StatusCode::FORBIDDEN);
    let first = publication(workspace, None, "allowed/test.md", Some(b"first"));
    assert_eq!(post_revision(&f, &first).await, StatusCode::OK);
    let moved = publication(workspace, Some(&first), "secret.md", Some(b"first"));
    assert_eq!(post_revision(&f, &moved).await, StatusCode::FORBIDDEN);
    // An operator-scoped publisher can move it; the old subfolder token cannot
    // retrieve any historical revision afterward, even by guessing its UUID.
    let scoped = f.token.clone();
    let output = f._dir.path().join("root.secret");
    admin::create_token(
        &f.data,
        "root".into(),
        "home".into(),
        RelPath::root(),
        all().into_iter().collect(),
        false,
        &output,
    )
    .unwrap();
    f.token = fs::read_to_string(output).unwrap();
    assert_eq!(post_revision(&f, &moved).await, StatusCode::OK);
    f.token = scoped;
    assert_eq!(
        f.request("GET", &format!("{SYNC}/{}", first.revision.id), None, &[])
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let (_, _, page) = f.request("GET", SYNC, None, &[]).await;
    assert_eq!(page["revisions"], json!([]));
    assert_eq!(page["heads"], json!({}));
    assert_eq!(
        f.request("GET", "/v1/workspaces/other/sync/revisions", None, &[])
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let read = Fixture::new(&[Permission::Read]);
    let workspace = sync_workspace(&read).await;
    assert_eq!(
        post_revision(
            &read,
            &publication(workspace, None, "allowed/test.md", Some(b"no"))
        )
        .await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn sync_rejects_hash_forgery_collisions_and_future_state_without_replacement() {
    let f = Fixture::new(&all());
    let workspace = sync_workspace(&f).await;
    let first = publication(workspace, None, "allowed/test.md", Some(b"first"));
    let mut forged = first.clone();
    forged.content_base64 = Some("YmFk".into());
    assert_eq!(post_revision(&f, &forged).await, StatusCode::BAD_REQUEST);
    assert_eq!(post_revision(&f, &first).await, StatusCode::OK);
    let conflict = publication(workspace, None, "allowed/test.md", Some(b"collision"));
    assert_eq!(post_revision(&f, &conflict).await, StatusCode::CONFLICT);
    let mut reuse = first.clone();
    reuse.revision.device = uuid::Uuid::new_v4();
    assert_eq!(post_revision(&f, &reuse).await, StatusCode::CONFLICT);
    let path = f.data.join("sync/home/vault.json");
    let mut state: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    state["publications"][0]["content_base64"] = json!("YmFk");
    let corrupt = serde_json::to_vec(&state).unwrap();
    fs::write(&path, &corrupt).unwrap();
    assert_eq!(
        f.request("GET", SYNC, None, &[]).await.0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(fs::read(&path).unwrap(), corrupt);
    state["schema"] = json!(999);
    let future = serde_json::to_vec(&state).unwrap();
    fs::write(&path, &future).unwrap();
    assert_eq!(
        f.request("GET", SYNC, None, &[]).await.0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(fs::read(&path).unwrap(), future);
    fs::write(&path, b"incomplete").unwrap();
    assert_eq!(
        post_revision(&f, &first).await,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(fs::read(&path).unwrap(), b"incomplete");
}

#[tokio::test]
async fn sync_vault_survives_offline_backup_restore() {
    let f = Fixture::new(&all());
    let workspace = sync_workspace(&f).await;
    let p = publication(workspace, None, "allowed/test.md", Some(b"\xff\r\n"));
    assert_eq!(post_revision(&f, &p).await, StatusCode::OK);
    // Simulate an abandoned temporary write: recovery must use the committed file.
    fs::write(f.data.join("sync/home/.abandoned.tmp"), b"partial").unwrap();
    let output = f._dir.path().join("backup.tar.gz");
    backup::backup(&f.data, &output).unwrap();
    let restored = f._dir.path().join("restored");
    backup::restore(&output, &restored).unwrap();
    let c = admin::authenticate(&admin::load(&restored).unwrap(), &f.token).unwrap();
    assert_eq!(
        notes_server::sync::fetch(&restored, &c, p.revision.id).unwrap(),
        p
    );
    assert!(
        !restored
            .join("sync/home/vault.lock")
            .metadata()
            .unwrap()
            .len()
            > 0
    );
}

#[tokio::test]
async fn sync_concurrent_publications_consume_one_head_and_capacity_refuses_without_loss() {
    let f = Fixture::new(&all());
    let workspace = sync_workspace(&f).await;
    let first = publication(workspace, None, "allowed/test.md", Some(b"base"));
    assert_eq!(post_revision(&f, &first).await, StatusCode::OK);
    let a = publication(workspace, Some(&first), "allowed/test.md", Some(b"a"));
    let b = publication(workspace, Some(&first), "allowed/test.md", Some(b"b"));
    let (ra, rb) = tokio::join!(post_revision(&f, &a), post_revision(&f, &b));
    assert_eq!(
        [ra, rb]
            .into_iter()
            .filter(|s| *s == StatusCode::OK)
            .count(),
        1
    );
    assert!([ra, rb]
        .into_iter()
        .any(|s| s == StatusCode::CONFLICT || s == StatusCode::SERVICE_UNAVAILABLE));
    let loser = if ra == StatusCode::OK { &b } else { &a };
    assert_eq!(post_revision(&f, loser).await, StatusCode::CONFLICT);
    // Exercise the actual byte quota; rejected content must not evict history.
    let bytes = vec![b'x'; notes_server::sync::MAX_CONTENT];
    for n in 0..3 {
        let large = publication(
            workspace,
            None,
            &format!("allowed/large{n}.md"),
            Some(&bytes),
        );
        assert_eq!(post_revision(&f, &large).await, StatusCode::OK);
    }
    let before = fs::read(f.data.join("sync/home/vault.json")).unwrap();
    let exceeds = publication(workspace, None, "allowed/excess.md", Some(&bytes));
    assert_eq!(
        post_revision(&f, &exceeds).await,
        StatusCode::INSUFFICIENT_STORAGE
    );
    assert_eq!(
        fs::read(f.data.join("sync/home/vault.json")).unwrap(),
        before
    );
}

#[tokio::test]
async fn sync_review_and_distinct_mutation_permissions_are_preserved() {
    let mut f = Fixture::new(&all());
    let workspace = sync_workspace(&f).await;
    let first = publication(workspace, None, "allowed/test.md", Some(b"base"));
    assert_eq!(post_revision(&f, &first).await, StatusCode::OK);
    let restricted = f._dir.path().join("restricted.secret");
    admin::create_token(
        &f.data,
        "restricted".into(),
        "home".into(),
        RelPath::parse("allowed").unwrap(),
        [Permission::Read, Permission::Update].into_iter().collect(),
        false,
        &restricted,
    )
    .unwrap();
    f.token = fs::read_to_string(restricted).unwrap();
    assert_eq!(
        post_revision(
            &f,
            &publication(workspace, Some(&first), "allowed/moved.md", Some(b"base"))
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post_revision(
            &f,
            &publication(workspace, Some(&first), "allowed/test.md", None)
        )
        .await,
        StatusCode::FORBIDDEN
    );
    let review = f._dir.path().join("review.secret");
    admin::create_token(
        &f.data,
        "review".into(),
        "home".into(),
        RelPath::parse("allowed").unwrap(),
        all().into_iter().collect(),
        true,
        &review,
    )
    .unwrap();
    f.token = fs::read_to_string(review).unwrap();
    assert_eq!(
        post_revision(
            &f,
            &publication(
                workspace,
                Some(&first),
                "allowed/proposals/stolen.md",
                Some(b"base")
            )
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post_revision(
            &f,
            &publication(
                workspace,
                None,
                "allowed/proposals/new.md",
                Some(b"proposal")
            )
        )
        .await,
        StatusCode::OK
    );
    admin::revoke(
        &f.data,
        admin::authenticate(&admin::load(&f.data).unwrap(), &f.token)
            .unwrap()
            .id,
    )
    .unwrap();
    assert_eq!(
        f.request("GET", SYNC, None, &[]).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn sync_application_acknowledgments_are_owned_monotonic_and_durable() {
    let mut f = Fixture::new(&all());
    let workspace = sync_workspace(&f).await;
    let first = publication(workspace, None, "allowed/test.md", Some(b"one"));
    assert_eq!(post_revision(&f, &first).await, StatusCode::OK);
    let second = publication(workspace, Some(&first), "allowed/test.md", Some(b"two"));
    assert_eq!(post_revision(&f, &second).await, StatusCode::OK);
    let route = "/v1/workspaces/home/sync/acknowledgments";
    let device = uuid::Uuid::new_v4();
    let mut receipt = json!({"workspace":workspace,"device":device,"revision":first.revision.id});
    assert_eq!(
        f.request("POST", route, Some(receipt.clone()), &[]).await.0,
        StatusCode::OK
    );
    let path = f.data.join("sync/home/vault.json");
    let before = fs::read(&path).unwrap();
    assert_eq!(
        f.request("POST", route, Some(receipt.clone()), &[]).await.2,
        receipt
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    receipt["revision"] = json!(second.revision.id);
    assert_eq!(
        f.request("POST", route, Some(receipt.clone()), &[]).await.0,
        StatusCode::OK
    );
    f.app = api::router(api::Server::new(f.data.clone(), None));
    assert_eq!(sync_workspace(&f).await, workspace);
    let accepted = fs::read(&path).unwrap();
    receipt["revision"] = json!(first.revision.id);
    assert_eq!(
        f.request("POST", route, Some(receipt.clone()), &[]).await.0,
        StatusCode::CONFLICT
    );
    receipt["revision"] = json!(uuid::Uuid::new_v4());
    assert_eq!(
        f.request("POST", route, Some(receipt.clone()), &[]).await.0,
        StatusCode::NOT_FOUND
    );
    receipt["revision"] = json!(second.revision.id);
    receipt["workspace"] = json!(uuid::Uuid::new_v4());
    assert_eq!(
        f.request("POST", route, Some(receipt.clone()), &[]).await.0,
        StatusCode::CONFLICT
    );
    receipt["workspace"] = json!(workspace);
    let other = f._dir.path().join("other.secret");
    admin::create_token(
        &f.data,
        "other".into(),
        "home".into(),
        RelPath::root(),
        [Permission::Read].into(),
        false,
        &other,
    )
    .unwrap();
    f.token = fs::read_to_string(other).unwrap();
    assert_eq!(
        f.request("POST", route, Some(receipt.clone()), &[]).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(fs::read(&path).unwrap(), accepted);
    // A new device can report application with Read alone; this writes no note.
    receipt["device"] = json!(uuid::Uuid::new_v4());
    assert_eq!(
        f.request("POST", route, Some(receipt), &[]).await.0,
        StatusCode::OK
    );
    let backup_path = f._dir.path().join("receipt-backup.tar");
    backup::backup(&f.data, &backup_path).unwrap();
    let restored = f._dir.path().join("restored-receipts");
    backup::restore(&backup_path, &restored).unwrap();
    assert_eq!(
        fs::read(restored.join("sync/home/vault.json")).unwrap(),
        fs::read(&path).unwrap()
    );
    let c = admin::authenticate(&admin::load(&restored).unwrap(), f.token.trim()).unwrap();
    notes_server::sync::page(&restored, &c, 0, 20).unwrap();
    assert!(fs::read_dir(f.data.join("workspaces/home/allowed"))
        .unwrap()
        .next()
        .is_none());
}

#[tokio::test]
async fn sync_acknowledgments_refuse_invisible_history_and_revoked_credentials() {
    let f = Fixture::new(&all());
    let workspace = sync_workspace(&f).await;
    let credential = admin::authenticate(&admin::load(&f.data).unwrap(), f.token.trim()).unwrap();
    let mut broad = credential.clone();
    broad.scope = RelPath::root();
    let p = publication(workspace, None, "secret.md", Some(b"private"));
    notes_server::sync::publish(&f.data, &broad, p.clone()).unwrap();
    let receipt =
        json!({"workspace":workspace,"device":uuid::Uuid::new_v4(),"revision":p.revision.id});
    let route = "/v1/workspaces/home/sync/acknowledgments";
    assert_eq!(
        f.request("POST", route, Some(receipt.clone()), &[]).await.0,
        StatusCode::NOT_FOUND
    );
    admin::revoke(&f.data, f.id).unwrap();
    assert_eq!(
        f.request("POST", route, Some(receipt), &[]).await.0,
        StatusCode::UNAUTHORIZED
    );
}

fn resolution(
    remote: &notes_server::sync::Publication,
    local: &notes_server::sync::Publication,
) -> notes_server::sync::Publication {
    let mut p = publication(
        remote.workspace,
        Some(remote),
        "allowed/test.md",
        Some(b"chosen"),
    );
    p.revision.parents.insert(local.revision.id);
    p.branches.push(notes_sync::transfer::Branch {
        attachments: vec![],
        revision: local.revision.clone(),
        content_base64: local.content_base64.clone(),
    });
    p
}

#[tokio::test]
async fn sync_resolution_keeps_both_histories_atomically_and_retries_after_restart() {
    let mut f = Fixture::new(&all());
    let workspace = sync_workspace(&f).await;
    let base = publication(workspace, None, "allowed/test.md", Some(b"base"));
    assert_eq!(post_revision(&f, &base).await, StatusCode::OK);
    let remote = publication(workspace, Some(&base), "allowed/test.md", Some(b"remote"));
    let local = publication(workspace, Some(&base), "allowed/test.md", Some(b"local"));
    assert_eq!(post_revision(&f, &remote).await, StatusCode::OK);
    assert_eq!(post_revision(&f, &local).await, StatusCode::CONFLICT);
    let merge = resolution(&remote, &local);
    let vault = f.data.join("sync/home/vault.json");
    let before = fs::read(&vault).unwrap();
    let mut stale = merge.clone();
    stale.expected = Some(base.revision.id);
    assert_eq!(post_revision(&f, &stale).await, StatusCode::CONFLICT);
    assert_eq!(fs::read(&vault).unwrap(), before);
    assert_eq!(post_revision(&f, &merge).await, StatusCode::OK);
    let after = fs::read(&vault).unwrap();
    f.app = api::router(api::Server::new(f.data.clone(), None));
    assert_eq!(post_revision(&f, &merge).await, StatusCode::OK);
    assert_eq!(fs::read(&vault).unwrap(), after);
    let (_, _, fetched) = f
        .request("GET", &format!("{SYNC}/{}", merge.revision.id), None, &[])
        .await;
    assert_eq!(fetched, serde_json::to_value(&merge).unwrap());
    let (_, _, page) = f.request("GET", SYNC, None, &[]).await;
    assert_eq!(page["next_cursor"], 3);
    assert_eq!(
        page["heads"][base.revision.note.to_string()],
        merge.revision.id.to_string()
    );
    assert!(fs::read_dir(f.data.join("workspaces/home/allowed"))
        .unwrap()
        .next()
        .is_none());
}

#[test]
fn acknowledged_resolution_pruning_keeps_graph_tombstones_and_cursors() {
    let f = Fixture::new(&all());
    let credential = admin::authenticate(&admin::load(&f.data).unwrap(), f.token.trim()).unwrap();
    let mut broad = credential.clone();
    broad.scope = RelPath::root();
    let workspace = notes_server::sync::page(&f.data, &credential, 0, 20)
        .unwrap()
        .workspace;
    let base = publication(workspace, None, "allowed/test.md", Some(b"base"));
    let remote = publication(workspace, Some(&base), "allowed/test.md", Some(b"remote"));
    let local = publication(
        workspace,
        Some(&base),
        "allowed/test.md",
        Some(b"![asset](../secret.bin)"),
    );
    let mut merge = resolution(&remote, &local);
    merge.branches[0].attachments = vec![notes_sync::transfer::Attachment::new(
        RelPath::parse("secret.bin").unwrap(),
        b"private branch asset",
    )];
    notes_server::sync::publish(&f.data, &credential, base).unwrap();
    notes_server::sync::publish(&f.data, &credential, remote).unwrap();
    notes_server::sync::publish(&f.data, &broad, merge.clone()).unwrap();
    let tombstone = publication(workspace, Some(&merge), "allowed/test.md", None);
    notes_server::sync::publish(&f.data, &broad, tombstone.clone()).unwrap();
    assert_eq!(
        notes_server::sync::prune_resolved(&f.data, "home")
            .unwrap()
            .pruned_resolutions,
        0
    );
    let first_device = uuid::Uuid::new_v4();
    let second_device = uuid::Uuid::new_v4();
    for (device, revision) in [
        (first_device, merge.revision.id),
        (second_device, merge.expected.unwrap()),
    ] {
        notes_server::sync::acknowledge(
            &f.data,
            &broad,
            &notes_sync::transfer::ApplicationAcknowledgment {
                workspace,
                device,
                revision,
            },
        )
        .unwrap();
    }
    notes_server::sync::acknowledge(
        &f.data,
        &broad,
        &notes_sync::transfer::ApplicationAcknowledgment {
            workspace,
            device: first_device,
            revision: tombstone.revision.id,
        },
    )
    .unwrap();
    assert_eq!(
        notes_server::sync::prune_resolved(&f.data, "home")
            .unwrap()
            .pruned_resolutions,
        0
    );
    notes_server::sync::acknowledge(
        &f.data,
        &broad,
        &notes_sync::transfer::ApplicationAcknowledgment {
            workspace,
            device: second_device,
            revision: tombstone.revision.id,
        },
    )
    .unwrap();
    let before = fs::metadata(f.data.join("sync/home/vault.json"))
        .unwrap()
        .len();
    let committed = fs::read(f.data.join("sync/home/vault.json")).unwrap();
    let mut instance = backup::instance_lock(&f.data).unwrap();
    let guard = instance.write().unwrap();
    let blocked = std::process::Command::new(env!("CARGO_BIN_EXE_notes-server"))
        .env("NOTES_SERVER_DATA", &f.data)
        .args(["sync-prune", "home"])
        .output()
        .unwrap();
    assert!(!blocked.status.success());
    drop(guard);
    assert_eq!(
        fs::read(f.data.join("sync/home/vault.json")).unwrap(),
        committed
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_notes-server"))
        .env("NOTES_SERVER_DATA", &f.data)
        .args(["sync-prune", "home"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["pruned_resolutions"], 1);
    assert!(report["pruned_payload_bytes"].as_u64().unwrap() > 0);
    assert_eq!(report["known_devices"], 2);
    let compacted = notes_server::sync::fetch(&f.data, &credential, merge.revision.id).unwrap();
    assert!(compacted.branches.is_empty());
    assert_eq!(compacted.history, vec![local.revision.clone()]);
    assert!(
        fs::metadata(f.data.join("sync/home/vault.json"))
            .unwrap()
            .len()
            < before
    );
    assert_eq!(
        notes_server::sync::fetch(&f.data, &credential, tombstone.revision.id).unwrap(),
        tombstone
    );
    let archive = f._dir.path().join("pruned-backup.tar.gz");
    backup::backup(&f.data, &archive).unwrap();
    let restored = f._dir.path().join("pruned-restored");
    backup::restore(&archive, &restored).unwrap();
    let restored_credential =
        admin::authenticate(&admin::load(&restored).unwrap(), f.token.trim()).unwrap();
    assert_eq!(
        notes_server::sync::fetch(&restored, &restored_credential, merge.revision.id).unwrap(),
        compacted
    );
    let page = notes_server::sync::page(&f.data, &credential, 0, 20).unwrap();
    assert_eq!(page.next_cursor, 4);
    assert_eq!(page.heads[&merge.revision.note], tombstone.revision.id);
    let pruned_state = fs::read(f.data.join("sync/home/vault.json")).unwrap();
    assert_eq!(
        notes_server::sync::publish(&f.data, &broad, merge.clone()).unwrap(),
        merge.revision.id
    );
    assert_eq!(
        fs::read(f.data.join("sync/home/vault.json")).unwrap(),
        pruned_state
    );
    assert!(matches!(
        notes_server::sync::publish(&f.data, &credential, merge.clone()),
        Err(notes_server::sync::Error::Forbidden)
    ));
    assert_eq!(
        fs::read(f.data.join("sync/home/vault.json")).unwrap(),
        pruned_state
    );
    assert_eq!(
        notes_server::sync::prune_resolved(&f.data, "home")
            .unwrap()
            .pruned_resolutions,
        0
    );
    assert!(fs::read_to_string(f.data.join("audit/events.jsonl"))
        .unwrap()
        .contains("sync_prune"));
    let mut injected = compacted;
    injected.revision.id = uuid::Uuid::new_v4();
    assert!(matches!(
        notes_server::sync::publish(&f.data, &credential, injected),
        Err(notes_server::sync::Error::Invalid)
    ));
}

#[tokio::test]
async fn sync_resolution_rejects_hidden_branches_forgery_and_unrelated_history() {
    let f = Fixture::new(&all());
    let workspace = sync_workspace(&f).await;
    let base = publication(workspace, None, "allowed/test.md", Some(b"base"));
    assert_eq!(post_revision(&f, &base).await, StatusCode::OK);
    let remote = publication(workspace, Some(&base), "allowed/test.md", Some(b"remote"));
    let local = publication(workspace, Some(&base), "allowed/test.md", Some(b"local"));
    assert_eq!(post_revision(&f, &remote).await, StatusCode::OK);
    let good = resolution(&remote, &local);
    let vault = f.data.join("sync/home/vault.json");
    let before = fs::read(&vault).unwrap();
    let mut hidden = good.clone();
    hidden.branches[0].revision.path = RelPath::parse("secret.md").unwrap();
    assert_eq!(post_revision(&f, &hidden).await, StatusCode::FORBIDDEN);
    let mut forged = good.clone();
    forged.branches[0].content_base64 = Some("YmFk".into());
    assert_eq!(post_revision(&f, &forged).await, StatusCode::BAD_REQUEST);
    let mut foreign = good.clone();
    foreign.branches[0].revision.note = notes_model::NoteId::default();
    assert_eq!(post_revision(&f, &foreign).await, StatusCode::BAD_REQUEST);
    let mut unrelated = good.clone();
    let extra = publication(
        workspace,
        Some(&base),
        "allowed/test.md",
        Some(b"unconsumed"),
    );
    unrelated.branches.push(notes_sync::transfer::Branch {
        attachments: vec![],
        revision: extra.revision,
        content_base64: extra.content_base64,
    });
    assert_eq!(post_revision(&f, &unrelated).await, StatusCode::BAD_REQUEST);
    let mut oversized = good.clone();
    oversized.branches = vec![good.branches[0].clone(); 21];
    assert_ne!(post_revision(&f, &oversized).await, StatusCode::OK);
    assert_eq!(fs::read(&vault).unwrap(), before);
    assert_eq!(post_revision(&f, &good).await, StatusCode::OK);
}

#[tokio::test]
async fn sync_resolution_preserves_create_move_and_delete_permissions() {
    for missing in [Permission::Create, Permission::Move, Permission::Delete] {
        let mut f = Fixture::new(&all());
        let workspace = sync_workspace(&f).await;
        let base = publication(workspace, None, "allowed/test.md", Some(b"base"));
        assert_eq!(post_revision(&f, &base).await, StatusCode::OK);
        let remote = publication(
            workspace,
            Some(&base),
            "allowed/remote.md",
            if missing == Permission::Create {
                None
            } else {
                Some(b"remote")
            },
        );
        let local = publication(workspace, Some(&base), "allowed/test.md", Some(b"local"));
        assert_eq!(post_revision(&f, &remote).await, StatusCode::OK);
        let mut merge = resolution(&remote, &local);
        if missing == Permission::Delete {
            merge.revision.content = None;
            merge.content_base64 = None;
        }
        let vault = f.data.join("sync/home/vault.json");
        let before = fs::read(&vault).unwrap();
        let original = f.token.clone();
        let secret = f._dir.path().join("restricted.secret");
        admin::create_token(
            &f.data,
            "restricted".into(),
            "home".into(),
            RelPath::parse("allowed").unwrap(),
            all().into_iter().filter(|p| *p != missing).collect(),
            false,
            &secret,
        )
        .unwrap();
        f.token = fs::read_to_string(secret).unwrap();
        assert_eq!(post_revision(&f, &merge).await, StatusCode::FORBIDDEN);
        assert_eq!(fs::read(&vault).unwrap(), before);
        f.token = original;
        assert_eq!(post_revision(&f, &merge).await, StatusCode::OK);
    }
}

#[tokio::test]
async fn sync_attachments_validate_references_hashes_and_historical_scope() {
    use notes_sync::transfer::Attachment;
    let mut f = Fixture::new(&all());
    let workspace = sync_workspace(&f).await;
    let mut p = publication(workspace, None, "allowed/test.md", Some(b"![a](asset.bin)"));
    let asset = Attachment::new(RelPath::parse("allowed/asset.bin").unwrap(), &[0, 255]);
    p.attachments = vec![asset.clone(), asset.clone()];
    assert!(!post_revision(&f, &p).await.is_success());
    p.attachments = vec![asset.clone()];
    p.attachments[0].content_base64 = "AAAA".into();
    assert!(!post_revision(&f, &p).await.is_success());
    p.attachments = vec![Attachment::new(
        RelPath::parse("allowed/unreferenced.bin").unwrap(),
        b"x",
    )];
    assert!(!post_revision(&f, &p).await.is_success());
    p.attachments = vec![asset];
    assert_eq!(post_revision(&f, &p).await, StatusCode::OK);
    let mut outside = publication(
        workspace,
        Some(&p),
        "allowed/test.md",
        Some(b"![a](../secret.bin)"),
    );
    outside.attachments = vec![Attachment::new(
        RelPath::parse("secret.bin").unwrap(),
        b"private",
    )];
    assert_eq!(post_revision(&f, &outside).await, StatusCode::FORBIDDEN);
    let scoped = f.token.clone();
    let output = f._dir.path().join("attachment-root.secret");
    admin::create_token(
        &f.data,
        "root".into(),
        "home".into(),
        RelPath::root(),
        all().into_iter().collect(),
        false,
        &output,
    )
    .unwrap();
    f.token = fs::read_to_string(output).unwrap();
    assert_eq!(post_revision(&f, &outside).await, StatusCode::OK);
    f.token = scoped;
    let (_, _, page) = f.request("GET", SYNC, None, &[]).await;
    assert_eq!(page["revisions"], json!([]));
    assert_eq!(
        f.request("GET", &format!("{SYNC}/{}", p.revision.id), None, &[])
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}
