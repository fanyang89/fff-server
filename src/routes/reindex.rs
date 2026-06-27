use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::dto::ReindexResponse;
use crate::error::Result;
use crate::state::{AppState, ReindexOutcome};

/// Trigger a background `updatedb` run to refresh the index.
///
/// Returns `200 {"status":"started"}` when a new run begins, or
/// `202 {"status":"already-running"}` when one is already in flight.
#[utoipa::path(
    post,
    path = "/api/reindex",
    tag = "lifecycle",
    responses(
        (status = 200, description = "Reindex started", body = ReindexResponse),
        (status = 202, description = "Already running", body = ReindexResponse),
    )
)]
pub async fn reindex(State(state): State<AppState>) -> Result<(StatusCode, Json<ReindexResponse>)> {
    match state.trigger_reindex() {
        ReindexOutcome::Started => Ok((
            StatusCode::OK,
            Json(ReindexResponse {
                status: "started".into(),
            }),
        )),
        ReindexOutcome::AlreadyRunning => Ok((
            StatusCode::ACCEPTED,
            Json(ReindexResponse {
                status: "already-running".into(),
            }),
        )),
    }
}
