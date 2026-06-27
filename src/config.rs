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

    /// Seconds between automatic `updatedb` runs. 0 disables the interval.
    #[arg(
        long,
        env = "PLOCATE_SERVER_REINDEX_INTERVAL_SECS",
        default_value_t = 21600
    )]
    pub reindex_interval_secs: u64,

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

    /// Per-query timeout (seconds). A plocate run exceeding this is killed and
    /// reported as a 504.
    #[arg(long, env = "PLOCATE_SERVER_SEARCH_TIMEOUT_SECS", default_value_t = 10)]
    pub search_timeout_secs: u64,

    /// Per-reindex timeout (seconds). An updatedb run exceeding this is killed.
    /// Generous by default to accommodate very large trees (10M+ files).
    #[arg(
        long,
        env = "PLOCATE_SERVER_UPDATEDB_TIMEOUT_SECS",
        default_value_t = 3600
    )]
    pub updatedb_timeout_secs: u64,
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
