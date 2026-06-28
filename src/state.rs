use std::cmp::Reverse;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use tokio::process::Command;
use tokio::sync::Semaphore;

use moka::sync::Cache;

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

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
    pub file_server_url: Arc<Option<String>>,
    pub feedback_email: Arc<Option<String>>,
    /// Skill/MCP instance name surfaced via /api/health and the web UI.
    pub instance_name: Arc<String>,
    /// Per-path `is_dir` cache (stat results). Keyed by absolute path; values
    /// are valid for the current reindex window — cleared on reindex completion.
    /// `moka::sync::Cache` is `Clone` and shares its store internally, so no
    /// `Arc` wrapper is needed.
    stat_cache: Cache<String, bool>,
    reindexing: Arc<AtomicBool>,
    last_run: Arc<Mutex<Option<ReindexRecord>>>,
    search_concurrency: Arc<Semaphore>,
    search_timeout: Duration,
    /// How long a request may wait for a concurrency slot before 503.
    /// Distinct from `search_timeout` (which bounds the plocate run itself).
    queue_timeout: Duration,
    updatedb_timeout: Duration,
}

/// Upper bound on the stat cache size. Each entry is ~100 B, so this caps
/// memory near 10 MB. Normal use stays far below (only searched paths enter,
/// and the cache is cleared on every reindex).
const STAT_CACHE_CAPACITY: u64 = 100_000;

