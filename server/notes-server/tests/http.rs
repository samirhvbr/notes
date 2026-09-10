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
