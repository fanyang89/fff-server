# fff-server

A RESTful API server built on top of the [`fff`](https://github.com/dmtrKovalenko/fff)
file-search engine. It keeps a single indexed directory resident in one
long-lived process and exposes frecency-ranked fuzzy search, glob matching,
and lifecycle management over HTTP — with an interactive Swagger UI.

## Features

- Fuzzy file / directory / mixed search with typo tolerance and constraint syntax
- Literal glob matching, frecency-ranked
- Frecency access tracking + query history (combo-boost scoring)
- Index lifecycle: health, scan progress, force rescan, refresh git status
- Auto-generated OpenAPI 3.0 spec + Swagger UI
- **mimalloc** global allocator + periodic heap compaction (`mimalloc-collect`)
- Fully-static musl binary via **cargo-zigbuild** (single-file deploy, no runtime deps)

## Build (static musl)

The release artifact is a single fully-statically-linked binary built with
[cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) against the
`x86_64-unknown-linux-musl` target. All native C deps (libgit2, zlib, LMDB,
mimalloc) are vendored — **no system C libraries required** to build or run.

### Prerequisites (build host)

```bash
rustup target add x86_64-unknown-linux-musl
cargo install cargo-zigbuild
# install zig 0.16+ (https://ziglang.org/download/)
```

### Build

```bash
task build           # or: cargo zigbuild --release --target x86_64-unknown-linux-musl
task inspect         # confirm: "statically linked" / "not a dynamic executable"
```

Output: `target/x86_64-unknown-linux-musl/release/fff-server` (~19 MB, stripped).

### Single-file deploy

```bash
scp target/x86_64-unknown-linux-musl/release/fff-server host:/usr/local/bin/
# no ld, no libc, no libssl — it just runs
```

For local gnu development without zig, use `task run` (or `cargo run`).

### mimalloc

mimalloc is wired as the Rust global allocator (`#[global_allocator]`), and the
`mimalloc-collect` feature of fff-search is enabled so the engine's background
pool periodically forces heap compaction (`mi_collect(true)`) — keeping RSS
tight under the cgroup `MemoryMax` ceiling. C libraries (libgit2, LMDB) keep
musl's allocator; this is intentional to avoid double-interposition.

## Quick start

```bash
cargo run --release -- --base-path /path/to/your/repo
```

Then open the Swagger UI:

```
http://127.0.0.1:8787/swagger-ui
```

Raw spec: `GET /openapi.json` (importable into Postman / Apifox).

## Configuration

All flags have matching environment variables.

| Flag                  | Env                            | Default                          |
|-----------------------|--------------------------------|----------------------------------|
| `--base-path`         | `FFF_SERVER_BASE_PATH`         | *(required)*                     |
| `--bind`              | `FFF_SERVER_BIND`              | `127.0.0.1:8787`                 |
| `--db-dir`            | `FFF_SERVER_DB_DIR`            | `$XDG_CACHE_HOME/fff-server`     |
| `--ai-mode`           | `FFF_SERVER_AI_MODE`           | `true`                           |
| `--watch`             | `FFF_SERVER_WATCH`             | `true`                           |
| `--content-indexing`  | `FFF_SERVER_CONTENT_INDEXING`  | `false`                          |
| `--mmap-cache`        | `FFF_SERVER_MMAP_CACHE`        | `false`                          |
| `--wait-scan-secs`    | `FFF_SERVER_WAIT_SCAN_SECS`    | `10`                             |
| `--max-results`       | `FFF_SERVER_MAX_RESULTS`       | `100`                            |

## API

| Method | Path                | Description                              |
|--------|---------------------|------------------------------------------|
| GET    | `/api/search`       | Fuzzy search (`mode=files\|dirs\|mixed`) |
| GET    | `/api/glob`         | Literal glob search                      |
| GET    | `/api/history`      | Retrieve a historical query              |
| POST   | `/api/track`        | Record a file access / query completion  |
| GET    | `/api/health`       | Engine + DB health                       |
| GET    | `/api/scan-progress`| Current scan progress                    |
| POST   | `/api/rescan`       | Trigger a full rescan                    |
| POST   | `/api/refresh-git`  | Refresh cached git statuses              |
| GET    | `/api/base-path`    | Currently indexed root                   |
| GET    | `/api/stats`        | Runtime: RSS / threads / index / cache   |

### Examples

```bash
# Fuzzy search for Rust files, exclude tests
curl 'http://127.0.0.1:8787/api/search?q=*.rs%20!test/%20schema&limit=20'

# Glob
curl 'http://127.0.0.1:8787/api/glob?pattern=**/*.toml'

# Track an access (feeds frecency ranking)
curl -X POST http://127.0.0.1:8787/api/track \
  -H 'Content-Type: application/json' \
  -d '{"path":"src/main.rs","query":"main"}'

# Health
curl http://127.0.0.1:8787/api/health
```

## Query syntax

`/api/search` accepts the full fff constraint language:

- `*.rs` / `*.{rs,toml}` — extension / glob filters
- `test/` — anything nested under `test/`
- `!something`, `!test/`, `!git:modified` — exclusions
- `git:modified`, `git:untracked`, `git:staged`, ... — git-status filters
- mix freely, e.g. `git:modified src/**/*.rs !src/**/mod.rs user controller`

## How it works

The server indexes one directory at startup and keeps the index resident.
Every search hits warm memory (sub-10 ms typical), so it is far cheaper than
forking `rg` / `fzf` per request. See the
[fff README](https://github.com/dmtrKovalenko/fff#what-is-fff-and-why-use-it-over-ripgrep-or-fzf)
for the algorithmic details.

Search calls are dispatched onto `tokio::task::spawn_blocking` since the fff
engine is CPU-bound (rayon) and must not block the async reactor.

## Deployment & resource control

fff keeps the file index resident in RAM, so on a shared host it can compete
with other services for memory and CPU. A systemd unit with cgroup v2 limits
is the recommended way to keep fff-server from affecting the foreground
service (e.g. `dufs`).

### Install

```bash
# 1. Build & install the binary
cargo build --release
sudo install -m 0755 target/release/fff-server /usr/local/bin/fff-server

# 2. Create a dedicated unprivileged user
sudo useradd -r -s /usr/sbin/nologin -d /var/lib/fff-server -M fff-server
sudo install -d -o fff-server -g fff-server /var/lib/fff-server

# 3. Install the unit (shipped at deploy/fff-server.service)
sudo install -m 0644 deploy/fff-server.service /etc/systemd/system/
sudo systemctl daemon-reload

# 4. Point it at your repo (override ExecStart without editing the file)
sudo systemctl edit fff-server
#   in the editor, drop a drop-in like:
#   [Service]
#   ExecStart=
#   ExecStart=/usr/local/bin/fff-server \
#       --base-path=/path/to/your/repo \
#       --bind=127.0.0.1:8787 \
#       --db-dir=/var/lib/fff-server

sudo systemctl enable --now fff-server
```

### Resource limits

The shipped unit applies these cgroup v2 constraints (edit to taste):

| Directive             | Value      | Effect                                                     |
|-----------------------|------------|------------------------------------------------------------|
| `MemoryMax`           | `4G`       | Hard RSS ceiling; OOM-killed + restarted if exceeded.      |
| `MemoryHigh`          | `3500M`    | Soft line; kernel reclaims/throttles before the hard cap.  |
| `CPUWeight`           | `20`       | Low weight vs the default 100 — yields CPU under load.     |
| `Nice`                | `19`       | Lowest static priority.                                    |
| `IOSchedulingClass`   | `idle`     | Disk IO only served when no one else wants it.             |

Together `Nice=19` + `IOSchedulingClass=idle` are the strongest guarantees
that a busy foreground service is never starved, regardless of how the
services are sliced. `CPUWeight` adds proportional fairness when they share
a slice. There is **no `CPUQuota`** — fff bursts to all cores when the
foreground service is idle, so no capacity is wasted. Add `CPUQuota=300%`
if you want a hard 3-core ceiling instead.

### Observe

```bash
# Live runtime stats (RSS, threads, index size, cache use)
curl http://127.0.0.1:8787/api/stats | jq

# cgroup pressure / current usage
systemctl status fff-server
systemd-cgtop -1 -n 1
```

If `rss_bytes` trends toward `MemoryMax`, the indexed repo is large — either
raise the cap, or keep `--content-indexing` / `--mmap-cache` off (the default)
since those drive the bulk of optional memory.

## License

MIT.
