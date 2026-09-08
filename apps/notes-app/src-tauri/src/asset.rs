//! The `notes-asset://` scheme handler.
//!
//! A note that shows a picture needs the bytes of a file inside the workspace,
//! and the WebView has **no filesystem capability** — that is the whole posture
//! of `docs/ARCHITECTURE.md` §12, and a `convertFileSrc` on the user's absolute
//! path would hand it one. So the preview writes
//! `notes-asset://<workspace-id>/<relative/path.png>` and this handler turns it
//! back into bytes.
//!
//! **It is a second entry point into the workspace, and it resolves nothing
//! itself.** The path is parsed into a [`RelPath`] and handed to `notes-core`,
//! which applies the same root jail as every command — the string check, then
//! `notes-fs` re-resolving each segment and refusing a symlink. A handler that
//! trusted the URL would be the one door in the building without a lock.

use notes_model::RelPath;
use tauri::http::{Request, Response, StatusCode};
use tauri::UriSchemeContext;

use crate::commands::App;

pub fn serve<R: tauri::Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    match resolve(ctx, &request) {
        Ok((bytes, mime)) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", mime)
            // The response is a picture and nothing else: it may not be framed,
            // it may not fetch, and a browser that guesses at its type may not
            // act on the guess.
            .header("Content-Security-Policy", "default-src 'none'; sandbox")
            .header("X-Content-Type-Options", "nosniff")
            .header("Cache-Control", "no-store")
            .body(bytes)
            .unwrap_or_else(|_| empty(StatusCode::INTERNAL_SERVER_ERROR)),
        Err(status) => empty(status),
    }
}

/// **The body is always empty on failure, deliberately.** A message would say
/// whether a path exists outside the root, which is a question the preview is
/// not entitled to ask; the status code is enough for an `<img>` to show its
/// alt text.
fn empty(status: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("Content-Security-Policy", "default-src 'none'; sandbox")
        .body(Vec::new())
        .expect("an empty response is always well formed")
}

type Served = (Vec<u8>, &'static str);

fn resolve<R: tauri::Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: &Request<Vec<u8>>,
) -> Result<Served, StatusCode> {
    use tauri::Manager;

    let uri = request.uri();
    // `notes-asset://<workspace-id>/<path>`. On Windows and Android the WebView
    // rewrites this to `http://notes-asset.localhost/<path>`, so the workspace
    // id may arrive as a host or as the first segment of the path; both are
    // checked against the open workspace, and neither is trusted to select one.
    let host = uri.host().unwrap_or_default().to_string();
    let raw_path = uri.path().trim_start_matches('/');

    let app = ctx.app_handle();
    let state = app.state::<App>();
    let svc = state
        .svc
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let open_id = svc.workspace_id().ok_or(StatusCode::NOT_FOUND)?.to_string();

    let rest = if host.eq_ignore_ascii_case(&open_id) {
        raw_path
    } else if let Some(rest) = raw_path
        .strip_prefix(&open_id)
        .and_then(|r| r.strip_prefix('/'))
    {
        rest
    } else {
        // A URL for a workspace that is not the open one. Not an error the user
        // can act on, and not a reason to open the other workspace.
        return Err(StatusCode::NOT_FOUND);
    };

    let decoded: String = rest
        .split('/')
        .map(notes_markdown::url::percent_decode)
        .collect::<Vec<_>>()
        .join("/");
    let path = RelPath::parse(&decoded).map_err(|_| StatusCode::FORBIDDEN)?;

    match svc.read_asset(&path) {
        Ok(a) => Ok((a.bytes, a.mime)),
        Err(notes_model::CoreError::Unsupported { .. }) => Err(StatusCode::UNSUPPORTED_MEDIA_TYPE),
        Err(notes_model::CoreError::OutsideRoot { .. })
        | Err(notes_model::CoreError::SymlinkNotFollowed { .. }) => Err(StatusCode::FORBIDDEN),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}
