# plocate-server docs

> **Language:** English · [中文](../zh/README.md)

Reference documentation for plocate-server. For a 30-second pitch and the
quick start, see the [top-level README](../../README.md).

## Contents

| # | Topic | When to read |
| --- | --- | --- |
| 1 | [Getting started](./getting-started.md) | First install, first run |
| 2 | [Configuration](./configuration.md) | All 17 flags + env vars |
| 3 | [REST API](./api.md) | Querying `/api/*` |
| 4 | [MCP integration](./mcp.md) | Wiring up an AI agent |
| 5 | [Deployment](./deployment.md) | systemd, cgroup, packages, reverse proxy |
| 6 | [Architecture](./architecture.md) | How the engine fits together |
| 7 | [HDD tuning](./hdd-tuning.md) | Slow-disk deployments |
| 8 | [Benchmark](./benchmark.md) | Running the load-test harness |

## Conventions

- Flag names use kebab-case (`--max-results`); env vars use the
  `PLOCATE_SERVER_*` prefix (`PLOCATE_SERVER_MAX_RESULTS`).
- File:line references point into this repository's source tree.
- Measured numbers come from `bench/results/20260628-145044-layer1/`.
