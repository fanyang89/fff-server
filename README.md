# fff-server

A RESTful **filename / path search** API server for very large file trees
(millions of files), backed by a [plocate](https://plocate.sesse.net/) trigram
index that lives on disk.

The index is built and refreshed by `updatedb` and queried by `plocate`. Because
the index is on disk, **a process restart never rescans** — the server starts
instantly and is ready to serve. Designed for a shared host where it must not
disturb a foreground service (e.g. `dufs`).

## Features

- Filename / path search via plocate's trigram inverted index (sub-millisecond
  even at 10M+ files)
- On-disk index — restart is instant, no rescan
- Periodic reindex via in-server interval + on-demand `POST /api/reindex`
- Glob matching
- Auto-generated OpenAPI 3.0 spec + Swagger UI
- **mimalloc** global allocator
- Fully-static musl binary via **cargo-zigbuild** (single-file deploy)
- cgroup-bounded so it never starves the foreground service

## Runtime requirements

The host needs the **`plocate`** package, which provides both the `plocate`
query binary and `updatedb`:

```bash
sudo dnf install plocate     # Fedora
sudo apt install plocate     # Debian/Ubuntu
```

## Build (static musl)

A single fully-statically-linked binary built with
[cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild). No C dependencies,
so the build is fast and the binary has no runtime libc requirement.

### Prerequisites (build host)

```bash
rustup target add x86_64-unknown-linux-musl
cargo install cargo-zigbuild
# install zig 0.16+ (https://ziglang.org/download/)
```

### Build

```bash
task build           # cargo zigbuild --release --target x86_64-unknown-linux-musl
task inspect         # confirm: "statically linked" / "not a dynamic executable"
```

Output: `target/x86_64-unknown-linux-musl/release/fff-server`.

### Single-file deploy

```bash
scp target/x86_64-unknown-linux-musl/release/fff-server host:/usr/local/bin/
# the binary is self-contained; plocate + updatedb must exist on the target host
```

For local gnu development without zig, use `task run` (or `cargo run`).

## Quick start

```bash
cargo run --release -- --base-path /srv/files
```

The first start builds the index in the background (searches return empty until
ready). Subsequent starts reuse the on-disk index and serve immediately. Open:

```
http://127.0.0.1:8787/swagger-ui
```

## Configuration

All flags have matching environment variables.

| Flag                        | Env                                  | Default                              |
|-----------------------------|--------------------------------------|--------------------------------------|
| `--base-path`               | `FFF_SERVER_BASE_PATH`               | *(required)*                         |
| `--bind`                    | `FFF_SERVER_BIND`                    | `127.0.0.1:8787`                     |
| `--db-path`                 | `FFF_SERVER_DB_PATH`                 | `$XDG_DATA_HOME/fff-server/files.db` |
| `--plocate-bin`             | `FFF_SERVER_PLOCATE_BIN`             | `plocate`                            |
| `--updatedb-bin`            | `FFF_SERVER_UPDATEDB_BIN`            | `updatedb`                           |
| `--reindex-interval-secs`   | `FFF_SERVER_REINDEX_INTERVAL_SECS`   | `21600` (6h; `0` disables)           |
| `--max-results`             | `FFF_SERVER_MAX_RESULTS`             | `100`                                |

## API

| Method | Path              | Description                                            |
|--------|-------------------|--------------------------------------------------------|
| GET    | `/api/search`     | Filename/path search (substring or glob)               |
| GET    | `/api/glob`       | Explicit glob search                                   |
| GET    | `/api/health`     | Index + binary health                                  |
| GET    | `/api/stats`      | Process RSS/threads, db size/mtime, last reindex       |
| POST   | `/api/reindex`    | Trigger a background `updatedb` run                    |
| GET    | `/api/base-path`  | Currently indexed root                                 |

### Examples

```bash
# Substring search (case-insensitive by default)
curl 'http://127.0.0.1:8787/api/search?q=invoice&limit=20'

# Match basename only
curl 'http://127.0.0.1:8787/api/search?q=readme&scope=basename'

# Case-sensitive
curl 'http://127.0.0.1:8787/api/search?q=README&case=true'

# Glob
curl 'http://127.0.0.1:8787/api/glob?pattern=*2024*.log'

# Force a refresh
curl -X POST http://127.0.0.1:8787/api/reindex

# Health
curl http://127.0.0.1:8787/api/health
```

