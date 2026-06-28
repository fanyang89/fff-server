# plocate-server — Agent Notes

## Pre-commit Checks

Before committing any Rust or web change, run **all** of these locally.
CI runs the same set and will fail the build otherwise.

```bash
# Rust (workspace root — covers plocate-server + bench)
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all

# Web (under web/)
pnpm --dir web lint
pnpm --dir web build
```

If `cargo fmt --check` reports a diff, run `cargo fmt --all` and commit
the formatting separately (or fold into the change commit). Do not bypass
the check.

## Workspace Layout

This is a Cargo workspace (`[workspace] members = [".", "bench"]` in the
root `Cargo.toml`):

- `.` — the `plocate-server` crate (sources in `src/`, frontend embedded
  from `web/dist/` via rust-embed at release build time)
- `bench/` — the `rlt`-based load testing harness; binary `bench`, run
  via `cargo run -p bench --release -- ...` or the `bench-*` Taskfile tasks

Always use `cargo <cmd> -p <crate>` (or `--all`) — bare `cargo <cmd>` in
a workspace dir picks an ambiguous default.

## Build System

`Taskfile.yml` is the canonical entry point for non-trivial build flows:

- `task build` — release musl binary (via cargo-zigbuild, needs zig 0.16+
  installed locally)
- `task web-build` / `task web-dev` — frontend
- `task rpm` / `task pacman` / `task packages` — Linux packages via nfpm
- `task bench-*` — load testing harness entry points

`task` is also what the release workflow runs; mirroring CI locally means
running the same `task` invocations.

## Branch / Commit Conventions

- Commit directly to `main` (no PR workflow for routine changes)
- Conventional Commit style (`feat:`, `fix:`, `refactor:`, `docs:`,
  `style:`, `chore:`, `ci:`)
- One self-contained change per commit — each commit should build and
  pass tests on its own

## HDD-specific Considerations

The server has explicit support for slow-disk deployments. When changing
anything in `src/state.rs` that touches `is_dir_cached`, `parse_paths`,
or the `search_concurrency` semaphore, re-read the Layer 1 findings in
`bench/docs/layer1-runbook.md` — the synchronous stat fan-out is the
single biggest production risk on HDD and the reason for several
non-obvious design choices (spawn_blocking, configurable
`--fuzzy-candidate-cap`, `--queue-timeout-secs`).
