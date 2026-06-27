use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime};

use tokio::process::Command;

use crate::config::Config;
use crate::dto::{FileItemDto, SearchResponse};
use crate::error::{AppError, Result};

/// Indexing root + plocate database handles shared with every handler.
#[derive(Clone)]
pub struct AppState {
    pub base_path: Arc<PathBuf>,
    pub db_path: Arc<PathBuf>,
    pub plocate_bin: Arc<OsString>,
    pub updatedb_bin: Arc<OsString>,
    pub max_results: usize,
    reindexing: Arc<AtomicBool>,
    last_run: Arc<Mutex<Option<ReindexRecord>>>,
}

#[derive(Clone, Debug)]
pub struct ReindexRecord {
    pub started_at: SystemTime,
    pub duration_secs: f64,
    pub success: bool,
    pub error: Option<String>,
}

pub enum ReindexOutcome {
    Started,
    AlreadyRunning,
}

impl AppState {
    pub fn new(cfg: &Config) -> Result<Self> {
        let base_path = cfg
            .base_path
            .canonicalize()
            .map_err(|e| AppError::BadRequest(format!("base_path invalid: {e}")))?;
        let db_path = cfg.resolved_db_path();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(Self {
            base_path: Arc::new(base_path),
            db_path: Arc::new(db_path),
            plocate_bin: Arc::new(cfg.plocate_bin.clone().into()),
            updatedb_bin: Arc::new(cfg.updatedb_bin.clone().into()),
            max_results: cfg.max_results,
            reindexing: Arc::new(AtomicBool::new(false)),
            last_run: Arc::new(Mutex::new(None)),
        })
    }

    pub fn db_exists(&self) -> bool {
        self.db_path.is_file()
    }

    pub fn last_run(&self) -> Option<ReindexRecord> {
        self.last_run.lock().ok().and_then(|g| g.clone())
    }

    pub fn is_reindexing(&self) -> bool {
        self.reindexing.load(Ordering::Acquire)
    }

    /// Run a plocate query (substring by default; glob when the pattern
    /// contains glob metacharacters, per plocate's own rules).
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
        case_insensitive: bool,
        basename_only: bool,
    ) -> Result<SearchResponse> {
        if !self.db_exists() {
            return Ok(SearchResponse::empty());
        }
        let cap = offset.saturating_add(limit);
        let raw = self
            .run_plocate(query, Some(cap), case_insensitive, basename_only)
            .await?;
        let items = parse_paths(&raw, &self.base_path);
        let total_returned = items.len();
        let truncated = total_returned == cap && cap > 0;
        let paged: Vec<FileItemDto> = items
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();
        Ok(SearchResponse {
            total_matched: total_returned,
            truncated,
            items: paged,
        })
    }

    async fn run_plocate(
        &self,
        pattern: &str,
        limit: Option<usize>,
        case_insensitive: bool,
        basename_only: bool,
    ) -> Result<Vec<u8>> {
        let mut cmd = Command::new(&*self.plocate_bin);
        cmd.arg("-d").arg(&*self.db_path).arg("-N").arg("-0");
        if case_insensitive {
            cmd.arg("-i");
        }
        if basename_only {
            cmd.arg("-b");
        }
        if let Some(n) = limit {
            cmd.arg("-l").arg(n.to_string());
        }
        cmd.arg("--").arg(pattern);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let output = cmd.output().await.map_err(|e| {
            AppError::Internal(format!("failed to run plocate: {e} (is it installed?)"))
        })?;
        if !output.status.success() {
            // Non-zero usually means "no matches" for some plocate versions; treat empty.
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No such file") || stderr.contains("cannot stat") {
                return Ok(Vec::new());
            }
            return Err(AppError::Internal(format!(
                "plocate failed ({}): {stderr}",
                output.status
            )));
        }
        Ok(output.stdout)
    }

    /// Trigger a background `updatedb` run if one is not already in flight.
    pub fn trigger_reindex(self) -> ReindexOutcome {
        if self.reindexing.swap(true, Ordering::AcqRel) {
            return ReindexOutcome::AlreadyRunning;
        }
        let state = self.clone();
        tokio::spawn(async move {
            let started = Instant::now();
            let started_at = SystemTime::now();
            let outcome = run_updatedb(&state).await;
            let rec = ReindexRecord {
                started_at,
                duration_secs: started.elapsed().as_secs_f64(),
                success: outcome.is_ok(),
                error: outcome.err().map(|e| e.to_string()),
            };
            tracing::info!(
                success = rec.success,
                duration_secs = %format!("{:.1}", rec.duration_secs),
                "reindex completed"
            );
            if let Ok(mut g) = state.last_run.lock() {
                *g = Some(rec);
            }
            state.reindexing.store(false, Ordering::Release);
        });
        ReindexOutcome::Started
    }

    /// Spawn the periodic reindex loop. Cancel by aborting the returned handle.
    pub fn spawn_reindex_interval(self, interval_secs: u64) -> Option<tokio::task::JoinHandle<()>> {
        if interval_secs == 0 {
            return None;
        }
        let state = self.clone();
        Some(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            // Skip the immediate first tick (no churn at startup).
            ticker.tick().await;
            loop {
                ticker.tick().await;
                if !state.is_reindexing() {
                    state.clone().trigger_reindex();
                }
            }
        }))
    }
}

