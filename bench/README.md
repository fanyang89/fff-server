# bench — load testing harness for plocate-server

A permanent, reproducible load-testing harness built on [`rlt`](https://crates.io/crates/rlt)
for measuring plocate-server's behavior under load and catching performance
regressions before release.

## Why rlt

- Rust-native, embeddable as a workspace member (not a standalone binary like `oha`)
- Constant-RPS (`--rate`) **and** constant-concurrency (`--concurrency`) modes
- Correctly avoids coordinated omission (uses `governor` token-bucket scheduling
  from an absolute clock — important because each `/api/search` request forks a
  plocate child process with a heavy latency tail)
- Built-in percentile reporting (p50/p90/p95/p99), histogram, status-code
  breakdown, error distribution
- `--baseline` / `--save-baseline` / `--fail-on-regression` for regression gating

## Quick start

```bash
# 1. Generate a synthetic artifact-tree index (10k / 100k / 1m)
task bench-data -- 100k

# 2. Start the server against that index (foreground terminal)
task bench-serve
#   (or with custom flags):
#   task bench-serve -- --max-concurrent-searches 4 --search-timeout-secs 5

# 3. In another terminal, drive load:
task bench-baseline -- --rate 100 --duration 5m
task bench-saturate  -- --concurrency 64 --duration 2m
```

## Two regimes

| Command | rlt mode | Measures |
|---|---|---|
| `bench-baseline` | `--rate N` | Steady-state latency distribution at a target RPS |
| `bench-saturate` | `--concurrency N` | Behavior past `--max-concurrent-searches` — verify graceful degradation (timeouts, not crashes / 5xx) |

A single binary covers both — the regime is chosen by which rlt flag you pass.
The Taskfile wrappers just pin the URL and queries-file defaults for
convenience.

## Query modes

`--mode substring|glob|fuzzy` selects the endpoint:

| Mode | Endpoint | Notes |
|---|---|---|
| `substring` (default) | `/api/search?q=` | Plain substring match |
| `glob` | `/api/glob?pattern=` | Explicit glob |
| `fuzzy` | `/api/fuzzy?q=` | Multi-keyword ranked; triggers a stat waterfall (heaviest path) |

The queries file (`bench/data/queries.txt`) groups queries by intended mode;
the harness sends every line as-is regardless of mode, so pick a corpus that
matches the mode you're exercising.

## Synthetic dataset

`bench/data/generate.sh [10k|100k|1m]` produces:

- `tree/` — empty placeholder files shaped like a Fedora/CentOS/Debian/Ubuntu
  artifact repository (RPMs, ISOs, source tarballs, debuginfo). plocate indexes
  paths only, so file contents are irrelevant.
- `files.db` — the plocate trigram index for `tree/`.
- `queries.txt` — curated query corpus (package basenames, extensions, version
  fragments, glob patterns, fuzzy multi-keyword phrases).

Re-runnable; wipes `tree/` and `files.db` each time. Sizes:

| Size | Files | Tree | Index | Use case |
|---|---|---|---|---|
| `10k`  | ~9k unique | ~0 MB | ~150 KB | quick smoke |
| `100k` | ~90k | ~3 MB | ~2 MB | realistic single-host repo |
| `1m`   | ~900k | ~30 MB | ~25 MB | stress / saturation finding |

## Baseline regression gating

rlt stores named baselines under `target/rlt-baseline/`. Save one, then compare
subsequent runs and fail the process if regression is detected:

```bash
task bench-baseline -- --rate 100 --duration 5m --save-baseline v1
# ... make changes ...
task bench-baseline -- --rate 100 --duration 5m --baseline v1 --fail-on-regression
```

Default regression metrics: iteration rate, latency mean / p90 / p99, success
ratio.

## Output formats

- Default: colored text with histogram (good for terminal)
- `--output json`: machine-readable JSON to stdout
- `--output-file <path>`: write report to a file instead

For batch runs, pipe JSON into `bench/results/<timestamp>/` for later diffing.

## Connection pool tuning

`--pool-idle N` (default 64) controls `reqwest`'s `pool_max_idle_per_host`. The
reqwest default of 1 silently caps throughput at a few hundred RPS via TIME_WAIT
exhaustion; bumping it is required for honest load tests against localhost.

## What this harness does *not* cover

(Reserved for future commits, not in scope for the initial version.)

- cgroup interference measurement (the "designed for shared hosts" SLO)
- Real-time server-side metric sampling (RSS, threads via `/api/stats`)
- Mixed-query workload simulation (weighted distribution across modes)
