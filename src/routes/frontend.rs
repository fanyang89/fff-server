//! Embedded single-page frontend.
//!
//! `web/dist` (built by Vite) is embedded into the binary at release compile
//! time via `rust-embed`; in debug builds it is read from the filesystem on
//! each request so frontend rebuilds do not require recompiling Rust.
//!
//! The `frontend_fallback` handler is installed as the router fallback, so it
//! only sees requests not matched by `/api/*`, `/swagger-ui`, `/openapi.json`
//! or `/mcp`. It serves a matching asset directly, or `index.html` for any
//! other path (standard SPA behavior).

use axum::body::Bytes;
use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "web/dist/"]
struct WebAsset;

pub async fn frontend_fallback(req: Request) -> Response {
    let raw = req.uri().path().trim_start_matches('/');
    let path = if raw.is_empty() { "index.html" } else { raw };

    if let Some(file) = WebAsset::get(path) {
        return serve(path, &file.data);
    }

    // SPA fallback: unknown non-asset paths serve the app shell.
    if let Some(idx) = WebAsset::get("index.html") {
        return serve("index.html", &idx.data);
    }

    (
        StatusCode::NOT_FOUND,
        "frontend not built — run `task web-build`",
    )
        .into_response()
}

fn serve(path: &str, data: &[u8]) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    (
        [(header::CONTENT_TYPE, mime.as_ref())],
        Bytes::copy_from_slice(data),
    )
        .into_response()
}
