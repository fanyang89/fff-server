use axum::extract::State;
use axum::Json;

use crate::dto::{
    BasePathResponse, HealthResponse, RefreshGitResponse, RescanResponse, ScanProgressResponse,
};
use crate::error::{AppError, Result};
use crate::state::AppState;

/// Service + engine health check.
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "lifecycle",
    responses(
        (status = 200, description = "Health", body = HealthResponse),
        (status = 503, description = "Unavailable", body = HealthResponse),
    )
)]
pub async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>> {
    let picker = state.picker.clone();
    let frecency = state.frecency.clone();
    let tracker = state.query_tracker.clone();
    let mode = state.mode;

    let resp = tokio::task::spawn_blocking(move || -> Result<HealthResponse> {
        let guard = picker.read()?;
        let picker_ref = guard
            .as_ref()
            .ok_or(AppError::Internal("file picker not ready".into()))?;
        let progress = picker_ref.get_scan_progress();
        let frecency_ok = frecency.read().ok().and_then(|g| g.as_ref().map(|_| true)).unwrap_or(false);
        let query_tracker_ok = tracker
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|_| true))
            .unwrap_or(false);
        Ok(HealthResponse {
            ok: !progress.is_scanning && frecency_ok && query_tracker_ok,
            base_path: picker_ref.base_path().to_string_lossy().into_owned(),
            mode: if mode.is_ai() { "ai" } else { "neovim" }.into(),
            live_file_count: picker_ref.live_file_count(),
            scanning: progress.is_scanning,
            watcher_ready: progress.is_watcher_ready,
            warmup_complete: progress.is_warmup_complete,
            frecency_ok,
            query_tracker_ok,
        })
    })
    .await
    .map_err(|e| AppError::Internal(format!("health task failed: {e}")))??;

    Ok(Json(resp))
}

/// Current scan progress.
#[utoipa::path(
    get,
    path = "/api/scan-progress",
    tag = "lifecycle",
    responses((status = 200, description = "Progress", body = ScanProgressResponse))
)]
pub async fn scan_progress(State(state): State<AppState>) -> Result<Json<ScanProgressResponse>> {
    let picker = state.picker.clone();
    let resp = tokio::task::spawn_blocking(move || -> Result<ScanProgressResponse> {
        let guard = picker.read()?;
        let picker_ref = guard
            .as_ref()
            .ok_or(AppError::Internal("file picker not ready".into()))?;
        let p = picker_ref.get_scan_progress();
        Ok(ScanProgressResponse {
            scanned_files_count: p.scanned_files_count,
            is_scanning: p.is_scanning,
            is_watcher_ready: p.is_watcher_ready,
            is_warmup_complete: p.is_warmup_complete,
        })
    })
    .await
    .map_err(|e| AppError::Internal(format!("scan-progress task failed: {e}")))??;

    Ok(Json(resp))
}

/// Trigger a full rescan of the indexed tree.
#[utoipa::path(
    post,
    path = "/api/rescan",
    tag = "lifecycle",
    responses((status = 200, description = "Rescan triggered", body = RescanResponse))
)]
pub async fn rescan(State(state): State<AppState>) -> Result<Json<RescanResponse>> {
    let picker = state.picker.clone();
    let frecency = state.frecency.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        picker.trigger_full_rescan_async(&frecency)?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::Internal(format!("rescan task failed: {e}")))??;

    Ok(Json(RescanResponse { started: true }))
}

/// Refresh cached git statuses for all indexed files.
#[utoipa::path(
    post,
    path = "/api/refresh-git",
    tag = "lifecycle",
    responses((status = 200, description = "Git status refreshed", body = RefreshGitResponse))
)]
pub async fn refresh_git(State(state): State<AppState>) -> Result<Json<RefreshGitResponse>> {
    let picker = state.picker.clone();
    let frecency = state.frecency.clone();
    let count = tokio::task::spawn_blocking(move || -> Result<usize> {
        Ok(picker.refresh_git_status(&frecency)?)
    })
    .await
    .map_err(|e| AppError::Internal(format!("refresh-git task failed: {e}")))??;

    Ok(Json(RefreshGitResponse {
        statuses_updated: count,
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
        base_path: state.base_path().to_string_lossy().into_owned(),
    }))
}
