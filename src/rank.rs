//! Re-ranking of plocate-recalled fuzzy candidates.
//!
//! `/api/fuzzy` recalls candidates via plocate (multi-token AND substring
//! match), then this module re-ranks them with a basename-centric scorer that
//! blends fuzzy relevance with structural signals:
//!
//! - basename match strength (exact/contains vs prefix vs path-only),
//! - multi-token nucleo score summed against the basename,
//! - path-depth penalty (shorter paths first),
//! - hidden-file penalty,
//! - junk-directory penalty (`node_modules`, `.git`, `build`, ...).
//!
//! ## Scoring model
//!
//! The final score is a `u32` bit-packed as `(tier << WITHIN_BITS) | within`,
//! so sorting by descending score yields a lexicographic "tier first, then
//! within-tier weighted sum" ordering — a hybrid of rigid bucketing (basename
//! match strength dominates) and flexible weighting (fuzzy relevance + signal
//! penalties trade off inside a bucket). The bucket dominates so that a strong
//! basename match is never buried by a slightly better fuzzy score on a weak
//! basename match.
//!
//! Weights are module-level constants, intentionally not exposed via CLI.

use std::cmp::Reverse;

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::dto::FileItemDto;

/// Path segments marking generated/vendored ("junk") subtrees. A candidate
/// descending through any of these is demoted within its tier. Matched
/// case-sensitively against the literal segment, which covers the common
/// layouts without false-positives on legitimate user directories.
const JUNK_SEGMENTS: &[&str] = &[
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "dist",
    "build",
    "target",
    "vendor",
    ".cache",
    "__pycache__",
    ".next",
    ".venv",
    "venv",
];

// Within-tier weights. Tunable constants; not surfaced via CLI by design.
//
// The model is `within = NUCLEO_RANGE * norm_nuc + CLEAN_BASELINE - penalties`,
// clamped to [0, WITHIN_MAX]. `CLEAN_BASELINE` is a positive offset so that
// items with no basename match (norm_nuc == 0) still get differentiated by
// structural penalties rather than clamping to a tie at 0.
/// Band occupied by the normalized ([0.0, 1.0]) nucleo basename score.
const NUCLEO_RANGE: f64 = 800_000.0;
/// Positive offset ensuring structural penalties alone still rank items.
const CLEAN_BASELINE: f64 = 200_000.0;
/// Per-segment penalty for nesting depth.
const W_DEPTH: f64 = 5_000.0;
/// Flat penalty when any path segment is dot-hidden.
const W_HIDDEN: f64 = 50_000.0;
/// Per-junk-segment penalty (graduated, up to `JUNK_TIER_CAP` segments).
const W_JUNK: f64 = 50_000.0;

/// Maximum number of junk segments counted; further junk stops accumulating.
const JUNK_TIER_CAP: u8 = 2;

/// Bits reserved for the within-tier score. The tier occupies the bits above,
/// so any item in a higher tier outranks every item in a lower tier
/// regardless of within-tier values.
const WITHIN_BITS: u32 = 20;
const WITHIN_MAX: u32 = (1u32 << WITHIN_BITS) - 1;

