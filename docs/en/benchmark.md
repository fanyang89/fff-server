# Benchmark

plocate-server ships an `rlt`-based load-testing harness as a workspace
member. Source: `bench/`. It is coordinated-omission-free (governor
token-bucket pacing) and covers both constant-RPS and constant-concurrency
regimes.

## Run

The `Taskfile.yml` exposes the common cases:

```bash
# Constant RPS — measures latency under steady load
task bench-baseline -- --rate 50 --duration 60

# Constant concurrency — saturates to find throughput ceiling
task bench-saturate -- --concurrency 32 --duration 60
```

Both delegate to `cargo run -p bench --release -- …`. Direct invocation:

```bash
cargo run -p bench --release -- \
    --url http://127.0.0.1:8787 \
    --mode substring \
    --queries bench/data/queries.txt \
    --rate 50 --duration 60
```

## Modes

| `--mode` | Endpoint hit | Query source |
| --- | --- | --- |
| `substring` (default) | `/api/search` | `bench/data/queries.txt` |
| `glob` | `/api/glob` | `bench/data/queries.txt` |
| `fuzzy` | `/api/fuzzy` | `bench/data/queries-fuzzy.txt` |

Queries are round-robin sampled from the corpus (`iter_no % len`, RNG-free)
so runs are reproducible.

## Datasets

`task bench-data -- 10k|100k|1m` generates a synthetic file tree under
`bench/data/tree/` and builds `bench/data/files.db`. Sizes:

- `10k` — quick smoke test
- `100k` — typical dev run
- `1m` — stress / regression gating

## HDD simulation

`bench/bin/slowio.so` is an `LD_PRELOAD` shim that injects configurable sleep
into `stat`-family syscalls (`SLOWIO_STAT_US`):

```bash
task bench-build-slowio                              # one-time
task bench-serve-hdd SLOWIO_STAT_US=5000             # 5 ms ≈ 7200 rpm HDD
task bench-baseline -- --mode fuzzy --rate 0.2 --duration 60
```

See [hdd-tuning](./hdd-tuning.md) for the findings and the full matrix.

## Reference results (`bench/results/20260628-145044-layer1/`)

888,016 unique paths, 8.3 MB db. Constant-RPS, three disk regimes:

| Regime | Mode | RPS | p50 | p99 |
| --- | --- | --- | --- | --- |
| A — SSD | substring | 49.6 | 2.60 ms | 3.84 ms |
| A — SSD | fuzzy | 10.0 | 10.04 ms | 15.41 ms |
| B — HDD 5 ms | substring | 2.0 | 413 ms | 519 ms |
| B — HDD 5 ms | fuzzy | 0.2 | 3666 ms | 5138 ms |
| C — HDD 15 ms | substring | 1.0 | 1223 ms | 1531 ms |
| C — HDD 15 ms | fuzzy | 0.1 | 11290 ms | 15158 ms |

Peak RSS stayed at ~21 MiB across all regimes. Full JSON + monitor time
series are in the results directory.

## Regression gating

`rlt` supports named baselines so CI can fail on regressions:

```bash
cargo run -p bench --release -- --rate 50 --duration 30 --save-baseline main
# ... after a change ...
cargo run -p bench --release -- --rate 50 --duration 30 \
    --baseline main --fail-on-regression
```

## What is being measured

The bench client measures **end-to-end HTTP latency** of a single plocate-server
instance — including the `plocate` child-process spawn, mmap index read, the
`stat` fan-out, and JSON serialization. It is not a microbenchmark of any one
of those layers; it reflects what real clients see.
