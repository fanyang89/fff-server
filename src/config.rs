use std::path::PathBuf;

use clap::Parser;

/// RESTful API server built on top of the fff file-search engine.
#[derive(Debug, Clone, Parser)]
#[command(name = "fff-server", version, about)]
pub struct Config {
    /// Directory to index and serve searches over.
    #[arg(long, env = "FFF_SERVER_BASE_PATH")]
    pub base_path: PathBuf,

    /// Bind address for the HTTP server.
    #[arg(long, env = "FFF_SERVER_BIND", default_value = "127.0.0.1:8787")]
    pub bind: String,

    /// Directory used for frecency / query-history databases.
    #[arg(long, env = "FFF_SERVER_DB_DIR")]
    pub db_dir: Option<PathBuf>,

    /// Enable AI mode (definition classification, enhanced scoring).
    #[arg(long, env = "FFF_SERVER_AI_MODE", default_value_t = true)]
    pub ai_mode: bool,

    /// Watch the filesystem for live index updates.
    #[arg(long, env = "FFF_SERVER_WATCH", default_value_t = true)]
    pub watch: bool,

    /// Index file contents for content-aware filtering.
    #[arg(long, env = "FFF_SERVER_CONTENT_INDEXING", default_value_t = false)]
    pub content_indexing: bool,

    /// Memory-map caches for top-frecency files after the initial scan.
    #[arg(long, env = "FFF_SERVER_MMAP_CACHE", default_value_t = false)]
    pub mmap_cache: bool,

    /// Seconds to wait for the initial scan to finish before serving.
    #[arg(long, env = "FFF_SERVER_WAIT_SCAN_SECS", default_value_t = 10)]
    pub wait_scan_secs: u64,

    /// Maximum number of results returned by a single search/glob call.
    #[arg(long, env = "FFF_SERVER_MAX_RESULTS", default_value_t = 100)]
    pub max_results: usize,
}

impl Config {
    /// Resolved database directory, defaulting to a cache dir under XDG.
    pub fn resolved_db_dir(&self) -> PathBuf {
        self.db_dir.clone().unwrap_or_else(|| {
            let base = std::env::var("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| dirs_like_home().join(".cache"));
            base.join("fff-server")
        })
    }
}

fn dirs_like_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
