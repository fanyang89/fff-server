# 性能基准

plocate-server 以工作区成员的形式附带一个基于 `rlt` 的压测工具。源码：
`bench/`。它无协调遗漏（governor 令牌桶 pacing），同时覆盖恒定 RPS 和恒定
并发两种模式。

## 运行

`Taskfile.yml` 暴露了常见场景：

```bash
# 恒定 RPS —— 测稳态负载下的延迟
task bench-baseline -- --rate 50 --duration 60

# 恒定并发 —— 饱和以找吞吐上限
task bench-saturate -- --concurrency 32 --duration 60
```

两者都委托给 `cargo run -p bench --release -- …`。直接调用：

```bash
cargo run -p bench --release -- \
    --url http://127.0.0.1:8787 \
    --mode substring \
    --queries bench/data/queries.txt \
    --rate 50 --duration 60
```

## 模式

| `--mode` | 命中端点 | 查询来源 |
| --- | --- | --- |
| `substring`（默认） | `/api/search` | `bench/data/queries.txt` |
| `glob` | `/api/glob` | `bench/data/queries.txt` |
| `fuzzy` | `/api/fuzzy` | `bench/data/queries-fuzzy.txt` |

查询从语料库轮询采样（`iter_no % len`，无 RNG），运行可复现。

## 数据集

`task bench-data -- 10k|100k|1m` 在 `bench/data/tree/` 下生成合成文件树并
构建 `bench/data/files.db`。规模：

- `10k` —— 快速冒烟测试
- `100k` —— 典型开发运行
- `1m` —— 压力 / 回归门禁

## HDD 模拟

`bench/bin/slowio.so` 是一个 `LD_PRELOAD` shim，向 `stat` 系列系统调用
注入可配置睡眠（`SLOWIO_STAT_US`）：

```bash
task bench-build-slowio                              # 一次性
task bench-serve-hdd SLOWIO_STAT_US=5000             # 5 ms ≈ 7200 转 HDD
task bench-baseline -- --mode fuzzy --rate 0.2 --duration 60
```

结论与完整矩阵见 [HDD 调优](./hdd-tuning.md)。

## 参考结果（`bench/results/20260628-145044-layer1/`）

888,016 条唯一路径，8.3 MB db。恒定 RPS，三档磁盘：

| 档位 | 模式 | RPS | p50 | p99 |
| --- | --- | --- | --- | --- |
| A —— SSD | substring | 49.6 | 2.60 ms | 3.84 ms |
| A —— SSD | fuzzy | 10.0 | 10.04 ms | 15.41 ms |
| B —— HDD 5 ms | substring | 2.0 | 413 ms | 519 ms |
| B —— HDD 5 ms | fuzzy | 0.2 | 3666 ms | 5138 ms |
| C —— HDD 15 ms | substring | 1.0 | 1223 ms | 1531 ms |
| C —— HDD 15 ms | fuzzy | 0.1 | 11290 ms | 15158 ms |

所有档位下峰值 RSS 都维持在 ~21 MiB。完整 JSON 与监控时间序列在结果目录
里。

## 回归门禁

`rlt` 支持命名基线，使 CI 能在回归时失败：

```bash
cargo run -p bench --release -- --rate 50 --duration 30 --save-baseline main
# ... 改动后 ...
cargo run -p bench --release -- --rate 50 --duration 30 \
    --baseline main --fail-on-regression
```

## 测的是什么

bench 客户端测量的是单个 plocate-server 实例的 **端到端 HTTP 延迟** —— 包
括 `plocate` 子进程拉起、mmap 索引读取、`stat` 扇出和 JSON 序列化。它不是
任一层的微基准；它反映真实客户端看到的延迟。