/// Upper bound on the candidate set fed to the nucleo fuzzy ranker. plocate
/// recalls candidates with multi-pattern AND semantics; this cap keeps the
/// ranking pass (and the per-path stat fan-out) bounded. The final response
/// is still limited by the client's `limit`.
const FUZZY_CANDIDATE_CAP: usize = 1000;

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
        crate::limits::validate_skill_name(&cfg.instance_name)?;
        Ok(Self {
            base_path: Arc::new(base_path),
            db_path: Arc::new(db_path),
            plocate_bin: Arc::new(cfg.plocate_bin.clone().into()),
            updatedb_bin: Arc::new(cfg.updatedb_bin.clone().into()),
            max_results: cfg.max_results,
            file_server_url: Arc::new(normalize_file_server_url(cfg.file_server_url.as_deref())),
            feedback_email: Arc::new(normalize_feedback_email(cfg.feedback_email.as_deref())),
            instance_name: Arc::new(cfg.instance_name.clone()),
            stat_cache: Cache::new(STAT_CACHE_CAPACITY),
            reindexing: Arc::new(AtomicBool::new(false)),
            last_run: Arc::new(Mutex::new(None)),
            search_concurrency: Arc::new(Semaphore::new(cfg.max_concurrent_searches.max(1))),
            search_timeout: Duration::from_secs(cfg.search_timeout_secs),
            queue_timeout: Duration::from_secs(cfg.queue_timeout_secs),
            updatedb_timeout: Duration::from_secs(cfg.updatedb_timeout_secs),
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
        // Bound concurrent plocate children; backpressure instead of fork-bomb.
        // The wait is bounded by `queue_timeout` — after that we 503 so
        // clients can retry rather than silent-timeout at their own end.
        let _permit = self.acquire_permit().await?;
        let raw = self
            .run_plocate(query, Some(cap), case_insensitive, basename_only)
            .await?;
        // Stat fan-out is moved to the blocking pool so it does not pin a
        // tokio worker thread. On HDD-class stat latency this is the
        // difference between 8 slots being stuck for seconds vs N slots
        // queueing freely behind a non-blocking async frontier.
        let base_path = self.base_path.clone();
        let stat_cache = self.stat_cache.clone();
        let items = tokio::task::spawn_blocking(move || parse_paths(&raw, &base_path, &stat_cache))
            .await
            .map_err(|e| AppError::Internal(format!("parse_paths join error: {e}")))?;
        let total_returned = items.len();
        let truncated = total_returned == cap && cap > 0;
        let paged: Vec<FileItemDto> = items.into_iter().skip(offset).take(limit).collect();
        Ok(SearchResponse {
            total_matched: total_returned,
            truncated,
            items: paged,
        })
    }

    /// Fuzzy search: recall candidates via plocate with multi-pattern AND
    /// semantics (one pattern per whitespace-separated token), then rank them
    /// with the nucleo fuzzy matcher (fzf-style scoring). Each result carries
    /// a `score`; results are ordered by descending score.
    ///
    /// This gives "search engine"-style multi-keyword matching: a query like
    /// `zookeeper rpm oe1` matches paths containing all three substrings, with
    /// better-matching paths (contiguous, prefix/word-boundary aligned) ranked
    /// first.
    pub async fn search_fuzzy(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
        case_insensitive: bool,
    ) -> Result<SearchResponse> {
        if !self.db_exists() {
            return Ok(SearchResponse::empty());
        }
        // Split into AND tokens; empty query → empty result.
        let tokens: Vec<&str> = query.split_whitespace().collect();
        if tokens.is_empty() {
            return Ok(SearchResponse::empty());
        }
        let _permit = self.acquire_permit().await?;
        let raw = self
            .run_plocate_multi(&tokens, FUZZY_CANDIDATE_CAP, case_insensitive)
            .await?;
        // Stat fan-out for fuzzy is up to FUZZY_CANDIDATE_CAP=1000 paths —
        // the heaviest path through the server. Moving it off the tokio
        // worker is the single most important fix for HDD deployments.
        let base_path = self.base_path.clone();
        let stat_cache = self.stat_cache.clone();
        let items = tokio::task::spawn_blocking(move || parse_paths(&raw, &base_path, &stat_cache))
            .await
            .map_err(|e| AppError::Internal(format!("parse_paths join error: {e}")))?;
        let truncated = items.len() >= FUZZY_CANDIDATE_CAP;
        // Rank with nucleo. match_paths() tunes scoring for path-like input
        // (prefers prefix/segment matches over mid-word matches).
        let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT.match_paths());
        let case = if case_insensitive {
            CaseMatching::Ignore
        } else {
            CaseMatching::Respect
        };
        let pattern = Pattern::parse(query, case, Normalization::Smart);
        let mut scored: Vec<(u32, FileItemDto)> = items
            .into_iter()
            .filter_map(|mut it| {
                let haystack = Utf32Str::Ascii(it.relative_path.as_bytes());
                pattern.score(haystack, &mut matcher).map(|s| {
                    it.score = Some(s);
                    (s, it)
                })
            })
            .collect();
        // Sort by score descending; stable, so ties keep plocate's order.
        scored.sort_by_key(|(s, _)| Reverse(*s));
        let total_matched = scored.len();
        let paged: Vec<FileItemDto> = scored
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|(_, it)| it)
            .collect();
        Ok(SearchResponse {
            total_matched,
            truncated,
            items: paged,
        })
    }

    /// Run plocate with multiple position arguments (AND semantics — a path
    /// must match every pattern). Output is NUL-separated, limited to `limit`.
    async fn run_plocate_multi(
        &self,
        patterns: &[&str],
        limit: usize,
        case_insensitive: bool,
    ) -> Result<Vec<u8>> {
        let mut cmd = Command::new(&*self.plocate_bin);
        cmd.arg("-d").arg(&*self.db_path).arg("-N").arg("-0");
        if case_insensitive {
            cmd.arg("-i");
        }
        cmd.arg("-l").arg(limit.to_string());
        cmd.arg("--");
        for p in patterns {
            cmd.arg(enrich_glob(p));
        }
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let output = run_with_timeout(&mut cmd, self.search_timeout, "plocate").await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.trim().is_empty() {
                return Ok(Vec::new());
            }
            return Err(AppError::Internal(format!(
                "plocate failed ({}): {stderr}",
                output.status
            )));
        }
        Ok(output.stdout)
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
        // plocate's glob anchors a pattern at the start of the full path, so a
        // bare `rust*json` could never match `/home/.../.rustc_info.json`.
        // When the pattern is a glob (contains * ? [) but is not already
        // wildcarded (`*`) or root-anchored (`/`), prepend `*` so it matches
        // anywhere in the path — matching the intuition of "find by name".
        let pattern = enrich_glob(pattern);
        cmd.arg("--").arg(pattern);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let output = run_with_timeout(&mut cmd, self.search_timeout, "plocate").await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // plocate exits 1 both for "no matches" and for genuine errors
            // (see plocate(1) EXIT STATUS). Disambiguate by stderr: an empty
            // stderr means no matches, so return an empty result rather than
            // surfacing a spurious error. Any real error (missing db, bad
            // flag, permission, ...) writes to stderr and is propagated.
            if stderr.trim().is_empty() {
                return Ok(Vec::new());
            }
            return Err(AppError::Internal(format!(
                "plocate failed ({}): {stderr}",
                output.status
            )));
        }
        Ok(output.stdout)
    }

    /// Wait for a concurrency slot, bounded by `queue_timeout`. Returns 503
    /// (QueueTimeout) on expiry so clients can retry / fail fast instead of
    /// silently timing out on their end while waiting for a slot.
    async fn acquire_permit(&self) -> Result<tokio::sync::SemaphorePermit<'_>> {
        match tokio::time::timeout(self.queue_timeout, self.search_concurrency.acquire()).await {
            Ok(Ok(p)) => Ok(p),
            Ok(Err(_)) => Err(AppError::Internal("search concurrency semaphore closed".into())),
            Err(_) => Err(AppError::QueueTimeout(format!(
                "no concurrency slot within {}s (saturated at {})",
                self.queue_timeout.as_secs(),
                self.search_concurrency.available_permits()
            ))),
        }
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
            // The index reflects the filesystem as of now, so stat results from
            // the previous window are stale — drop them.
            state.stat_cache.invalidate_all();
            state.reindexing.store(false, Ordering::Release);
        });
        ReindexOutcome::Started
    }
}