/// Re-rank `items` by descending relevance to `query`, setting each kept
/// item's `score`.
///
/// Items that do not fuzzy-match the whole query against the full path are
/// dropped, preserving the historical filter semantics of `/api/fuzzy` (the
/// legacy single-pass nucleo scorer likewise returned `None` for them). The
/// returned order is stable with respect to the input for tied scores, so
/// plocate's recall order remains the tie-breaker.
pub fn rerank(query: &str, case_insensitive: bool, items: Vec<FileItemDto>) -> Vec<FileItemDto> {
    let tokens: Vec<&str> = query.split_whitespace().collect();
    // The caller guards against empty queries, but stay defensive: with
    // nothing to score, leave the input order untouched.
    if tokens.is_empty() {
        return items;
    }
    let case = if case_insensitive {
        CaseMatching::Ignore
    } else {
        CaseMatching::Respect
    };

    // Gate pattern: whole-query fuzzy match against the full path. nucleo
    // splits the query on whitespace into atoms and requires all of them to
    // match, so this both filters and preserves the legacy "every token must
    // fuzzy-match the path" contract.
    let gate = Pattern::parse(query, case, Normalization::Smart);
    // Per-token patterns scored against the basename. The basename is a single
    // name rather than a path, so the default config is used instead of
    // `match_paths()` (which tunes scoring for slash-separated input).
    let token_patterns: Vec<Pattern> = tokens
        .iter()
        .map(|t| Pattern::parse(t, case, Normalization::Smart))
        .collect();

    struct Feat {
        item: FileItemDto,
        nuc_sum: u32,
        tier: u8,
        depth: u32,
        is_hidden: bool,
        junk_tier: u8,
    }

    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut feats: Vec<Feat> = Vec::with_capacity(items.len());
    let mut max_nuc: u32 = 0;

    for item in items {
        let path_hay = Utf32Str::Ascii(item.relative_path.as_bytes());
        if gate.score(path_hay, &mut matcher).is_none() {
            continue;
        }
        let base_hay = Utf32Str::Ascii(item.name.as_bytes());
        let mut nuc_sum: u32 = 0;
        for p in &token_patterns {
            if let Some(s) = p.score(base_hay, &mut matcher) {
                nuc_sum = nuc_sum.saturating_add(s);
            }
        }
        let (depth, is_hidden, junk_tier) = path_features(&item.relative_path);
        let tier = basename_tier(&item.name, &tokens);
        if nuc_sum > max_nuc {
            max_nuc = nuc_sum;
        }
        feats.push(Feat {
            item,
            nuc_sum,
            tier,
            depth,
            is_hidden,
            junk_tier,
        });
    }

    // Second pass: normalize the basename score by the candidate-set maximum
    // so cross-query scale differences do not distort the within-tier weighting,
    // then blend with the structural penalties.
    let mut scored: Vec<(u32, FileItemDto)> = feats
        .into_iter()
        .map(|f| {
            let norm_nuc = if max_nuc > 0 {
                f.nuc_sum as f64 / max_nuc as f64
            } else {
                0.0
            };
            let mut within: f64 = NUCLEO_RANGE * norm_nuc + CLEAN_BASELINE
                - W_DEPTH * f.depth as f64
                - W_HIDDEN * f.is_hidden as u32 as f64
                - W_JUNK * f.junk_tier as f64;
            if within < 0.0 {
                within = 0.0;
            }
            let within_u32 = (within.round() as u64).min(WITHIN_MAX as u64) as u32;
            let final_score = ((f.tier as u32) << WITHIN_BITS) | within_u32;
            let mut item = f.item;
            item.score = Some(final_score);
            (final_score, item)
        })
        .collect();

    // Stable sort by final score descending: tier dominates, within-tier
    // weighting breaks ties, and equal scores retain plocate's recall order.
    scored.sort_by_key(|(s, _)| Reverse(*s));
    scored.into_iter().map(|(_, it)| it).collect()
}

/// Compute structural features of a relative path.
///
/// Returns `(depth, is_hidden, junk_tier)`:
/// - `depth` — number of non-empty segments (monotonic with nesting).
/// - `is_hidden` — true if any segment (including basename) starts with `.`.
/// - `junk_tier` — count of junk segments, capped at `JUNK_TIER_CAP`.
fn path_features(relative_path: &str) -> (u32, bool, u8) {
    let mut depth: u32 = 0;
    let mut is_hidden = false;
    let mut junk_tier: u8 = 0;
    for seg in relative_path.split('/') {
        if seg.is_empty() {
            continue;
        }
        depth = depth.saturating_add(1);
        if seg.starts_with('.') {
            is_hidden = true;
        }
        if junk_tier < JUNK_TIER_CAP && JUNK_SEGMENTS.contains(&seg) {
            junk_tier = junk_tier.saturating_add(1);
        }
    }
    (depth, is_hidden, junk_tier)
}

