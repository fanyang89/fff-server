pub mod health;
pub mod reindex;
pub mod search;
pub mod stats;

use axum::routing::{get, post};
use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::openapi::ApiDoc;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/api/search", get(search::search))
        .route("/api/glob", get(search::glob))
        .route("/api/health", get(health::health))
        .route("/api/stats", get(stats::stats))
        .route("/api/reindex", post(reindex::reindex))
        .route("/api/base-path", get(health::base_path));

    Router::new()
        .merge(api)
        .merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", ApiDoc::openapi()))
        .with_state(state)
}
