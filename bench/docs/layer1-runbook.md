# Layer 1 — HDD simulation runbook

Goal: reproduce the production condition (10M-file index on slow HDD) on a
fast dev machine, and **measure** the resulting latency / degradation so
the P0/P1 fixes in Layer 3 land on data rather than guesswork.

The pathological case we're chasing is the per-result `std::fs::symlink_metadata`
call inside `state.rs::is_dir_cached()`. On SSD it's microseconds; on HDD
it's 5-20 ms per path — and a `limit=100` search stats 100 paths synchronously
inside an async task, blocking the tokio worker.

## Why this harness shape

- **cgroup v2 io.max is a no-op on multi-queue NVMe** — the kernel only wires
  up `io.cost.*` on this device class, not the throttle interface. Verified
  by direct write: `IOReadBandwidthMax=... 1M` leaves a 5.8 GB/s dd untouched.
- **LD_PRELOAD stat shim** is the surgical answer: it intercepts every
  stat-family syscall in the server process and injects a configurable
  sleep. `cargo run --release` produces a dynamically-linked binary, so the
  shim takes effect with no kernel/root/mount setup.
- This deliberately simulates **stat latency only** — the one pathology we
  actually need to reproduce. plocate's own index read stays fast; we're
  isolating the server-side stat cost.

## Prerequisites

```bash
# one-time: build server + bench, generate dataset, compile shim
task build          # plocate-server release
cargo build --release -p bench
task bench-data -- 1m        # ~10 minutes, ~90k unique paths, 25 MB index
task bench-build-slowio       # bench/bin/slowio.so

# plocate + updatedb on PATH (provided by the plocate package)
```

The 1M dataset is the smallest size where plocate query cost becomes
non-trivial (10M takes too long to generate; 100k is too fast to show
the problem).

## Test matrix

Three disk regimes × four query scenarios = 12 runs. Each run is 30 s.

|              | substring (S1)      | fuzzy (S2)             | repeat cache A/B (S3) | wide glob (S4) |
|--------------|---------------------|------------------------|-----------------------|----------------|
| **A: SSD**   | `bench-serve`       |                        |                       |                |
| **B: HDD 5ms** | `bench-serve-hdd` SLOWIO_STAT_US=5000  ||  ||
| **C: HDD 15ms** | `bench-serve-hdd` SLOWIO_STAT_US=15000 ||  ||

SLOWIO_STAT_US picks: 5 ms ≈ 7200 rpm HDD average stat, 15 ms ≈ slow HDD
under load or cold cache.

Query scenarios:

- **S1 substring** — `queries.txt` (the curated mix), `--mode substring`,
  `--rate 50 --duration 30s`. Models the common case.
