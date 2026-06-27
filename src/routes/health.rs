use std::path::Path;

use axum::Json;
use axum::extract::State;

use crate::dto::{BasePathResponse, FeedbackResponse, FileServerResponse, HealthResponse};
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

/// External file-server base URL (optional). Clients build browse links by
/// appending `/<result.relative_path>` to this URL.
#[utoipa::path(
    get,
    path = "/api/file-server",
    tag = "lifecycle",
    responses((status = 200, description = "File-server base URL", body = FileServerResponse))
)]
pub async fn file_server(State(state): State<AppState>) -> Result<Json<FileServerResponse>> {
    Ok(Json(FileServerResponse {
        url: state.file_server_url.as_deref().map(str::to_owned),
    }))
}

/// Contact email for bug reports and feedback (optional). Clients render a
/// mailto link when present, and hide the feedback entry when null.
#[utoipa::path(
    get,
    path = "/api/feedback",
    tag = "lifecycle",
    responses((status = 200, description = "Feedback contact email", body = FeedbackResponse))
)]
pub async fn feedback(State(state): State<AppState>) -> Result<Json<FeedbackResponse>> {
    Ok(Json(FeedbackResponse {
        email: state.feedback_email.as_deref().map(str::to_owned),
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
