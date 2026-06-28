# 快速上手

## 1. 安装宿主依赖

plocate-server 通过调用 `plocate` 查询二进制和 `updatedb` 索引器来工作，
两者都由 **`plocate`** 软件包提供：

```bash
sudo dnf install plocate     # Fedora / RHEL
sudo apt install plocate     # Debian / Ubuntu
sudo pacman -S plocate       # Arch
```

验证：

```bash
plocate --version
updatedb --version
```

## 2. 从源码运行

```bash
git clone https://github.com/fanyang89/plocate-server
cd plocate-server
cargo run --release -- --base-path /srv/files
```

首次启动会在后台拉起 `updatedb` 构建 `files.db`。索引就绪前搜索返回空；
后续启动直接复用磁盘上的索引并立即提供服务。

打开：

| URL | 入口 |
| --- | --- |
| http://127.0.0.1:8787 | React 单页应用 |
| http://127.0.0.1:8787/swagger-ui | Swagger UI（"Try it out"） |
| http://127.0.0.1:8787/openapi.json | OpenAPI 3.0 规范 |

## 3. 构建全静态二进制（可选，用于部署）

通过 [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) 生成
单个完全静态链接的 musl 二进制。无 C 运行时依赖，单文件即可分发。

```bash
rustup target add x86_64-unknown-linux-musl
cargo install cargo-zigbuild
# zig 0.16+ — https://ziglang.org/download/

task build           # cargo zigbuild --release --target x86_64-unknown-linux-musl
task inspect         # 确认输出 "statically linked"
```

产物路径：`target/x86_64-unknown-linux-musl/release/plocate-server`。

部署：

```bash
scp target/x86_64-unknown-linux-musl/release/plocate-server host:/usr/local/bin/
```

本机 gnu 开发（不依赖 zig）请用 `task run`（或裸 `cargo run`）。

## 4. 指向目标目录

`--base-path` 是唯一必填项 —— 它告诉 `updatedb` 要索引哪个根目录。该根
之下的所有路径都会变得可搜索；该根之外的任何路径永远不可达。

```bash
plocate-server --base-path /srv/files \
               --db-path /var/lib/plocate-server/files.db
```

## 下一步

- 完整 flag 列表 → [配置参考](./configuration.md)
- systemd unit + cgroup → [部署运维](./deployment.md)
- API 总览 → [REST API](./api.md)
