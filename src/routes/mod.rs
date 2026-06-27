pub mod history;
pub mod lifecycle;
pub mod search;

use axum::routing::{get, post};
use axum::Router;
use utoipa_swagger_ui::SwaggerUi;

use crate::openapi::ApiDoc;
use crate::state::AppState;
use utoipa::OpenApi;

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/api/search", get(search::search))
        .route("/api/glob", get(search::glob))
        .route("/api/history", get(history::history))
        .route("/api/track", post(history::track))
        .route("/api/health", get(lifecycle::health))
        .route("/api/scan-progress", get(lifecycle::scan_progress))
        .route("/api/rescan", post(lifecycle::rescan))
        .route("/api/refresh-git", post(lifecycle::refresh_git))
        .route("/api/base-path", get(lifecycle::base_path));

    Router::new()
        .merge(api)
        .merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", ApiDoc::openapi()))
        .with_state(state)
}
