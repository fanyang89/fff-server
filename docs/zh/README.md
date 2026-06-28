# plocate-server 文档

> **语言：** [English](../en/README.md) · 中文

plocate-server 参考文档。30 秒速览与快速上手请看[仓库根目录的 README](../../README.md)。

## 目录

| # | 主题 | 何时阅读 |
| --- | --- | --- |
| 1 | [快速上手](./getting-started.md) | 首次安装、首次运行 |
| 2 | [配置参考](./configuration.md) | 全部 17 个 flag 与环境变量 |
| 3 | [REST API](./api.md) | 调用 `/api/*` |
| 4 | [MCP 集成](./mcp.md) | 接入 AI Agent |
| 5 | [部署运维](./deployment.md) | systemd、cgroup、打包、反向代理 |
| 6 | [架构原理](./architecture.md) | 引擎如何协作 |
| 7 | [HDD 调优](./hdd-tuning.md) | 慢盘部署 |
| 8 | [性能基准](./benchmark.md) | 运行压测工具 |

## 约定

- flag 名使用 kebab-case（`--max-results`）；环境变量使用 `PLOCATE_SERVER_*`
  前缀（`PLOCATE_SERVER_MAX_RESULTS`）。
- 文中的 `文件:行号` 引用指向本仓库源码。
- 实测数据来自 `bench/results/20260628-145044-layer1/`。
