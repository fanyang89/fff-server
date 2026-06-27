use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileItemDto {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    pub relative_path: String,
    pub absolute_path: String,
    /// Unix bytes; null for directories.
    pub size: Option<u64>,
    /// Unix modified timestamp (seconds); null for directories.
    pub modified: Option<u64>,
    /// Short git status label, e.g. "modified", "untracked". null when not in a git repo.
    pub git_status: Option<String>,
    /// Present only for files.
    pub is_binary: Option<bool>,
    /// Total frecency score.
    pub score: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchResponse {
    pub total_matched: usize,
    pub total_files: usize,
    pub total_dirs: Option<usize>,
    pub items: Vec<FileItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub ok: bool,
    pub base_path: String,
    pub mode: String,
    pub live_file_count: usize,
    pub scanning: bool,
    pub watcher_ready: bool,
    pub warmup_complete: bool,
    pub frecency_ok: bool,
    pub query_tracker_ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScanProgressResponse {
    pub scanned_files_count: usize,
    pub is_scanning: bool,
    pub is_watcher_ready: bool,
    pub is_warmup_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BasePathResponse {
    pub base_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RescanResponse {
    pub started: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshGitResponse {
    pub statuses_updated: usize,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TrackRequest {
    /// Relative or absolute path of the file that was opened.
    pub path: String,
    /// The search query that led to opening this file (optional).
    pub query: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TrackResponse {
    pub tracked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HistoryResponse {
    pub query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatsProcess {
    pub pid: u32,
    /// Resident set size in bytes.
    pub rss_bytes: u64,
    pub threads: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatsIndex {
    pub live_file_count: usize,
    pub total_files_seen: usize,
    pub dir_count: usize,
    pub scanning: bool,
    pub watcher_ready: bool,
    pub warmup_complete: bool,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatsCache {
    pub cached_files: usize,
    pub cached_bytes: u64,
    pub max_files: usize,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatsResponse {
    pub process: StatsProcess,
    pub index: StatsIndex,
    pub cache: StatsCache,
}