### Notes on results

- plocate's index stores **paths only** — no size/mtime/git metadata. Items
  contain `name`, `relative_path`, `absolute_path`, and `type` (inferred from a
  trailing `/` for directories). This matches the "filename and path only" use
  case.
- `total_matched` is the number of entries plocate returned up to the requested
  cap (`offset + limit`), not an exact total over the whole index. `truncated`
  indicates more matches likely exist.

## Query syntax

`/api/search` passes the pattern to plocate after a `--` separator (no shell,
so no injection). plocate treats a pattern as:

- a **substring** if it has no glob metacharacters,
- a **glob** if it contains `*`, `?`, or `[` (must be wrapped in `*...*` to also
  match substrings).

Multiple patterns are AND-ed. See `plocate(1)` for the full semantics.

## How it works

```
HTTP request
   │  axum handler spawns:
   ▼
plocate -d <db> -i -N -0 -l <cap> -- <pattern>     (short-lived child process)
   │  reads the on-disk trigram index (mmap, io_uring), streams NUL-separated paths
   ▼
parsed → JSON
```

The index is produced independently by `updatedb -U <root> -o <db>`, run either
by the in-server interval or by `POST /api/reindex`. Because the index is a file,
the server process holds **no index in RAM** — its footprint is just the HTTP
runtime (~7 MB RSS). This is what makes it safe to run alongside a busy file
server.

## Deployment & resource control

### Install

```bash
# 1. Build & install the binary
task build
sudo install -m 0755 target/x86_64-unknown-linux-musl/release/fff-server /usr/local/bin/fff-server

# 2. Ensure plocate is installed
sudo dnf install plocate

# 3. Dedicated unprivileged user
sudo useradd -r -s /usr/sbin/nologin -d /var/lib/fff-server -M fff-server
sudo install -d -o fff-server -g fff-server /var/lib/fff-server

# 4. Install the unit (shipped at deploy/fff-server.service)
sudo install -m 0644 deploy/fff-server.service /etc/systemd/system/
sudo systemctl daemon-reload

# 5. Point it at your tree (override ExecStart without editing the file)
sudo systemctl edit fff-server
#   [Service]
#   ExecStart=
#   ExecStart=/usr/local/bin/fff-server \
#       --base-path=/srv/files \
#       --db-path=/var/lib/fff-server/files.db \
#       --reindex-interval-secs=21600

sudo systemctl enable --now fff-server
```

### Resource limits

The shipped unit applies these cgroup v2 constraints (edit to taste):

| Directive             | Value     | Effect                                                     |
|-----------------------|-----------|------------------------------------------------------------|
| `MemoryMax`           | `1G`      | Hard RSS ceiling. The server holds no index in RAM.       |
| `AmbientCapabilities` | `CAP_DAC_READ_SEARCH` | Lets `updatedb` traverse the whole tree without root. |
| `CPUWeight`           | `20`      | Low weight vs default 100 — yields CPU under load.         |
| `Nice`                | `19`      | Lowest static priority.                                    |
| `IOSchedulingClass`   | `idle`    | Disk IO only served when no one else wants it.             |

`Nice=19` + `IOSchedulingClass=idle` are the strongest guarantees that a busy
foreground service is never starved; `updatedb` runs inherit these too. No
`CPUQuota` — capacity is not wasted when the host is idle.

### Permissions

`updatedb` must read every file under `--base-path` to index it. The unit grants
`CAP_DAC_READ_SEARCH` so the unprivileged `fff-server` user can do this without
running as root. The resulting `files.db` is owned by `fff-server`, so the
`plocate` child can read it back directly.

### Observe

```bash
curl http://127.0.0.1:8787/api/stats | jq    # RSS, db size/mtime, last reindex
curl http://127.0.0.1:8787/api/health | jq
systemctl status fff-server
```

## License

MIT.
