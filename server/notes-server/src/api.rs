//! Path-addressed, authenticated API. Policy and note writes stay in notes-core.
use crate::admin;
use axum::{
    body::to_bytes,
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use notes_core::agent::{AgentArgs, AgentConfig, AgentService};
use notes_model::{BaseRev, CoreError, IoKind, RelPath};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;
use uuid::Uuid;

const BODY_LIMIT: usize = 16 * 1024 * 1024;
#[derive(Clone)]
pub struct Server {
    pub data: PathBuf,
    pub trusted_proxy: Option<IpAddr>,
    slots: Arc<Semaphore>,
    rates: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
}
impl Server {
    pub fn new(data: PathBuf, trusted_proxy: Option<IpAddr>) -> Self {
        Self {
            data,
            trusted_proxy,
            slots: Arc::new(Semaphore::new(8)),
            rates: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    fn rate(&self, key: String, limit: u32) -> bool {
        let Ok(mut rates) = self.rates.lock() else {
            return false;
        };
        rates.retain(|_, (time, _)| time.elapsed() < Duration::from_secs(60));
        if rates.len() >= 4096 && !rates.contains_key(&key) {
            return false;
        }
        let entry = rates.entry(key).or_insert((Instant::now(), 0));
        entry.1 += 1;
        entry.1 <= limit
    }
}
pub fn router(server: Server) -> Router {
    Router::new().fallback(any(handle)).with_state(server)
}
#[derive(Debug)]
struct ApiError(StatusCode, &'static str);
type ApiResult<T> = std::result::Result<T, ApiError>;
fn err(status: StatusCode, code: &'static str) -> ApiError {
    ApiError(status, code)
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (self.0, Json(json!({"error":self.1}))).into_response();
        if self.0 == StatusCode::TOO_MANY_REQUESTS {
            response
                .headers_mut()
                .insert("retry-after", "60".parse().unwrap());
        }
        if self.0 == StatusCode::UNAUTHORIZED {
            response
                .headers_mut()
                .insert("www-authenticate", "Bearer".parse().unwrap());
        }
        response
    }
}
impl From<CoreError> for ApiError {
    fn from(error: CoreError) -> Self {
        use StatusCode as S;
        match error {
            CoreError::Conflict { .. } => err(S::PRECONDITION_FAILED, "revision_changed"),
            CoreError::NotFound { .. }
            | CoreError::Io {
                kind: IoKind::NotFound,
                ..
            } => err(S::NOT_FOUND, "not_found"),
            CoreError::AlreadyExists { .. } => err(S::CONFLICT, "already_exists"),
            CoreError::OutsideRoot { .. }
            | CoreError::SymlinkNotFollowed { .. }
            | CoreError::ReadOnly { .. } => err(S::FORBIDDEN, "forbidden"),
            CoreError::Unsupported { cap } if cap == "agent permission denied" => {
                err(S::FORBIDDEN, "forbidden")
            }
            CoreError::Unsupported { .. } | CoreError::InvalidPath { .. } => {
                err(S::BAD_REQUEST, "invalid_request")
            }
            CoreError::LockTimeout => err(S::SERVICE_UNAVAILABLE, "busy"),
            _ => internal(),
        }
    }
}
fn internal() -> ApiError {
    err(StatusCode::INTERNAL_SERVER_ERROR, "operation_failed")
}
fn header<'a>(headers: &'a HeaderMap, key: &str) -> Option<&'a str> {
    headers.get(key)?.to_str().ok()
}
fn base(headers: &HeaderMap) -> ApiResult<BaseRev> {
    let value = header(headers, "if-match")
        .ok_or(err(StatusCode::PRECONDITION_REQUIRED, "if_match_required"))?;
    if value.len() > 1024 {
        return Err(err(StatusCode::BAD_REQUEST, "invalid_etag"));
    }
    let raw = value
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or(err(StatusCode::BAD_REQUEST, "invalid_etag"))?;
    let bytes = URL_SAFE_NO_PAD
        .decode(raw)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "invalid_etag"))?;
    serde_json::from_slice(&bytes).map_err(|_| err(StatusCode::BAD_REQUEST, "invalid_etag"))
}
fn reply(mut value: Value, status: StatusCode) -> Response {
    // Internal identity is not a second REST addressing convention.
    let rev = value.as_object_mut().and_then(|o| {
        o.remove("note_id");
        o.remove("note_ids");
        o.remove("base_rev")
    });
    let mut response = (status, Json(value)).into_response();
    if let Some(rev) = rev {
        let tag = format!(
            "\"{}\"",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&rev).unwrap())
        );
        response.headers_mut().insert("etag", tag.parse().unwrap());
    }
    response
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    limit: Option<usize>,
    cursor: Option<usize>,
    q: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Create {
    path: RelPath,
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Text {
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Move {
    from: RelPath,
    to: RelPath,
}
fn body<T: serde::de::DeserializeOwned>(bytes: &[u8], headers: &HeaderMap) -> ApiResult<T> {
    if header(headers, "content-type").and_then(|s| s.split(';').next()) != Some("application/json")
    {
        return Err(err(StatusCode::UNSUPPORTED_MEDIA_TYPE, "json_required"));
    }
    serde_json::from_slice(bytes).map_err(|_| err(StatusCode::BAD_REQUEST, "invalid_json"))
}
async fn handle(State(server): State<Server>, request: Request) -> Response {
    let id = Uuid::new_v4().to_string();
    let secure = server.trusted_proxy.is_some();
    let result = execute(server, request, id.clone()).await;
    let mut response = result.unwrap_or_else(IntoResponse::into_response);
    let h = response.headers_mut();
    for (key, value) in [
        ("cache-control", "no-store"),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        (
            "content-security-policy",
            "default-src 'none'; frame-ancestors 'none'",
        ),
        ("x-frame-options", "DENY"),
    ] {
        h.insert(
            axum::http::HeaderName::from_static(key),
            value.parse().unwrap(),
        );
    }
    h.insert("x-request-id", id.parse().unwrap());
    if secure {
        h.insert(
            "strict-transport-security",
            "max-age=31536000".parse().unwrap(),
        );
    }
    response
}
async fn execute(server: Server, request: Request, id: String) -> ApiResult<Response> {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())
        .ok_or(err(StatusCode::FORBIDDEN, "invalid_peer"))?;
    if let Some(proxy) = server.trusted_proxy {
        if peer != proxy || header(request.headers(), "x-forwarded-proto") != Some("https") {
            return Err(err(StatusCode::FORBIDDEN, "https_required"));
        }
    } else if !peer.is_loopback() {
        return Err(err(StatusCode::FORBIDDEN, "https_required"));
    }
    if request.headers().contains_key("origin") {
        return Err(err(StatusCode::FORBIDDEN, "browser_origin_denied"));
    }
    if !server.rate(format!("ip:{peer}"), 120) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "rate_limited"));
    }
    if request.method() == "GET" && request.uri().path() == "/healthz" {
        return Ok(Json(json!({"status":"ok"})).into_response());
    }
    let permit = server
        .slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| err(StatusCode::SERVICE_UNAVAILABLE, "busy"))?;
    let (parts, body_stream) = request.into_parts();
    // The semaphore bounds bodies as well as blocking filesystem operations.
    let bytes = tokio::time::timeout(Duration::from_secs(15), to_bytes(body_stream, BODY_LIMIT))
        .await
        .map_err(|_| err(StatusCode::REQUEST_TIMEOUT, "body_timeout"))?
        .map_err(|_| err(StatusCode::PAYLOAD_TOO_LARGE, "body_too_large"))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let lock = admin::lock(&server.data).map_err(|_| internal())?;
        let _guard = lock.read().map_err(|_| internal())?;
        let store = admin::load(&server.data).map_err(|_| internal())?;
        let credential = header(&parts.headers, "authorization")
            .and_then(|s| s.strip_prefix("Bearer "))
            .filter(|s| s.len() < 200)
            .and_then(|s| admin::authenticate(&store, s));
        let Some(credential) = credential else {
            admin::audit(
                &server.data,
                "anonymous",
                &peer.to_string(),
                "authenticate",
                "denied",
                &id,
            )
            .map_err(|_| internal())?;
            return Err(err(StatusCode::UNAUTHORIZED, "unauthorized"));
        };
        if !server.rate(format!("token:{}", credential.id), 60) {
            return Err(err(StatusCode::TOO_MANY_REQUESTS, "rate_limited"));
        }
        // Only an allowlisted operation name enters the log, never a request URL.
        let operation = match parts.method.as_str() {
            "GET" => "read",
            "POST" => "create_or_move",
            "PUT" => "update",
            "PATCH" => "append",
            "DELETE" => "delete",
            _ => "unsupported",
        };
        let target_ref = blake3::hash(parts.uri.path().as_bytes()).to_hex()[..20].to_owned();
        admin::audit_target(
            &server.data,
            &credential.id.to_string(),
            &peer.to_string(),
            operation,
            "started",
            &id,
            Some(&target_ref),
        )
        .map_err(|_| internal())?;
        let result = dispatch(&server, &credential, &parts, &bytes);
        let outcome = result.as_ref().map(|_| "ok").unwrap_or_else(|e| e.1);
        admin::audit_target(
            &server.data,
            &credential.id.to_string(),
            &peer.to_string(),
            operation,
            outcome,
            &id,
            Some(&target_ref),
        )
        .map_err(|_| internal())?;
        result
    })
    .await
    .map_err(|_| internal())?
}
fn dispatch(
    server: &Server,
    credential: &admin::Credential,
    parts: &axum::http::request::Parts,
    bytes: &[u8],
) -> ApiResult<Response> {
    let method = parts.method.as_str();
    let path = percent_encoding::percent_decode_str(parts.uri.path())
        .decode_utf8()
        .map_err(|_| err(StatusCode::BAD_REQUEST, "invalid_path"))?;
    if path.len() > 4096 {
        return Err(err(StatusCode::URI_TOO_LONG, "path_too_long"));
    }
    let query: Query = serde_urlencoded::from_str(parts.uri.query().unwrap_or(""))
        .map_err(|_| err(StatusCode::BAD_REQUEST, "invalid_query"))?;
    let limit = query.limit.unwrap_or(100);
    let offset = query.cursor.unwrap_or(0);
    if !(1..=200).contains(&limit) || offset > 1_000_000 {
        return Err(err(StatusCode::BAD_REQUEST, "invalid_page"));
    }
    if method == "GET" && path == "/v1/openapi.json" {
        return Ok(reply(
            serde_json::from_str(include_str!("../openapi.json")).map_err(|_| internal())?,
            StatusCode::OK,
        ));
    }
    if method == "GET" && path == "/v1/workspaces" {
        return Ok(reply(
            json!({"workspaces":[{"name":credential.workspace,"scope":credential.scope,"permissions":credential.permissions,"review":credential.review}]}),
            StatusCode::OK,
        ));
    }
    let route = path
        .strip_prefix("/v1/workspaces/")
        .and_then(|p| p.split_once('/'))
        .ok_or(err(StatusCode::NOT_FOUND, "not_found"))?;
    if route.0 != credential.workspace {
        return Err(err(StatusCode::FORBIDDEN, "forbidden"));
    }
    if route.1 == "sync/revisions" || route.1.starts_with("sync/revisions/") {
        return sync_dispatch(server, credential, parts, bytes, route.1, offset, limit);
    }
    let config = AgentConfig {
        workspace: admin::workspace(&server.data, &credential.workspace).map_err(|_| internal())?,
        scope: credential.scope.clone(),
        permissions: credential.permissions.clone(),
        review: credential.review,
    };
    let mut service = AgentService::with_data_dir(config, &server.data.join("state"))?;
    let mut args = AgentArgs {
        limit: Some(limit),
        offset: Some(offset),
        ..Default::default()
    };
    let (tool, status) = match (method, route.1) {
        ("GET", "notes") => ("notes_list", StatusCode::OK),
        ("GET", "search") => {
            args.query = query.q;
            ("notes_search", StatusCode::OK)
        }
        ("POST", "notes") => {
            if header(&parts.headers, "if-none-match") != Some("*") {
                return Err(err(
                    StatusCode::PRECONDITION_REQUIRED,
                    "if_none_match_required",
                ));
            }
            let input: Create = body(bytes, &parts.headers)?;
            args.path = Some(input.path);
            args.text = Some(input.text);
            ("notes_create", StatusCode::CREATED)
        }
        ("POST", "moves") => {
            let input: Move = body(bytes, &parts.headers)?;
            args.path = Some(input.from);
            args.to = Some(input.to);
            args.base_rev = Some(base(&parts.headers)?);
            ("notes_move", StatusCode::OK)
        }
        (_, rest) if rest.starts_with("notes/") => {
            args.path = Some(
                RelPath::parse(&rest[6..])
                    .map_err(|_| err(StatusCode::BAD_REQUEST, "invalid_path"))?,
            );
            if method != "GET" {
                args.base_rev = Some(base(&parts.headers)?);
            }
            let tool = match method {
                "GET" => "notes_read",
                "DELETE" => "notes_delete",
                "PUT" | "PATCH" => {
                    args.text = Some(body::<Text>(bytes, &parts.headers)?.text);
                    if method == "PUT" {
                        "notes_update"
                    } else {
                        "notes_append"
                    }
                }
                _ => return Err(err(StatusCode::METHOD_NOT_ALLOWED, "method_not_allowed")),
            };
            (tool, StatusCode::OK)
        }
        _ => return Err(err(StatusCode::NOT_FOUND, "not_found")),
    };
    let requested_path = args.to.clone().or_else(|| args.path.clone());
    let mut value = service.call(tool, args)?;
    if let Some(path) = requested_path {
        value["path"] = json!(path);
    }
    if tool == "notes_list" || tool == "notes_search" {
        value["next_cursor"] = if value["truncated"] == true {
            json!(offset + limit)
        } else {
            Value::Null
        };
    }
    Ok(reply(value, status))
}