- **S2 fuzzy** — `queries-fuzzy.txt`, `--mode fuzzy`, `--rate 5` (fuzzy is
  expensive — don't push RPS or you saturate without learning anything).
  This is the heaviest path; expect the worst degradation here.
- **S3 repeat cache** — `queries-repeat.txt`, `--mode substring`, run the
  same `--rate 50 --duration 30s` **twice back-to-back** against the same
  server instance. Compare round 1 (cold) vs round 2 (warm). The delta
  isolates the stat cost.
- **S4 wide glob** — `queries-wide.txt`, `--mode glob`, `--rate 10`. Wide
  patterns force plocate to traverse every match internally even with
  `-l 100`.

## Running a scenario

Two terminals.

Terminal 1 — start the server under test:

```bash
# A: SSD baseline
task bench-serve -- --bind 127.0.0.1:8787 --max-concurrent-searches 8 --search-timeout-secs 10

# B: HDD sim, 5 ms per stat
SLOWIO_STAT_US=5000 task bench-serve-hdd -- --max-concurrent-searches 8 --search-timeout-secs 10

# C: HDD sim, 15 ms per stat
SLOWIO_STAT_US=15000 task bench-serve-hdd -- --max-concurrent-searches 8 --search-timeout-secs 10
```

Terminal 2 — start the monitor, then drive the load:

```bash
mkdir -p bench/results/$(date +%Y%m%d-%H%M%S)
RESULTS=bench/results/$(date +%Y%m%d-%H%M%S)

# background sampler — RSS / threads / plocate_count
bench/bin/monitor.sh http://127.0.0.1:8787 1 > $RESULTS/s1-substring-A.monitor.ndjson &
MON=$!

# S1 substring
task bench-baseline -- --rate 50 --duration 30s --output json \
  > $RESULTS/s1-substring-A.json

# S2 fuzzy
task bench-baseline -- --mode fuzzy --queries bench/data/queries-fuzzy.txt \
  --rate 5 --duration 30s --output json > $RESULTS/s2-fuzzy-A.json

# S3 repeat (round 1, cold)
task bench-baseline -- --queries bench/data/queries-repeat.txt \
  --rate 50 --duration 30s --output json > $RESULTS/s3-repeat-A-r1.json
# S3 repeat (round 2, warm — same server instance, do NOT restart)
task bench-baseline -- --queries bench/data/queries-repeat.txt \
  --rate 50 --duration 30s --output json > $RESULTS/s3-repeat-A-r2.json

# S4 wide glob
task bench-baseline -- --mode glob --queries bench/data/queries-wide.txt \
  --rate 10 --duration 30s --output json > $RESULTS/s4-wide-A.json

kill $MON
```

Then restart the server with the next disk regime (B, then C) and re-run.
Restart the server between regimes; **do not** restart between S3 round 1
and round 2 (that's the whole point of S3).

## Extracting the numbers

```bash
extract() {
  python3 -c "
import json, sys
d = json.load(open(sys.argv[1]))
l = d['latency']['percentiles']; s = d['summary']
print(f\"{sys.argv[2]:20} iters={s['iters']['total']:6} rps={s['iters']['rate']:6.1f}  \"\
f\"p50={l['p50']*1000:7.2f}ms p95={l['p95']*1000:7.2f}ms p99={l['p99']*1000:7.2f}ms  \"\
f\"p99.9={l['p99.9']*1000:8.2f}ms  succ={s['success_ratio']:.4f}\")
" "$1" "$2"
}

for f in bench/results/*/*.json; do
  extract "$f" "$(basename "$f" .json)"
done
```

Monitor aggregation:

```bash
tail -1 bench/results/*/s2-fuzzy-C.monitor.ndjson   # peak RSS, peak plocate_count
# peak plocate_count over the whole run:
python3 -c "
import json, sys
mx = 0
for line in open(sys.argv[1]):
    try: mx = max(mx, json.loads(line).get('plocate_count', 0))
    except: pass
print('peak plocate_count:', mx)
" bench/results/*/s2-fuzzy-C.monitor.ndjson
```

## What to look for

| Signal                                    | Healthy                              | Concerning                              |
|-------------------------------------------|--------------------------------------|-----------------------------------------|
| S1 p99 (SSD)                              | <20 ms                               | >50 ms — plocate itself is slow         |
| S1 p99 (HDD 5ms)                          | ~500 ms (100 stat × 5 ms)            | >>600 ms — queueing on top of stat      |
| S2 p99 (HDD 5ms)                          | 1–5 s (up to 1000 stat × 5 ms)       | >10 s or timeouts — fuzzy saturated     |
| S3 round-2 p99                            | approaches SSD baseline              | stays close to round 1 — cache broken   |
| S3 (round1 − round2) p99                  | stat cost only                       |                                        |
| `plocate_count` peak (HDD sim, fuzzy)     | ≤ `--max-concurrent-searches`        | pinned at max — semaphore saturated     |
| `success_ratio`                           | 1.000                                | <0.99 — timeouts / 5xx                  |
| RSS growth over run                       | flat (cache bounded)                 | unbounded growth — leak                 |

## Filling in the findings

After each session, drop a `findings.md` in the result directory:

```markdown
# Layer 1 findings — YYYY-MM-DD

Index: 1M (901,158 unique paths, 25 MB db)
Server: --max-concurrent-searches=8 --search-timeout-secs=10

## S1 substring p99

| Disk regime | p50 | p95 | p99 | p99.9 | success |
|-------------|-----|-----|-----|-------|---------|
| A: SSD      |     |     |     |       |         |
| B: HDD 5ms  |     |     |     |       |         |
| C: HDD 15ms |     |     |     |       |         |

## S2 fuzzy p99 (heaviest path)
... same table ...

## S3 cache effect
| Disk | round 1 p99 | round 2 p99 | delta (stat cost) |
...

## S4 wide glob
...

## Verdict
- stat blocking confirmed as P0? Y/N
- recommended --max-concurrent-searches for production: ___
- recommended --search-timeout-secs for production: ___
- next step (Layer 3 priorities): ___
```

## Caveats

- slowio.so **only affects the server process**, not plocate children. That's
  intentional — we want to isolate server-side stat cost. If you also want
  plocate's index reads to be slow, you need a real HDD or fuse delay (out
  of scope for Layer 1).
- Numbers are **relative**, not absolute production predictors. A 100×
  difference between SSD and HDD-sim is the signal; the exact p99 on real
  hardware will differ.
- The `stat_cache` is process-wide and survives across requests within one
  server instance — that's why S3 round 2 needs the **same** process. A
  server restart resets the cache.
