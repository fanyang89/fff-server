# HDD 调优

plocate-server 为共享主机设计，必须不干扰前台服务。SSD 上这是自动的；HDD
上 `parse_paths`（`src/state.rs:249`）里的同步 `stat` 扇出会成为主导成本，
几个旋钮很关键。

本页总结自 [`bench/docs/layer1-runbook.md`](../../bench/docs/layer1-runbook.md)
—— 完整方法论与复现步骤请读那份文档。

## 病理

plocate 只返回路径；服务器要 stat 每条路径才能分类 `file` 还是 `directory`。
HDD 上这些 `lstat` 调用受寻道约束：单次 5 ms 寻道 × 1000 候选 = 每条模糊
查询 5 秒。内核页缓存一旦热起来有帮助，但缓存会在每次 reindex 后冷掉
（moka `stat_cache` 默认失效）以及进程重启后冷掉。

## 实测延迟（88.8 万路径，`bench/results/20260628-145044-layer1/`）

| 磁盘档位 | `/api/search` p99 | `/api/fuzzy` p99 | 备注 |
| --- | --- | --- | --- |
| A —— SSD | 3.84 ms | 15.41 ms | 生产可用。 |
| B —— HDD，5 ms 寻道 | 519 ms | **5138 ms**（0.2 RPS） | 生产不可用。 |
| C —— HDD，15 ms 寻道 | 1531 ms | **15158 ms**（0.1 RPS） | fuzzy 基本不可用。 |

所有档位下峰值 RSS 都维持在 ~21 MiB —— 缓存有界，无泄漏。

通过 `bench/bin/slowio.so` 复现，这是一个 `LD_PRELOAD` shim，向 `stat` 系列
系统调用注入可配置的睡眠（`SLOWIO_STAT_US`）。

## 调优旋钮

| Flag | 默认 | HDD 指导 |
| --- | --- | --- |
| `--fuzzy-candidate-cap` | `1000` | 调低以约束 stat 扇出。慢 HDD 上 `200`–`500`。以召回率换延迟。 |
| `--queue-timeout-secs` | `5` | 保持低值 —— 磁盘饱和时 `503` 比排队好。 |
| `--max-concurrent-searches` | `8` | HDD 上调到 `2`–`4`，避免并发 stat 抢同一根磁臂。 |
| `--invalidate-stat-cache-on-reindex` | `true` | HDD 上考虑 `false` —— 接受 reindex 后短暂的类型陈旧，换取保留热缓存。 |

HDD 调优 unit override 示例：

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

## 没有帮助的做法

- **cgroup v2 `io.max`** —— 在多队列 NVMe 上无效；内核只接 `io.cost.*`。
  HDD 上 systemd unit 里的 `IOSchedulingClass=idle` 才是真正的杠杆，且已
  默认应用。
- **加大并发** —— 并发 stat 争用同一磁臂；提高 `max-concurrent-searches`
  通常会 *恶化* HDD 的 p99。
- **启动时预热缓存** —— 已放弃：对整库做一次 stat 扫描在 HDD 上本身要花
  几分钟，且会饿死前台服务，正是设计要避免的。

## `fuzzy` 根本用不上的情形

在 HDD C 档下，fuzzy 基本不可用。优先：

- `/api/search` 配精确子串（单次寻道模式，快）。
- `/api/glob` 配紧凑模式（候选少，扇出小）。
- 把 `/api/fuzzy` 留给热缓存 / SSD 部署。

## 复现这些数字

```bash
task bench-build-slowio                          # 构建 bench/bin/slowio.so
task bench-serve-hdd SLOWIO_STAT_US=5000         # B 档
task bench-baseline -- --mode fuzzy --rate 0.2 --duration 60
```

工具概览见 [性能基准](./benchmark.md)，完整矩阵见
[layer1-runbook](../../bench/docs/layer1-runbook.md)。
