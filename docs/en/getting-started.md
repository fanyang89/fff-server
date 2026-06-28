# Getting started

## 1. Install the host dependency

plocate-server shells out to the `plocate` query binary and the `updatedb`
indexer, both shipped by the **`plocate`** package:

```bash
sudo dnf install plocate     # Fedora / RHEL
sudo apt install plocate     # Debian / Ubuntu
sudo pacman -S plocate       # Arch
```

Verify:

```bash
plocate --version
updatedb --version
```

## 2. Run from source

```bash
git clone https://github.com/fanyang89/plocate-server
cd plocate-server
cargo run --release -- --base-path /srv/files
```

The first start spawns `updatedb` in the background to build
`files.db`. Searches return empty until that finishes; subsequent starts reuse
the on-disk index and serve immediately.

Open:

| URL | Surface |
| --- | --- |
| http://127.0.0.1:8787 | React SPA |
| http://127.0.0.1:8787/swagger-ui | Swagger UI ("Try it out") |
| http://127.0.0.1:8787/openapi.json | OpenAPI 3.0 spec |

## 3. Build a fully-static binary (optional, for deploy)

A single statically-linked musl binary via
[cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild). No C runtime
dependency, one file to ship.

```bash
rustup target add x86_64-unknown-linux-musl
cargo install cargo-zigbuild
# zig 0.16+ — https://ziglang.org/download/

task build           # cargo zigbuild --release --target x86_64-unknown-linux-musl
task inspect         # confirm: "statically linked"
```

Output: `target/x86_64-unknown-linux-musl/release/plocate-server`.

Deploy:

```bash
scp target/x86_64-unknown-linux-musl/release/plocate-server host:/usr/local/bin/
```

For local gnu development without zig, use `task run` (or bare `cargo run`).

## 4. Point it at a tree

`--base-path` is the only required flag — it tells `updatedb` which root to
index. Everything under that root becomes searchable; nothing outside it is
ever reachable.

```bash
plocate-server --base-path /srv/files \
               --db-path /var/lib/plocate-server/files.db
```

## Next

- Full flag list → [configuration](./configuration.md)
- systemd unit + cgroup → [deployment](./deployment.md)
- API surface → [api](./api.md)
