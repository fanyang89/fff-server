//! Sliding-window "trending queries" tracker.
//!
//! Counts how often each (normalized) search query is issued within a rolling
//! window, so the UI can surface a "hot searches" board — useful right after a
//! popular ISO / RPM drops and many users hunt for the same paths.
//!
//! Implementation: fixed-bucket rotation. The window is divided into N equal
//! buckets (e.g. 24 × 1h for a 24h window). Every search increments the count
//! for the *current* bucket under a normalized key. A background tokio task
//! advances the current-bucket pointer every `bucket_secs` and clears the
//! bucket it lands on (which by then is one full window old). Reading the
//! leaderboard sums all N buckets per key.
//!
//! Properties:
//!   - Write cost: one HashMap lookup + u64 bump (mutex held briefly).
//!   - Read cost: O(N buckets × distinct terms) aggregation.
//!   - Sliding precision: each count is at most one bucket older than the
//!     nominal window; a fresh "drop" becomes visible immediately in the
//!     current bucket, and stale counts vanish one bucket after the window.
//!   - Pure in-memory; a restart zeroes everything (by design — matches the
//!     server's stateless ethos).

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Hard upper bound on `?limit` to bound read-side work.
pub const MAX_TOP_N: usize = 100;

pub struct TrendingTracker {
    buckets: Vec<Mutex<HashMap<String, u64>>>,
    current: AtomicUsize,
    num_buckets: usize,
    bucket_secs: u64,
    window_secs: u64,
    min_len: usize,
    default_top_n: usize,
}

impl TrendingTracker {
    /// Construct a tracker and spawn its bucket-rotation task.
    ///
    /// `window_secs` / `bucket_secs` are clamped to >= 1; the bucket count is
    /// `(window / bucket).max(1)`. If `bucket_secs > window_secs`, the whole
    /// window collapses to a single bucket (still correct, just coarser).
    pub fn with_top_n(
        window_secs: u64,
        bucket_secs: u64,
        min_len: usize,
        default_top_n: usize,
    ) -> Arc<Self> {
        let bucket_secs = bucket_secs.max(1);
        let window_secs = window_secs.max(1);
        let num_buckets = ((window_secs / bucket_secs).max(1)) as usize;
        let default_top_n = default_top_n.clamp(1, MAX_TOP_N);
        let me = Arc::new(Self {
            buckets: (0..num_buckets)
                .map(|_| Mutex::new(HashMap::new()))
                .collect(),
            current: AtomicUsize::new(0),
            num_buckets,
            bucket_secs,
            window_secs,
            min_len,
            default_top_n,
        });
        me.spawn_rotator();
        me
    }

    /// Build a tracker for tests without spawning the rotator.
    #[cfg(test)]
    pub fn for_test(min_len: usize) -> Arc<Self> {
        Arc::new(Self {
            buckets: (0..3).map(|_| Mutex::new(HashMap::new())).collect(),
            current: AtomicUsize::new(0),
            num_buckets: 3,
            bucket_secs: 1,
            window_secs: 3,
            min_len,
            default_top_n: 20,
        })
    }

