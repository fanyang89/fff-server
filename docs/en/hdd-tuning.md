# HDD tuning

plocate-server is designed for shared hosts where it must not disturb a
foreground service. On SSD this is automatic; on HDD the synchronous `stat`
fan-out in `parse_paths` (`src/state.rs:249`) becomes the dominant cost, and
a few knobs matter.

This page summarises the findings from
[`bench/docs/layer1-runbook.md`](../../bench/docs/layer1-runbook.md) — read
that for the full methodology and reproduction steps.

## The pathology

plocate returns paths only; the server stats each one to classify
`file` vs `directory`. On HDD those `lstat` calls are seek-bound: a single
5 ms seek × 1000 candidates = 5 seconds per fuzzy query. The kernel page
cache helps once warm, but the cache is cold after every reindex (the moka
`stat_cache` is invalidated by default) and after a process restart.

## Measured latency (888k paths, `bench/results/20260628-145044-layer1/`)

| Disk regime | `/api/search` p99 | `/api/fuzzy` p99 | Notes |
| --- | --- | --- | --- |
| A — SSD | 3.84 ms | 15.41 ms | Production-comfortable. |
| B — HDD, 5 ms seek | 519 ms | **5138 ms** at 0.2 RPS | Production-breaking. |
| C — HDD, 15 ms seek | 1531 ms | **15158 ms** at 0.1 RPS | Effectively unusable for fuzzy. |

Peak RSS stayed at ~21 MiB across all regimes — the cache is bounded, no leak.

Reproduced with `bench/bin/slowio.so`, an `LD_PRELOAD` shim that injects
configurable sleep into the `stat`-family syscalls (`SLOWIO_STAT_US`).

## Tuning knobs

| Flag | Default | HDD guidance |
| --- | --- | --- |
| `--fuzzy-candidate-cap` | `1000` | Lower to cap the stat fan-out. `200`–`500` on slow HDD. Trades recall for latency. |
| `--queue-timeout-secs` | `5` | Keep low — `503` is better than queueing on a saturated disk. |
| `--max-concurrent-searches` | `8` | Lower to `2`–`4` on HDD so concurrent stats don't thrash the spindle. |
| `--invalidate-stat-cache-on-reindex` | `true` | Consider `false` on HDD — accept a brief type-staleness window after reindex in exchange for keeping the warm cache. |

Example HDD-tuned unit override:

```ini
# systemctl edit plocate-server
[Service]
ExecStart=
ExecStart=/usr/local/bin/plocate-server \
    --base-path=/srv/files \
    --db-path=/var/lib/plocate-server/files.db \
    --max-concurrent-searches=4 \
    --fuzzy-candidate-cap=300 \
    --queue-timeout-secs=3 \
    --invalidate-stat-cache-on-reindex=false
```

## What does NOT help

- **cgroup v2 `io.max`** — a no-op on multi-queue NVMe; the kernel only wires
  `io.cost.*` there. On HDD, `IOSchedulingClass=idle` in the systemd unit is
  the real lever, and it already applies.
- **Throwing more concurrency at it** — concurrent stats compete for the same
  spindle; higher `max-concurrent-searches` usually *worsens* HDD p99.
- **Pre-warming the cache at startup** — abandoned: a full corpus stat sweep
  would itself take minutes on HDD and starve the foreground service, the
  exact thing the design avoids.

## When `fuzzy` is simply the wrong tool

On HDD regime C, fuzzy is effectively unusable. Prefer:

- `/api/search` with a precise substring (single seek pattern, fast).
- `/api/glob` with a tight pattern (few candidates, small fan-out).
- Save `/api/fuzzy` for warm caches / SSD deployments.

## Reproducing the numbers

```bash
task bench-build-slowio                          # build bench/bin/slowio.so
task bench-serve-hdd SLOWIO_STAT_US=5000         # regime B
task bench-baseline -- --mode fuzzy --rate 0.2 --duration 60
```

See [benchmark](./benchmark.md) for the harness overview and
[layer1-runbook](../../bench/docs/layer1-runbook.md) for the full matrix.