async fn run_updatedb(state: &AppState) -> Result<()> {
    let mut cmd = Command::new(&*state.updatedb_bin);
    cmd.arg("-U")
        .arg(&*state.base_path)
        .arg("-o")
        .arg(&*state.db_path)
        .arg("--require-visibility")
        .arg("no")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let output = cmd.output().await.map_err(|e| {
        AppError::Internal(format!("failed to run updatedb: {e} (is it installed?)"))
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!(
            "updatedb failed ({}): {stderr}",
            output.status
        )));
    }
    Ok(())
}

/// Parse NUL-separated plocate output into DTO items.
fn parse_paths(raw: &[u8], base_path: &Path) -> Vec<FileItemDto> {
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(|&b| b == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| {
            // plocate output is UTF-8 bytes; filesystems may contain non-UTF-8,
            // lossy-convert those rare cases.
            let abs = String::from_utf8_lossy(chunk).into_owned();
            build_item(&abs, base_path)
        })
        .collect()
}

fn build_item(abs: &str, base_path: &Path) -> FileItemDto {
    let is_dir = abs.ends_with('/') || abs.ends_with(std::path::MAIN_SEPARATOR);
    let abs_trimmed = abs.trim_end_matches('/').trim_end_matches(std::path::MAIN_SEPARATOR);
    let relative = abs_trimmed
        .strip_prefix(base_path.to_string_lossy().as_ref())
        .map(|r| r.trim_start_matches('/').to_owned())
        .unwrap_or_else(|| abs_trimmed.to_string());
    let name = abs_trimmed
        .rsplit(['/', std::path::MAIN_SEPARATOR])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(abs_trimmed)
        .to_string();
    FileItemDto {
        kind: if is_dir { "directory" } else { "file" }.into(),
        name,
        relative_path: relative,
        absolute_path: abs_trimmed.into(),
    }
}

/// Read RSS (bytes) and thread count from `/proc/self/status` (Linux).
pub fn proc_status() -> io::Result<(u64, u32)> {
    let content = std::fs::read_to_string("/proc/self/status")?;
    let mut rss: Option<u64> = None;
    let mut threads: Option<u32> = None;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            rss = rest.split_whitespace().next().and_then(|v| v.parse::<u64>().ok());
        } else if let Some(rest) = line.strip_prefix("Threads:") {
            threads = rest.split_whitespace().next().and_then(|v| v.parse::<u32>().ok());
        }
        if rss.is_some() && threads.is_some() {
            break;
        }
    }
    Ok((rss.unwrap_or(0).saturating_mul(1024), threads.unwrap_or(0)))
}
