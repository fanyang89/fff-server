use std::fs;

use axum::extract::State;
use axum::Json;

use crate::dto::{StatsCache, StatsIndex, StatsProcess, StatsResponse};
use crate::error::{AppError, Result};
use crate::state::AppState;

/// Process and engine runtime statistics (RSS, threads, index size, cache use).
#[utoipa::path(
    get,
    path = "/api/stats",
    tag = "lifecycle",
    responses(
        (status = 200, description = "Runtime stats", body = StatsResponse),
        (status = 500, description = "Internal error", body = serde_json::Value),
    )
)]
pub async fn stats(State(state): State<AppState>) -> Result<Json<StatsResponse>> {
    let picker = state.picker.clone();
    let mode = state.mode;
    let pid = std::process::id();

    let resp = tokio::task::spawn_blocking(move || -> Result<StatsResponse> {
        let (rss_bytes, threads) = read_proc_status()?;
        let guard = picker.read()?;
        let picker_ref = guard
            .as_ref()
            .ok_or(AppError::Internal("file picker not ready".into()))?;

        let progress = picker_ref.get_scan_progress();
        let budget = picker_ref.cache_budget();
        let cache = StatsCache {
            cached_files: budget.cached_count.load(std::sync::atomic::Ordering::Relaxed),
            cached_bytes: budget.cached_bytes.load(std::sync::atomic::Ordering::Relaxed),
            max_files: budget.max_files,
            max_bytes: budget.max_bytes,
        };

        let index = StatsIndex {
            live_file_count: picker_ref.live_file_count(),
            total_files_seen: picker_ref.get_files().len(),
            dir_count: picker_ref.get_dirs().len(),
            scanning: progress.is_scanning,
            watcher_ready: progress.is_watcher_ready,
            warmup_complete: progress.is_warmup_complete,
            mode: if mode.is_ai() { "ai" } else { "neovim" }.into(),
        };

        Ok(StatsResponse {
            process: StatsProcess {
                pid,
                rss_bytes,
                threads,
            },
            index,
            cache,
        })
    })
    .await
    .map_err(|e| AppError::Internal(format!("stats task failed: {e}")))??;

    Ok(Json(resp))
}

/// Parse `/proc/self/status` for `VmRSS` (in kB) and `Threads`.
fn read_proc_status() -> Result<(u64, u32)> {
    let content = fs::read_to_string("/proc/self/status")
        .map_err(|e| AppError::Internal(format!("failed to read /proc/self/status: {e}")))?;
    let mut rss_kb: Option<u64> = None;
    let mut threads: Option<u32> = None;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            rss_kb = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok());
        } else if let Some(rest) = line.strip_prefix("Threads:") {
            threads = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u32>().ok());
        }
        if rss_kb.is_some() && threads.is_some() {
            break;
        }
    }
    let rss = rss_kb.unwrap_or(0).saturating_mul(1024);
    let thr = threads.unwrap_or(0);
    Ok((rss, thr))
}
