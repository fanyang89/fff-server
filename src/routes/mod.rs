pub mod health;
pub mod reindex;
pub mod search;
pub mod stats;

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::mcp::PlocateMcpHandler;
use crate::openapi::ApiDoc;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/api/search", get(search::search))
        .route("/api/glob", get(search::glob))
        .route("/api/health", get(health::health))
        .route("/api/stats", get(stats::stats))
        .route("/api/reindex", post(reindex::reindex))
        .route("/api/base-path", get(health::base_path))
        .route("/api/file-server", get(health::file_server));

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
        .merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", ApiDoc::openapi()))
        .nest_service("/mcp", mcp_service)
        .with_state(state)
}
