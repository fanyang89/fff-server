use std::path::PathBuf;

use clap::Parser;

/// RESTful filename-search API server backed by a plocate database.
///
/// Builds and queries a dedicated plocate index for a single directory tree,
/// refreshed periodically by `updatedb`. The index lives on disk, so a process
/// restart never rescans.
#[derive(Debug, Clone, Parser)]
#[command(name = "fff-server", version, about)]
pub struct Config {
    /// Root directory to index.
    #[arg(long, env = "FFF_SERVER_BASE_PATH")]
    pub base_path: PathBuf,

    /// Bind address for the HTTP server.
    #[arg(long, env = "FFF_SERVER_BIND", default_value = "127.0.0.1:8787")]
    pub bind: String,

    /// Path to the plocate database (created/refreshed by updatedb).
    #[arg(long, env = "FFF_SERVER_DB_PATH")]
    pub db_path: Option<PathBuf>,

    /// Override the `plocate` binary path.
    #[arg(long, env = "FFF_SERVER_PLOCATE_BIN", default_value = "plocate")]
    pub plocate_bin: String,

    /// Override the `updatedb` binary path.
    #[arg(long, env = "FFF_SERVER_UPDATEDB_BIN", default_value = "updatedb")]
    pub updatedb_bin: String,

    /// Seconds between automatic `updatedb` runs. 0 disables the interval.
    #[arg(
        long,
        env = "FFF_SERVER_REINDEX_INTERVAL_SECS",
        default_value_t = 21600
    )]
    pub reindex_interval_secs: u64,

    /// Maximum results returned by a single search/glob call.
    #[arg(long, env = "FFF_SERVER_MAX_RESULTS", default_value_t = 100)]
    pub max_results: usize,
}

impl Config {
    /// Resolved database path, defaulting next to a cache dir.
    pub fn resolved_db_path(&self) -> PathBuf {
        self.db_path.clone().unwrap_or_else(|| {
            let base = std::env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home_dir().join(".local").join("share"));
            base.join("fff-server").join("files.db")
        })
    }
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