/// Classify the basename into a relevance tier.
///
/// - `2` — strong: every query token is a (case-insensitive) substring of the
///   basename. The basename alone would have surfaced this file.
/// - `1` — medium: the basename starts with the first token, but not all
///   tokens are in the basename.
/// - `0` — weak: tokens match only in the surrounding path.
fn basename_tier(basename: &str, tokens: &[&str]) -> u8 {
    let base_lc = basename.to_ascii_lowercase();
    let all_in_base = tokens
        .iter()
        .all(|t| base_lc.contains(&t.to_ascii_lowercase()));
    if all_in_base {
        return 2;
    }
    let first_lc = tokens[0].to_ascii_lowercase();
    if base_lc.starts_with(&first_lc) {
        return 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::rerank;
    use crate::dto::FileItemDto;

    fn item(name: &str, relative_path: &str) -> FileItemDto {
        FileItemDto {
            kind: "file".into(),
            name: name.into(),
            relative_path: relative_path.into(),
            absolute_path: format!("/root/{relative_path}"),
            score: None,
        }
    }

    fn names(out: &[FileItemDto]) -> Vec<&str> {
        out.iter().map(|i| i.name.as_str()).collect()
    }

    /// Every returned item carries a relevance score.
    #[test]
    fn every_kept_item_has_score() {
        let out = rerank("cargo", true, vec![item("cargo.toml", "proj/cargo.toml")]);
        assert_eq!(out.len(), 1);
        assert!(out[0].score.is_some());
    }

    /// Items that do not fuzzy-match the query against the full path are
    /// dropped (legacy `/api/fuzzy` filter semantics).
    #[test]
    fn non_matching_items_dropped() {
        let out = rerank(
            "cargo core",
            true,
            vec![
                item("cargo_core.rs", "proj/cargo_core.rs"),
                item("readme.md", "docs/readme.md"),
            ],
        );
        // readme.md has neither "cargo" nor "core" in its path → gated out.
        assert_eq!(names(&out), ["cargo_core.rs"]);
    }

    /// Tier 2 (basename contains all tokens) outranks tier 1 (prefix only),
    /// which outranks tier 0 (path-only match).
    #[test]
    fn tier_ordering_strong_medium_weak() {
        let out = rerank(
            "cargo core",
            true,
            vec![
                // tier 0: tokens only in path.
                item("helper.rs", "cargo/core/helper.rs"),
                // tier 1: basename prefixed by first token, second token only in path.
                item("cargo_helper.rs", "core/cargo_helper.rs"),
                // tier 2: basename contains both tokens.
                item("cargo_core.rs", "proj/cargo_core.rs"),
            ],
        );
        assert_eq!(
            names(&out),
            ["cargo_core.rs", "cargo_helper.rs", "helper.rs"]
        );
    }

    /// Within a tier, a shallower path outranks a deeper one.
    #[test]
    fn shallower_path_outranks_deeper_within_tier() {
        let out = rerank(
            "cargo core",
            true,
            vec![
                item("z.rs", "cargo/core/deep/nested/z.rs"),
                item("z.rs", "cargo/core/z.rs"),
            ],
        );
        // Both tier 0, identical basename; the shorter path wins.
        assert_eq!(out[0].relative_path, "cargo/core/z.rs");
        assert_eq!(out[1].relative_path, "cargo/core/deep/nested/z.rs");
    }

    /// Within a tier and at equal depth, a junk path is demoted.
    #[test]
    fn junk_directory_demoted_within_tier() {
        let out = rerank(
            "cargo core",
            true,
            vec![
                // Both depth 4, tier 0; the second descends through node_modules.
                item("z.rs", "cargo/node_modules/core/z.rs"),
                item("z.rs", "cargo/aaa/core/z.rs"),
            ],
        );
        assert_eq!(out[0].relative_path, "cargo/aaa/core/z.rs");
        assert_eq!(out[1].relative_path, "cargo/node_modules/core/z.rs");
    }

    /// Within a tier and at equal depth, a hidden path is demoted.
    #[test]
    fn hidden_path_demoted_within_tier() {
        let out = rerank(
            "cargo core",
            true,
            vec![
                item("z.rs", "cargo/.core/aaa/z.rs"),
                item("z.rs", "cargo/bcore/aaa/z.rs"),
            ],
        );
        assert_eq!(out[0].relative_path, "cargo/bcore/aaa/z.rs");
        assert_eq!(out[1].relative_path, "cargo/.core/aaa/z.rs");
    }

    /// Equal scores retain the input (plocate recall) order — stable sort.
    #[test]
    fn ties_keep_input_order() {
        let out = rerank(
            "cargo core",
            true,
            vec![
                item("z.rs", "cargo/core/aaa/z.rs"),
                item("z.rs", "cargo/bbb/core/z.rs"),
            ],
        );
        // Both tier 0, depth 4, no junk/hidden — identical scores. Input order
        // must be preserved (aaa before bbb).
        assert_eq!(out[0].relative_path, "cargo/core/aaa/z.rs");
        assert_eq!(out[1].relative_path, "cargo/bbb/core/z.rs");
    }

    /// `path_features`-derived signals compose: a deep junk path ranks below a
    /// shallow clean path even when both share the same tier.
    #[test]
    fn depth_and_junk_compose() {
        let out = rerank(
            "cargo core",
            true,
            vec![
                item("z.rs", "cargo/node_modules/build/core/z.rs"),
                item("z.rs", "cargo/core/z.rs"),
            ],
        );
        assert_eq!(out[0].relative_path, "cargo/core/z.rs");
    }

    /// A tier-2 (basename exact) result outranks a tier-0 result even when the
    /// tier-2 result sits in a junk directory — tier dominates within-tier
    /// weighting.
    #[test]
    fn tier_dominates_junk_penalty() {
        let out = rerank(
            "cargo core",
            true,
            vec![
                // tier 0, clean, shallow.
                item("helper.rs", "cargo/core/helper.rs"),
                // tier 2, but deep in junk.
                item("cargo_core.rs", "cargo/node_modules/build/cargo_core.rs"),
            ],
        );
        assert_eq!(names(&out), ["cargo_core.rs", "helper.rs"]);
    }

    /// Multi-token: a basename matching all tokens (tier 2) outranks a basename
    /// matching only some (tier 1/0).
    #[test]
    fn multi_token_basename_match_outranks_partial() {
        let out = rerank(
            "zookeeper rpm",
            true,
            vec![
                // tier 0: neither token in basename.
                item("release.tar", "zookeeper/rpm/release.tar"),
                // tier 2: both tokens in basename.
                item("zookeeper.rpm", "build/zookeeper.rpm"),
            ],
        );
        assert_eq!(names(&out), ["zookeeper.rpm", "release.tar"]);
    }

    /// Empty query is a no-op (returns input untouched, scores unset).
    #[test]
    fn empty_query_noop() {
        let items = vec![item("a.rs", "x/a.rs")];
        let out = rerank("", true, items);
        assert_eq!(out.len(), 1);
        assert!(out[0].score.is_none());
    }
}