    fn spawn_rotator(self: &Arc<Self>) {
        // Detached task: tokio keeps it alive until the runtime shuts down,
        // which is exactly the process lifetime we want. The task advances
        // the current-bucket pointer every `bucket_secs` and clears the
        // bucket it lands on (which by construction is one full window old).
        let me = Arc::clone(self);
        let period = std::time::Duration::from_secs(me.bucket_secs);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(period);
            // The first tick fires immediately; skip it so we don't rotate
            // at startup before the first bucket has collected anything.
            ticker.tick().await;
            loop {
                ticker.tick().await;
                me.rotate();
            }
        });
    }

    /// Advance to the next bucket and clear it. Order matters: clear first,
    /// then publish the new index. Recorders read `current` then lock the
    /// bucket at that index, so a stale reader can at worst land in the
    /// bucket that just became the oldest (its counts linger one extra cycle
    /// — harmless).
    fn rotate(&self) {
        let next = (self.current.load(Ordering::Acquire) + 1) % self.num_buckets;
        {
            let mut g = self.buckets[next].lock().unwrap();
            g.clear();
        }
        self.current.store(next, Ordering::Release);
    }

    /// Normalize and count a query. No-op for too-short queries.
    ///
    /// Normalization is `trim().to_lowercase()` — collapses cosmetic
    /// whitespace/case differences without erasing meaningful path structure.
    pub fn record(&self, raw: &str) {
        let s = raw.trim();
        if s.chars().count() < self.min_len {
            return;
        }
        let key = s.to_lowercase();
        let idx = self.current.load(Ordering::Acquire);
        let mut g = self.buckets[idx].lock().unwrap();
        *g.entry(key).or_insert(0) += 1;
    }

    /// Aggregate counts across the whole window and return the top-`k` queries
    /// by descending count, ties broken alphabetically.
    pub fn top_k(&self, k: usize) -> Vec<(String, u64)> {
        let k = k.clamp(1, MAX_TOP_N);
        let mut agg: HashMap<String, u64> = HashMap::new();
        for b in &self.buckets {
            for (key, cnt) in b.lock().unwrap().iter() {
                *agg.entry(key.clone()).or_insert(0) += cnt;
            }
        }
        let mut v: Vec<(String, u64)> = agg.into_iter().collect();
        // Sort by count desc, then key asc for stable, deterministic ordering.
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v.truncate(k);
        v
    }

    pub fn window_secs(&self) -> u64 {
        self.window_secs
    }

    pub fn default_top_n(&self) -> usize {
        self.default_top_n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_increments_current_bucket() {
        let t = TrendingTracker::for_test(2);
        t.record("Cargo.toml");
        t.record("cargo.toml"); // case-folded onto the same key
        t.record("Cargo.toml");
        t.record("x"); // below min_len → dropped
        let top = t.top_k(10);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0], ("cargo.toml".to_string(), 3));
    }

    #[test]
    fn rotation_clears_oldest_bucket() {
        let t = TrendingTracker::for_test(2);
        t.record("alpha");
        t.record("alpha");
        // Rotate once: current advances, the bucket we just filled is now the
        // oldest but still in the window.
        t.rotate();
        t.record("beta");
        let top = t.top_k(10);
        let map: HashMap<String, u64> = top.into_iter().collect();
        assert_eq!(map.get("alpha").copied(), Some(2));
        assert_eq!(map.get("beta").copied(), Some(1));

        // Rotate twice more (total 3 = num_buckets): the alpha bucket is now
        // cleared and its counts drop out of the window entirely.
        t.rotate();
        t.rotate();
        let map: HashMap<String, u64> = t.top_k(10).into_iter().collect();
        assert_eq!(map.get("alpha"), None);
    }

    #[test]
    fn top_k_is_sorted_and_truncated() {
        let t = TrendingTracker::for_test(2);
        for _ in 0..5 {
            t.record("busy");
        }
        for _ in 0..3 {
            t.record("mid");
        }
        t.record("rare");
        let top = t.top_k(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "busy");
        assert_eq!(top[0].1, 5);
        assert_eq!(top[1].0, "mid");
        assert_eq!(top[1].1, 3);
    }

    #[test]
    fn trim_normalizes_whitespace() {
        let t = TrendingTracker::for_test(2);
        t.record("  Cargo.toml  ");
        t.record("Cargo.toml");
        let map: HashMap<String, u64> = t.top_k(10).into_iter().collect();
        assert_eq!(map.get("cargo.toml").copied(), Some(2));
    }

    #[test]
    fn min_len_uses_char_count_not_byte_count() {
        let t = TrendingTracker::for_test(2);
        // '长度' is 2 chars / 6 bytes — must count as length 2, not be
        // rejected by an accidental byte-length check.
        t.record("长度");
        let top = t.top_k(10);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].0, "长度");
    }

    #[test]
    fn empty_window_returns_empty_leaderboard() {
        let t = TrendingTracker::for_test(2);
        assert!(t.top_k(10).is_empty());
    }

    /// Construction with degenerate inputs must not panic and must collapse
    /// to a single bucket (still a valid, if coarse, window).
    #[tokio::test]
    async fn new_collapses_degenerate_ratios() {
        let t = TrendingTracker::with_top_n(0, 0, 2, 20);
        assert!(t.window_secs() >= 1);
        t.record("hello");
        assert_eq!(t.top_k(10).len(), 1);
    }
}