/// plocate treats a pattern as a glob when it contains any of `*`, `?`, `[`.
/// Its glob is matched against the full path and anchored at the start, so
/// `rust*json` cannot match `/home/.../.rustc_info.json`. To make name-like
/// globs behave as "match anywhere in the path", prepend a leading `*` unless
/// the pattern already starts with `*` (already wildcarded) or `/`
/// (explicit root anchor, e.g. `/etc/*.conf`).
fn enrich_glob(pattern: &str) -> String {
    const GLOB_META: &[char] = &['*', '?', '['];
    if pattern.chars().next().is_some_and(|c| c == '*' || c == '/') {
        return pattern.to_string();
    }
    if !pattern.contains(GLOB_META) {
        return pattern.to_string();
    }
    format!("*{pattern}")
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
    let output = run_with_timeout(&mut cmd, state.updatedb_timeout, "updatedb").await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!(
            "updatedb failed ({}): {stderr}",
            output.status
        )));
    }
    Ok(())
}

/// Spawn a command, wait for it with a timeout, and kill it if it exceeds the
/// deadline. Returns the captured output on success, or `Timeout` on expiry.
async fn run_with_timeout(
    cmd: &mut Command,
    timeout: Duration,
    label: &str,
) -> Result<std::process::Output> {
    use tokio::io::AsyncReadExt;
    let mut child = cmd.spawn().map_err(|e| {
        AppError::Internal(format!("failed to run {label}: {e} (is it installed?)"))
    })?;
    // Take the pipes up front so we can drain them concurrently with wait(),
    // avoiding a deadlock when output exceeds the pipe buffer.
    let mut out = child.stdout.take();
    let mut err = child.stderr.take();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let wait = child.wait();
    let drain_out = async {
        if let Some(s) = out.as_mut() {
            let _ = s.read_to_end(&mut stdout).await;
        }
    };
    let drain_err = async {
        if let Some(s) = err.as_mut() {
            let _ = s.read_to_end(&mut stderr).await;
        }
    };

    match tokio::time::timeout(timeout, async { tokio::join!(wait, drain_out, drain_err) }).await {
        Ok((status, _, _)) => Ok(std::process::Output {
            status: status.map_err(|e| AppError::Internal(format!("{label}: {e}")))?,
            stdout,
            stderr,
        }),
        Err(_) => {
            // Timed out — kill and reap the child to avoid leaking processes.
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(AppError::Timeout(format!(
                "{label} exceeded {}s",
                timeout.as_secs()
            )))
        }
    }
}

/// Parse NUL-separated plocate output into DTO items.
fn parse_paths(raw: &[u8], base_path: &Path, stat_cache: &Cache<String, bool>) -> Vec<FileItemDto> {
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(|&b| b == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| {
            // plocate output is UTF-8 bytes; filesystems may contain non-UTF-8,
            // lossy-convert those rare cases.
            let abs = String::from_utf8_lossy(chunk).into_owned();
            build_item(&abs, base_path, stat_cache)
        })
        .collect()
}

fn build_item(abs: &str, base_path: &Path, stat_cache: &Cache<String, bool>) -> FileItemDto {
    // plocate does not tag directories in its output (no trailing slash, no
    // type field), so directory-ness is determined by stat at query time and
    // memoized in `stat_cache` for the duration of the current reindex window.
    let is_dir = is_dir_cached(stat_cache, abs);
    let abs_trimmed = abs
        .trim_end_matches('/')
        .trim_end_matches(std::path::MAIN_SEPARATOR);
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
        score: None,
    }
}

/// Look up whether `abs` is a directory, using the shared stat cache. On a
/// cache miss, `symlink_metadata` is called (without following symlinks, so
/// symlinks — even to directories — are reported as files) and the result is
/// stored. Stat failures (deleted/unreadable) default to `false`.
fn is_dir_cached(cache: &Cache<String, bool>, abs: &str) -> bool {
    if let Some(v) = cache.get(abs) {
        return v;
    }
    let is_dir = std::fs::symlink_metadata(abs)
        .map(|m| m.is_dir())
        .unwrap_or(false);
    cache.insert(abs.to_owned(), is_dir);
    is_dir
}

