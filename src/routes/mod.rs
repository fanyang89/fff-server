pub mod frontend;
pub mod health;
pub mod reindex;
pub mod search;
pub mod stats;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use utoipa_swagger_ui::SwaggerUi;

use crate::mcp::PlocateMcpHandler;
use crate::openapi::ApiDoc;
use crate::state::AppState;

/// Build the production router. When `--public-base-url` is configured the
/// entire surface (REST + Swagger + OpenAPI + MCP + frontend) is nested under
/// the path prefix; otherwise the server is mounted at `/` (legacy behavior).
pub fn router(state: AppState) -> Router {
    let prefix = state.base_url_prefix.as_str().to_owned();
    // Prefer the full canonical URL for OpenAPI `servers` (so Swagger "Try it
    // out" hits the public origin); fall back to the bare prefix path.
    let openapi_server = state
        .base_url_public
        .as_deref()
        .map(|s| s.to_string())
        .or_else(|| {
            let p = state.base_url_prefix.as_str();
            if p.is_empty() {
                None
            } else {
                Some(p.to_string())
            }
        });

    let inner = inner_router(state, openapi_server.as_deref());

    if prefix.is_empty() {
        inner
    } else {
        // Nest every endpoint under the prefix. axum's `nest` does not match
        // the bare prefix (e.g. `/search`), so add an explicit permanent
        // redirect to `/search/` — otherwise a user typing `/search` would
        // land on the SPA fallback with the browser resolving relative
        // asset URLs against `/`, breaking asset loading.
        let redirect_to = format!("{prefix}/");
        Router::new()
            .route(&prefix, get(move || async move {
                axum::response::Redirect::permanent(&redirect_to)
            }))
            .nest(&prefix, inner)
    }
}

/// Inner router built at root paths; the outer `router()` may nest it under
/// a prefix. `openapi_server` is the value (if any) to inject into the
/// OpenAPI `servers` field.
fn inner_router(state: AppState, openapi_server: Option<&str>) -> Router {
    let api = Router::new()
        .route("/api/search", get(search::search))
        .route("/api/glob", get(search::glob))
        .route("/api/fuzzy", get(search::fuzzy))
        .route("/api/health", get(health::health))
        .route("/api/stats", get(stats::stats))
        .route("/api/reindex", post(reindex::reindex))
        .route("/api/base-path", get(health::base_path))
        .route("/api/file-server", get(health::file_server))
        .route("/api/feedback", get(health::feedback));

    // MCP / Streamable HTTP endpoint — shares the same engine/state as REST.
    let mcp_state = state.clone();
    let mcp_service: StreamableHttpService<PlocateMcpHandler, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(PlocateMcpHandler::new(mcp_state.clone())),
            Arc::new(LocalSessionManager::default()),
            // Stateless mode: each JSON-RPC request is self-contained, no
            // Mcp-Session-Id handshake required — simplest for tool-only agents.
            StreamableHttpServerConfig::default().with_stateful_mode(false),
        );

    Router::new()
        .merge(api)
        .merge(SwaggerUi::new("/swagger-ui").url(
            "/openapi.json",
            ApiDoc::openapi_with_server(openapi_server),
        ))
        .nest_service("/mcp", mcp_service)
        .fallback(frontend::frontend_fallback)
        .with_state(state)
}
