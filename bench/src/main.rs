//! Load testing harness for plocate-server.
//!
//! Built on `rlt` (https://crates.io/crates/rlt), which provides constant-RPS
//! and constant-concurrency load with correct coordinated-omission-free latency
//! measurement — important here because each `/api/search` request forks a
//! plocate child process with a heavy latency tail.
//!
//! The binary reuses rlt's own CLI options (`--rate`, `--concurrency`,
//! `--duration`, `--warmup`, `--baseline`, `--save-baseline`, ...) so a single
//! binary covers both the constant-RPS and constant-concurrency regimes:
//!
//!     # baseline: constant-RPS latency distribution
//!     cargo run -p bench --release -- --rate 100 --duration 5m
//!
//!     # saturate: fixed-concurrency, push past --max-concurrent-searches
//!     cargo run -p bench --release -- --concurrency 64 --duration 2m
//!
//! Both commands sample queries round-robin from `--queries` and target the
//! endpoint selected by `--mode` (substring / glob / fuzzy).

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::{Parser, ValueEnum};
use reqwest::{Client, Url};
use rlt::{
    IterInfo, IterReport, Status,
    cli::{BenchCli, run},
};
use tokio::time::Instant;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Mode {
    /// `/api/search?q=` — substring, or glob if the query contains * ? [ ].
    Substring,
    /// `/api/glob?pattern=` — explicit glob.
    Glob,
    /// `/api/fuzzy?q=` — multi-keyword ranked fuzzy match.
    Fuzzy,
}

impl Mode {
    fn path(self) -> &'static str {
        match self {
            Mode::Substring => "/api/search",
            Mode::Glob => "/api/glob",
            Mode::Fuzzy => "/api/fuzzy",
        }
    }
    fn param(self) -> &'static str {
        match self {
            Mode::Substring => "q",
            Mode::Glob => "pattern",
            Mode::Fuzzy => "q",
        }
    }
}

/// Top-level CLI: rlt's `BenchCli` flattened together with our scenario knobs.
#[derive(Parser, Clone)]
#[command(name = "bench", about = "Load tester for plocate-server")]
struct Opts {
    /// Base URL of the running plocate-server.
    #[clap(long, default_value = "http://127.0.0.1:8787")]
    url: Url,

    /// Query mode — selects the endpoint.
    #[clap(long, value_enum, default_value_t = Mode::Substring)]
    mode: Mode,

    /// Path to queries file (one query per line). Lines starting with `#` and
    /// blank lines are ignored.
    #[clap(long, default_value = "bench/data/queries.txt")]
    queries: PathBuf,

    /// Connection pool size per host. Tune up to avoid TIME_WAIT exhaustion at
    /// a few hundred RPS; the reqwest default (1) is too low for load testing.
    #[clap(long, default_value_t = 64)]
    pool_idle: usize,

    #[command(flatten)]
    bench: BenchCli,
}

#[derive(Clone)]
struct SearchBench {
    base_url: Url,
    mode: Mode,
    /// Queries are loaded once at startup and shared (immutable) across workers
    /// — workers index in by `IterInfo::iter_no % len` so the load is
    /// reproducible and RNG-free.
    queries: Arc<Vec<String>>,
    pool_idle: usize,
}

#[async_trait]
impl rlt::BenchSuite for SearchBench {
    type WorkerState = Client;

    async fn state(&self, _worker_id: u32) -> Result<Self::WorkerState> {
        Ok(Client::builder()
            .pool_max_idle_per_host(self.pool_idle)
            .pool_idle_timeout(Some(std::time::Duration::from_secs(30)))
            .build()
            .context("building reqwest client")?)
    }

    async fn bench(
        &mut self,
        client: &mut Self::WorkerState,
        info: &IterInfo,
    ) -> Result<IterReport> {
        // queries are shared; workers index by global iteration count so the
        // load is reproducible and RNG-free.
        let q = &self.queries[info.runner_seq as usize % self.queries.len()];

        let mut url = self.base_url.clone();
        url.set_path(self.mode.path());
        url.set_query(Some(&format!("{}={}", self.mode.param(), q)));

        let t = Instant::now();
        let resp = client.get(url).send().await?;
        let status: Status = resp.status().into();
        let bytes = resp.bytes().await?.len() as u64;
        Ok(IterReport {
            duration: t.elapsed(),
            status,
            bytes,
            items: 1,
        })
    }
}

fn load_queries(path: &PathBuf) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading queries file: {}", path.display()))?;
    let queries: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect();
    if queries.is_empty() {
        anyhow::bail!("queries file {} has no usable lines", path.display());
    }
    Ok(queries)
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();

    let queries = load_queries(&opts.queries)?;
    eprintln!(
        "loaded {} queries from {} (mode: {:?})",
        queries.len(),
        opts.queries.display(),
        opts.mode
    );

    let suite = SearchBench {
        base_url: opts.url.clone(),
        mode: opts.mode,
        queries: Arc::new(queries),
        pool_idle: opts.pool_idle,
    };

    run(opts.bench, suite).await
}
