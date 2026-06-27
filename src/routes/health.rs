use std::path::Path;

use axum::extract::State;
use axum::Json;

use crate::dto::{BasePathResponse, HealthResponse};
use crate::error::Result;
use crate::state::AppState;

/// Service + index health.
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "lifecycle",
    responses(
        (status = 200, description = "Health", body = HealthResponse),
        (status = 503, description = "Index unavailable", body = HealthResponse),
    )
)]
pub async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>> {
    let db_present = state.db_exists();
    let (db_mtime_unix, db_size_bytes) = db_meta(&state);
    let plocate_available = which(&state.plocate_bin.as_ref().to_string_lossy());
    let updatedb_available = which(&state.updatedb_bin.as_ref().to_string_lossy());
    let reindexing = state.is_reindexing();
    let ok = db_present && plocate_available && updatedb_available && !reindexing;

    Ok(Json(HealthResponse {
        ok,
        base_path: state.base_path.to_string_lossy().into_owned(),
        db_present,
        db_mtime_unix,
        db_size_bytes,
        reindexing,
        plocate_available,
        updatedb_available,
    }))
}

/// Return the currently indexed base path.
#[utoipa::path(
    get,
    path = "/api/base-path",
    tag = "lifecycle",
    responses((status = 200, description = "Base path", body = BasePathResponse))
)]
pub async fn base_path(State(state): State<AppState>) -> Result<Json<BasePathResponse>> {
    Ok(Json(BasePathResponse {
        base_path: state.base_path.to_string_lossy().into_owned(),
    }))
}

pub(crate) fn db_meta(state: &AppState) -> (Option<u64>, Option<u64>) {
    match std::fs::metadata(&*state.db_path) {
        Ok(m) => (
            m.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            Some(m.len()),
        ),
        Err(_) => (None, None),
    }
}

fn which(bin: &str) -> bool {
    // Resolve via PATH the same way the subprocess would.
    if bin.contains('/') {
        return Path::new(bin).is_file();
    }
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths)
                .map(|p| p.join(bin))
                .any(|p| p.is_file())
        })
        .unwrap_or(false)
}
