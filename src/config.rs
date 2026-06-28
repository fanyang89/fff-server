use std::path::PathBuf;

use clap::Parser;

/// RESTful filename-search API server backed by a plocate database.
///
/// Builds and queries a dedicated plocate index for a single directory tree,
/// refreshed periodically by `updatedb`. The index lives on disk, so a process
/// restart never rescans.
#[derive(Debug, Clone, Parser)]
#[command(name = "plocate-server", version, about)]
pub struct Config {
    /// Root directory to index.
    #[arg(long, env = "PLOCATE_SERVER_BASE_PATH")]
    pub base_path: PathBuf,

    /// Bind address for the HTTP server.
    #[arg(long, env = "PLOCATE_SERVER_BIND", default_value = "127.0.0.1:8787")]
    pub bind: String,

    /// Path to the plocate database (created/refreshed by updatedb).
    #[arg(long, env = "PLOCATE_SERVER_DB_PATH")]
    pub db_path: Option<PathBuf>,

    /// Override the `plocate` binary path.
    #[arg(long, env = "PLOCATE_SERVER_PLOCATE_BIN", default_value = "plocate")]
    pub plocate_bin: String,

    /// Override the `updatedb` binary path.
    #[arg(long, env = "PLOCATE_SERVER_UPDATEDB_BIN", default_value = "updatedb")]
    pub updatedb_bin: String,

    /// Maximum results returned by a single search/glob call.
    #[arg(long, env = "PLOCATE_SERVER_MAX_RESULTS", default_value_t = 100)]
    pub max_results: usize,

    /// Maximum concurrent plocate query processes. Excess requests wait
    /// (backpressure) rather than spawning unbounded children.
    #[arg(
        long,
        env = "PLOCATE_SERVER_MAX_CONCURRENT_SEARCHES",
        default_value_t = 8
    )]
    pub max_concurrent_searches: usize,

    /// Per-phase timeout (seconds). Applied to (1) the plocate child process
    /// and (2) the per-path `stat` fan-out that follows. Worst-case request
    /// latency is therefore up to 2× this value. A plocate run exceeding it
    /// is killed and reported as a 504; the stat fan-out stops early and the
    /// response is returned with `truncated=true`. Note this does NOT bound
    /// the time spent waiting for a concurrency slot — see
    /// `--queue-timeout-secs` for that.
    #[arg(long, env = "PLOCATE_SERVER_SEARCH_TIMEOUT_SECS", default_value_t = 10)]
    pub search_timeout_secs: u64,

    /// Maximum time (seconds) a request waits for a concurrency slot before
    /// returning 503. Distinct from `--search-timeout-secs`: this bounds the
    /// queue wait (admission control), not the query itself. Set to 0 to wait
    /// forever (legacy behavior; not recommended — clients will silent-timeout
    /// first). 5 s is a reasonable default for HDD deployments where a single
    /// fuzzy query can occupy a slot for seconds.
    #[arg(long, env = "PLOCATE_SERVER_QUEUE_TIMEOUT_SECS", default_value_t = 5)]
    pub queue_timeout_secs: u64,

    /// Upper bound on the candidate set fed to the nucleo fuzzy ranker.
    /// plocate recalls candidates with multi-pattern AND semantics; this cap
    /// bounds the ranking pass AND the per-path stat fan-out that follows.
    /// On SSD the default 1000 is fine (stat is sub-millisecond). On HDD
    /// where each stat costs 5-20 ms, lowering to 200 cuts fuzzy latency
    /// 5× at the cost of recall on rare multi-token queries.
    #[arg(
        long,
        env = "PLOCATE_SERVER_FUZZY_CANDIDATE_CAP",
        default_value_t = 1000
    )]
    pub fuzzy_candidate_cap: usize,

    /// Whether to clear stat_cache when a reindex completes. Default true
    /// keeps results strictly consistent with the new index. On HDD, clearing
    /// means every post-reindex query pays the full stat waterfall (100-1000
    /// cold stats × HDD latency). Set to false to keep the cache warm across
    /// reindexes — at the cost of briefly reporting deleted directories with
    /// a trailing slash until natural LRU eviction catches up.
    #[arg(
        long,
        env = "PLOCATE_SERVER_INVALIDATE_STAT_CACHE_ON_REINDEX",
        default_value_t = true
    )]
    pub invalidate_stat_cache_on_reindex: bool,

    /// Per-reindex timeout (seconds). An updatedb run exceeding this is killed.
    /// Generous by default to accommodate very large trees (10M+ files).
    #[arg(
        long,
        env = "PLOCATE_SERVER_UPDATEDB_TIMEOUT_SECS",
        default_value_t = 3600
    )]
    pub updatedb_timeout_secs: u64,

    /// Base URL of an external file-browsing service (dufs / caddy file_server /
    /// nginx autoindex ...) serving the same tree as `--base-path`. When set,
    /// clients can build a browse link per search result by appending its
    /// relative path. Optional; omitted = no browse links.
    #[arg(long, env = "PLOCATE_SERVER_FILE_SERVER_URL")]
    pub file_server_url: Option<String>,

    /// Contact email surfaced in the web UI for bug reports and feedback.
    /// When unset, the feedback entry is hidden entirely.
    #[arg(long, env = "PLOCATE_SERVER_FEEDBACK_EMAIL")]
    pub feedback_email: Option<String>,

    /// Skill/MCP instance name surfaced in the web UI install dialog and used
    /// as the default name in generated `opencode mcp add` / `codex mcp add`
    /// commands. Must match `^[a-z0-9]+(-[a-z0-9]+)*$` (1-64 chars, lowercase
    /// alphanum + single hyphens). Defaults to "plocate".
    #[arg(long, env = "PLOCATE_SERVER_INSTANCE_NAME", default_value = "plocate")]
    pub instance_name: String,
}

impl Config {
    /// Resolved database path, defaulting next to a cache dir.
    pub fn resolved_db_path(&self) -> PathBuf {
        self.db_path.clone().unwrap_or_else(|| {
            let base = std::env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home_dir().join(".local").join("share"));
            base.join("plocate-server").join("files.db")
        })
    }
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
