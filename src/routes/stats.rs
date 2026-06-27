use axum::extract::State;
use axum::Json;

use crate::dto::{ReindexRecordDto, StatsIndex, StatsProcess, StatsResponse};
use crate::error::Result;
use crate::routes::health::db_meta;
use crate::state::{proc_status, AppState};

/// Runtime statistics: process RSS/threads, index file, last reindex run.
#[utoipa::path(
    get,
    path = "/api/stats",
    tag = "lifecycle",
    responses((status = 200, description = "Runtime stats", body = StatsResponse))
)]
pub async fn stats(State(state): State<AppState>) -> Result<Json<StatsResponse>> {
    let (db_mtime_unix, db_size_bytes) = db_meta(&state);
    let reindexing = state.is_reindexing();
    let (rss_bytes, threads) = proc_status().unwrap_or((0, 0));
    let pid = std::process::id();

    let last_reindex = state.last_run().map(|r| ReindexRecordDto {
        started_at_unix: r
            .started_at
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        duration_secs: r.duration_secs,
        success: r.success,
        error: r.error,
    });

    Ok(Json(StatsResponse {
        process: StatsProcess {
            pid,
            rss_bytes,
            threads,
        },
        index: StatsIndex {
            db_present: state.db_exists(),
            db_size_bytes,
            db_mtime_unix,
            reindexing,
        },
        last_reindex,
    }))
}
