use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::dto::{TrendingItem, TrendingResponse};
use crate::error::Result;
use crate::state::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct TrendingParams {
    /// Maximum number of items to return. Defaults to the server-configured
    /// `--trending-top-n`; clamped to 1..=100.
    pub limit: Option<usize>,
}

/// Top searches within the rolling window. Empty when `--trending-enabled=false`.
#[utoipa::path(
    get,
    path = "/api/trending",
    tag = "lifecycle",
    params(TrendingParams),
    responses((status = 200, description = "Trending queries", body = TrendingResponse))
)]
pub async fn trending(
    State(state): State<AppState>,
    Query(params): Query<TrendingParams>,
) -> Result<Json<TrendingResponse>> {
    let (window_secs, items) = match &state.trending {
        Some(t) => {
            let limit = params
                .limit
                .unwrap_or_else(|| t.default_top_n())
                .clamp(1, crate::trending::MAX_TOP_N);
            (t.window_secs(), t.top_k(limit))
        }
        None => (0, Vec::new()),
    };
    Ok(Json(TrendingResponse {
        window_secs,
        items: items
            .into_iter()
            .map(|(query, count)| TrendingItem { query, count })
            .collect(),
    }))
}
