use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileItemDto {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    pub relative_path: String,
    pub absolute_path: String,
    /// Fuzzy relevance score (only populated by `/api/fuzzy` and the
    /// `fuzzy_search` MCP tool). Higher is better. Absent for plain search.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub score: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchResponse {
    /// Number of entries matched up to the (offset+limit) cap requested from
    /// plocate. Not an exact total over the whole index.
    pub total_matched: usize,
    /// True when plocate hit the request cap, i.e. more matches likely exist.
    pub truncated: bool,
    pub items: Vec<FileItemDto>,
}

impl SearchResponse {
    pub fn empty() -> Self {
        Self {
            total_matched: 0,
            truncated: false,
            items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub ok: bool,
    pub base_path: String,
    /// Skill/MCP instance name surfaced in the web UI install dialog.
    pub instance_name: String,
    pub db_present: bool,
    pub db_mtime_unix: Option<u64>,
    pub db_size_bytes: Option<u64>,
    pub reindexing: bool,
    pub plocate_available: bool,
    pub updatedb_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatsResponse {
    pub process: StatsProcess,
    pub index: StatsIndex,
    pub last_reindex: Option<ReindexRecordDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatsProcess {
    pub pid: u32,
    pub rss_bytes: u64,
    pub threads: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatsIndex {
    pub db_present: bool,
    pub db_size_bytes: Option<u64>,
    pub db_mtime_unix: Option<u64>,
    pub reindexing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReindexRecordDto {
    pub started_at_unix: u64,
    pub duration_secs: f64,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BasePathResponse {
    pub base_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReindexResponse {
    /// "started" or "already-running".
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileServerResponse {
    /// Base URL of an external file-browsing service, or null if unconfigured.
    /// Clients append `/<result.relative_path>` (URL-encoding each segment).
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeedbackResponse {
    /// Contact email for bug reports and feedback, or null if unconfigured.
    pub email: Option<String>,
}