/// Validate an externally-configured file-server URL. Accepts http/https only;
/// trims trailing slashes so callers can safely append `/<relative_path>`.
/// Returns None (with a warning) on invalid input rather than failing startup.
fn normalize_file_server_url(raw: Option<&str>) -> Option<String> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    if !(raw.starts_with("http://") || raw.starts_with("https://")) {
        tracing::warn!(
            value = raw,
            "file_server_url must be http(s)://...; ignoring"
        );
        return None;
    }
    Some(raw.trim_end_matches('/').to_owned())
}

/// Trim the feedback email; empty input collapses to None so the UI hides the
/// feedback entry entirely.
fn normalize_feedback_email(raw: Option<&str>) -> Option<String> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    Some(raw.to_owned())
}

/// Read RSS (bytes) and thread count from `/proc/self/status` (Linux).
pub fn proc_status() -> io::Result<(u64, u32)> {
    let content = std::fs::read_to_string("/proc/self/status")?;
    let mut rss: Option<u64> = None;
    let mut threads: Option<u32> = None;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            rss = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok());
        } else if let Some(rest) = line.strip_prefix("Threads:") {
            threads = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u32>().ok());
        }
        if rss.is_some() && threads.is_some() {
            break;
        }
    }
    Ok((rss.unwrap_or(0).saturating_mul(1024), threads.unwrap_or(0)))
}

#[cfg(test)]
mod tests {
    use super::enrich_glob;
    use nucleo_matcher::Matcher;
    use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
    use nucleo_matcher::{Config, Utf32Str};
    use std::cmp::Reverse;

    #[test]
    fn enrich_glob_prepends_star_for_name_glob() {
        assert_eq!(enrich_glob("rust*json"), "*rust*json");
        assert_eq!(enrich_glob("src/*.rs"), "*src/*.rs");
        assert_eq!(enrich_glob("[Rr]eadme*"), "*[Rr]eadme*");
        assert_eq!(enrich_glob("?.rs"), "*?.rs");
    }

    #[test]
    fn enrich_glob_leaves_already_wildcarded_untouched() {
        assert_eq!(enrich_glob("*.rs"), "*.rs");
        assert_eq!(enrich_glob("**/2024/*.log"), "**/2024/*.log");
        assert_eq!(enrich_glob("*rust*json"), "*rust*json");
    }

    #[test]
    fn enrich_glob_leaves_root_anchored_untouched() {
        assert_eq!(enrich_glob("/etc/*.conf"), "/etc/*.conf");
        assert_eq!(enrich_glob("/rust*json"), "/rust*json");
    }

    #[test]
    fn enrich_glob_leaves_plain_substring_untouched() {
        // No glob metacharacters → plocate treats it as a substring; must not
        // add a leading `*` (that would turn it into a glob and change
        // semantics).
        assert_eq!(enrich_glob("Cargo.toml"), "Cargo.toml");
        assert_eq!(enrich_glob("config json"), "config json");
        assert_eq!(enrich_glob(""), "");
        assert_eq!(enrich_glob("*"), "*");
    }

    /// Smoke-test the fuzzy ranking used by `search_fuzzy`: a multi-token
    /// query must match paths containing all tokens, and paths with tighter
    /// (prefix/contiguous) alignment must outrank scattered matches.
    #[test]
    fn fuzzy_ranks_multi_token_query() {
        let paths = [
            "zookeeper/rpm/oe1/release.rpm", // all three, aligned
            "oe1/zookeeper/build.rpm",       // all three, reordered
            "zookeeper/rpm/other.tar",       // missing oe1
            "docs/zookeeper.md",             // only zookeeper
        ];
        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
        let pattern = Pattern::parse(
            "zookeeper rpm oe1",
            CaseMatching::Ignore,
            Normalization::Smart,
        );
        let mut scored: Vec<(Option<u32>, &str)> = paths
            .iter()
            .map(|p| {
                (
                    pattern.score(Utf32Str::Ascii(p.as_bytes()), &mut matcher),
                    *p,
                )
            })
            .collect();
        scored.sort_by_key(|(s, _)| Reverse(*s));
        // The two paths containing all three tokens rank above the partials.
        assert_eq!(scored[0].1, "zookeeper/rpm/oe1/release.rpm");
        assert_eq!(scored[1].1, "oe1/zookeeper/build.rpm");
        // Partials do not match (score is None) and sink to the bottom.
        assert!(scored[2].0.is_none());
    }
}