fn sync_dispatch(
    server: &Server,
    credential: &admin::Credential,
    parts: &axum::http::request::Parts,
    bytes: &[u8],
    route: &str,
    cursor: usize,
    limit: usize,
) -> ApiResult<Response> {
    use crate::sync;
    let map = |e| match e {
        sync::Error::Forbidden => err(StatusCode::FORBIDDEN, "forbidden"),
        sync::Error::Invalid => err(StatusCode::BAD_REQUEST, "invalid_sync_revision"),
        sync::Error::Missing => err(StatusCode::NOT_FOUND, "not_found"),
        sync::Error::Stale => err(StatusCode::CONFLICT, "sync_revision_changed"),
        sync::Error::Limit => err(StatusCode::INSUFFICIENT_STORAGE, "sync_capacity_reached"),
        sync::Error::Busy => err(StatusCode::SERVICE_UNAVAILABLE, "busy"),
        sync::Error::Storage => internal(),
    };
    match (parts.method.as_str(), route) {
        ("GET", "sync/revisions") => Ok(Json(
            sync::page(&server.data, credential, cursor, limit).map_err(map)?,
        )
        .into_response()),
        ("POST", "sync/revisions") => {
            let input: sync::Publication = body(bytes, &parts.headers)?;
            let revision = sync::publish(&server.data, credential, input).map_err(map)?;
            Ok(Json(json!({"revision":revision,"stored":true,"applied":false})).into_response())
        }
        ("GET", path) => {
            let id = path
                .strip_prefix("sync/revisions/")
                .and_then(|s| Uuid::parse_str(s).ok())
                .ok_or(err(StatusCode::BAD_REQUEST, "invalid_revision_id"))?;
            Ok(Json(sync::fetch(&server.data, credential, id).map_err(map)?).into_response())
        }
        _ => Err(err(StatusCode::METHOD_NOT_ALLOWED, "method_not_allowed")),
    }
}
