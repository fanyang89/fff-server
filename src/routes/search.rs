use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::dto::SearchResponse;
use crate::error::{AppError, Result};
use crate::state::AppState;

/// Maximum accepted query/pattern length (characters).
const MAX_QUERY_LEN: usize = 256;
/// Maximum accepted pagination offset (deep pagination is expensive and
/// meaningless for this use case).
const MAX_OFFSET: usize = 10000;

#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchParams {
    /// Query string. plocate matches it as a substring (wholename/path) by
    /// default; if it contains glob metacharacters (* ? []) it is treated as
    /// a glob. Multiple patterns are AND-ed.
    pub q: String,
    #[param(default = 100)]
    pub limit: Option<usize>,
    #[param(default = 0)]
    pub offset: Option<usize>,
    /// Case-insensitive match (default true).
    #[param(default = true)]
    pub case: Option<bool>,
    /// `path` (default, match against full path) or `basename`.
    #[param(default = "path")]
    pub scope: Option<String>,
}

/// Filename / path search via the plocate trigram index.
#[utoipa::path(
    get,
    path = "/api/search",
    tag = "search",
    params(SearchParams),
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 500, description = "Internal error", body = serde_json::Value),
    )
)]
pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>> {
    validate_query(&params.q)?;
    let limit = clamp_limit(params.limit, state.max_results);
    let offset = validate_offset(params.offset)?;
    let case_insensitive = params.case.unwrap_or(true);
    let basename_only = matches!(
        params.scope.as_deref(),
        Some("basename") | Some("b") | Some("name")
    );
    let resp = state
        .search(&params.q, limit, offset, case_insensitive, basename_only)
        .await?;
    Ok(Json(resp))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct GlobParams {
    /// Glob pattern, e.g. `*.rs` or `**/2024/*.log`.
    pub pattern: String,
    #[param(default = 100)]
    pub limit: Option<usize>,
    #[param(default = 0)]
    pub offset: Option<usize>,
    /// Case-insensitive match (default true).
    #[param(default = true)]
    pub case: Option<bool>,
}

/// Glob search. Identical to `/api/search` but explicit about glob intent.
#[utoipa::path(
    get,
    path = "/api/glob",
    tag = "search",
    params(GlobParams),
    responses(
        (status = 200, description = "Glob results", body = SearchResponse),
        (status = 500, description = "Internal error", body = serde_json::Value),
    )
)]
pub async fn glob(
    State(state): State<AppState>,
    Query(params): Query<GlobParams>,
) -> Result<Json<SearchResponse>> {
    validate_query(&params.pattern)?;
    let limit = clamp_limit(params.limit, state.max_results);
    let offset = validate_offset(params.offset)?;
    let case_insensitive = params.case.unwrap_or(true);
    let resp = state
        .search(&params.pattern, limit, offset, case_insensitive, false)
        .await?;
    Ok(Json(resp))
}

fn clamp_limit(req: Option<usize>, max: usize) -> usize {
    req.unwrap_or(max).clamp(1, max.max(1))
}

fn validate_offset(req: Option<usize>) -> Result<usize> {
    match req {
        Some(n) if n > MAX_OFFSET => Err(AppError::BadRequest(format!(
            "offset too large (max {MAX_OFFSET})"
        ))),
        Some(n) => Ok(n),
        None => Ok(0),
    }
}

fn validate_query(q: &str) -> Result<()> {
    if q.chars().count() > MAX_QUERY_LEN {
        return Err(AppError::BadRequest(format!(
            "query too long (max {MAX_QUERY_LEN} chars)"
        )));
    }
    Ok(())
}
