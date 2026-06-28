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
//!
//! When the server is mounted under a public base URL prefix, the index.html
//! bytes are rewritten once at startup to inject `<base href="{prefix}/">`
//! (so the browser resolves relative asset URLs under the prefix) and a
//! `window.__BASE_PATH__` script (so the SPA's runtime fetch/URL construction
//! prepends the prefix).

use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

use crate::state::AppState;

#[derive(Embed)]
#[folder = "web/dist/"]
struct WebAsset;

pub async fn frontend_fallback(State(state): State<AppState>, req: Request) -> Response {
    let full_path = req.uri().path();
    let prefix = state.base_url_prefix.as_str();

    // When the server is mounted under a prefix, the SPA exists ONLY under
    // that prefix. Requests outside it (e.g. `/api/health` without the
    // prefix, or the bare root `/`) must 404 rather than serve the SPA —
    // otherwise the fallback would mask the fact that the caller hit the
    // wrong path.
    if !prefix.is_empty() && !full_path.starts_with(prefix) {
        return (StatusCode::NOT_FOUND, "not under configured base url").into_response();
    }

    // Strip the prefix (if any) before looking up embedded assets.
    let rel = full_path.strip_prefix(prefix).unwrap_or(full_path);
    let raw = rel.trim_start_matches('/');
    let path = if raw.is_empty() { "index.html" } else { raw };

    // Direct asset hit. `index.html` is the only asset that needs the prefix
    // rewrite; everything else (JS/CSS/fonts) is served verbatim.
    if path == "index.html"
        && let Some(rewritten) = state.index_html_override.as_deref()
    {
        return serve("index.html", rewritten);
    }
    if let Some(file) = WebAsset::get(path) {
        return serve(path, &file.data);
    }

    // SPA fallback: unknown non-asset paths serve the app shell. Prefer the
    // rewritten index.html when a prefix is configured.
    if let Some(rewritten) = state.index_html_override.as_deref() {
        return serve("index.html", rewritten);
    }
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

/// Compute the index.html bytes with prefix-aware injection. Returns None
/// when the frontend has not been built (no index.html in the embed).
///
/// When `prefix` is empty the original bytes are returned unchanged. When
/// non-empty, two tags are injected just before `</head>`:
///  1. `<base href="{prefix}/">` — makes the browser resolve the build's
///     relative asset URLs (`./assets/...`) against the prefix.
///  2. `<script>window.__BASE_PATH__="{prefix}";</script>` — read by the
///     SPA's `lib/config.ts` to prefix runtime fetch/URL construction.
pub fn rewrite_index_html(prefix: &str) -> Option<Vec<u8>> {
    let original = WebAsset::get("index.html")?;
    if prefix.is_empty() {
        return Some(original.data.into_owned());
    }
    let html = String::from_utf8_lossy(&original.data);
    let injection =
        format!(r#"<base href="{prefix}/"><script>window.__BASE_PATH__="{prefix}";</script>"#);
    // replacen(_, _, 1) injects exactly once; falls back to appending if the
    // built HTML ever drops its </head> (defensive — should never happen).
    let rewritten = match html.replacen("</head>", &format!("{injection}</head>"), 1) {
        s if s.contains(&injection) => s,
        s => format!("{s}{injection}"),
    };
    Some(rewritten.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::rewrite_index_html;

    #[test]
    fn rewrite_empty_prefix_is_noop() {
        // When the embed has no index.html (e.g. fresh clone before web-build),
        // the function returns None — assert that contract plus the passthrough
        // shape when an index.html IS present.
        match rewrite_index_html("") {
            None => {} // frontend not built in this checkout — acceptable
            Some(bytes) => {
                let s = String::from_utf8_lossy(&bytes);
                // Empty prefix must not inject anything.
                assert!(!s.contains("__BASE_PATH__"));
                assert!(!s.contains("<base href="));
            }
        }
    }

    #[test]
    fn rewrite_with_prefix_injects_both_tags() {
        match rewrite_index_html("/search") {
            None => {} // frontend not built — skip
            Some(bytes) => {
                let s = String::from_utf8_lossy(&bytes);
                assert!(
                    s.contains(r#"<base href="/search/">"#),
                    "base href must be injected with trailing slash"
                );
                assert!(
                    s.contains(r#"window.__BASE_PATH__="/search""#),
                    "BASE_PATH script must be injected"
                );
                // Exactly one injection.
                assert_eq!(s.matches("<base href=").count(), 1);
            }
        }
    }
}
