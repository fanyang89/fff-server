use std::path::PathBuf;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::dto::{HistoryResponse, TrackRequest, TrackResponse};
use crate::error::{AppError, Result};
use crate::state::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct HistoryParams {
    /// Look up the Nth historical query (0 = most recent).
    #[param(default = 0)]
    pub offset: Option<usize>,
}

/// Retrieve a historical query (most-recent first).
#[utoipa::path(
    get,
    path = "/api/history",
    tag = "history",
    params(HistoryParams),
    responses(
        (status = 200, description = "Historical query", body = HistoryResponse),
        (status = 500, description = "Internal error", body = serde_json::Value),
    )
)]
pub async fn history(
    State(state): State<AppState>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<HistoryResponse>> {
    let offset = params.offset.unwrap_or(0);
    let tracker = state.query_tracker.clone();
    let base_path: PathBuf = state.base_path().to_path_buf();

    let query = tokio::task::spawn_blocking(move || -> Result<Option<String>> {
        let guard = tracker.read()?;
        let qt = guard
            .as_ref()
            .ok_or(AppError::Internal("query tracker not ready".into()))?;
        Ok(qt.get_historical_query(&base_path, offset)?)
    })
    .await
    .map_err(|e| AppError::Internal(format!("history task failed: {e}")))??;

    Ok(Json(HistoryResponse { query }))
}

/// Record a file access (and optionally the query that led to it) to feed
/// frecency ranking and combo-boost scoring.
#[utoipa::path(
    post,
    path = "/api/track",
    tag = "history",
    request_body = TrackRequest,
    responses(
        (status = 200, description = "Tracked", body = TrackResponse),
        (status = 400, description = "Bad request", body = serde_json::Value),
        (status = 500, description = "Internal error", body = serde_json::Value),
    )
)]
pub async fn track(
    State(state): State<AppState>,
    Json(req): Json<TrackRequest>,
) -> Result<Json<TrackResponse>> {
    let base_path = state.base_path().to_path_buf();
    let path_str = req.path.clone();
    let query_opt = req.query.clone();
    let frecency = state.frecency.clone();
    let tracker = state.query_tracker.clone();

    tokio::task::spawn_blocking(move || -> Result<()> {
        let resolved = resolve_path(&path_str, &base_path)?;

        // Frecency access tracking (read-level op on the handle).
        {
            let guard = frecency.read()?;
            if let Some(tr) = guard.as_ref() {
                tr.track_access(&resolved)?;
            }
        }

        if let Some(query) = query_opt {
            let mut guard = tracker.write()?;
            if let Some(qt) = guard.as_mut() {
                qt.track_query_completion(&query, &base_path, &resolved)?;
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| AppError::Internal(format!("track task failed: {e}")))??;

    Ok(Json(TrackResponse { tracked: true }))
}

/// Resolve a user-supplied path (relative or absolute) against the base path.
fn resolve_path(raw: &str, base_path: &std::path::Path) -> Result<PathBuf> {
    let p = PathBuf::from(raw);
    let resolved = if p.is_absolute() {
        p
    } else {
        base_path.join(raw)
    };
    Ok(resolved)
}
