# bmlrssbot

Rust 服务：定时读取 RSS/Atom 订阅，将条目转换为 Telegram HTML 图文消息并发布到频道或群组。项目使用多阶段 Docker 构建，运行时为非 root 用户，发布状态保存在挂载卷中。

## 快速开始

1. 在 Telegram 中打开 [@BotFather](https://t.me/BotFather)，执行 `/newbot` 创建机器人并取得 Bot Token。
2. 将机器人加入目标频道并授予发布消息权限。频道可使用 `@channelusername`，私有频道使用类似 `-1001234567890` 的 chat id。
3. 复制配置并启动：

```bash
cp .env.example .env
# 编辑 .env，至少设置 TELEGRAM_BOT_TOKEN、TELEGRAM_CHAT_ID、RSS_FEEDS
docker compose up -d --build
```

首次验证可以不调用 Telegram：

```bash
DRY_RUN=true docker compose run --rm bmlrssbot --once
```

也可以本地编译运行（需要 Rust 1.88+）：

```bash
cargo run --release -- --once
```

## 配置

| 变量 | 必填 | 说明 |
| --- | --- | --- |
| `TELEGRAM_BOT_TOKEN` | 是（dry-run 除外） | @BotFather 生成的 Token |
| `TELEGRAM_CHAT_ID` | 是（dry-run 除外） | 目标频道或群组 |
| `RSS_FEEDS` | 是 | 逗号分隔的 RSS/Atom URL |
| `POLL_INTERVAL_SECONDS` | 否 | 轮询间隔，默认 300 秒 |
| `STATE_FILE` | 否 | 已发布 UID 文件，Docker 默认 `/data/state.json` |
| `MAX_ITEMS_PER_FEED` | 否 | 每个源每轮最多读取条数，默认 100，用于完整覆盖聚合窗口 |
| `MAX_POSTS_PER_FEED` | 否 | 每个源只观察最新的发布批次，默认 5，避免首次启动倒灌历史 |
| `GROUP_WINDOW_SECONDS` | 否 | 同字幕组、番剧和集数的语言版本聚合窗口，默认 600 秒；窗口结束后发布 |
| `DRY_RUN` | 否 | `true` 时只抓取和渲染，不调用 Telegram |

程序使用条目的 id/guid/link 生成 SHA-256 UID，成功发布后才写入状态，因此网络或 Telegram 失败不会被错误标记为已发布。

## 官方 API 与项目调研

- [Telegram Bot API](https://core.telegram.org/bots/api)：当前官方页面标示 Bot API 10.2。本文项目使用 `sendMessage`、`sendPhoto`，请求地址为 `https://api.telegram.org/bot<TOKEN>/<method>`。
- [feedforbot](https://github.com/shpaker/feedforbot)：MIT，提供调度、缓存、健康检查和 flood-wait 重试，适合作为工程化参考。
- [RSS-to-Telegram-Bot](https://github.com/Rongronggg9/RSS-to-Telegram-Bot)：AGPL-3.0，功能完整，支持多用户、媒体和格式化；本项目保持更小的单用途部署面。
- [rss-feed-telegram-bot](https://github.com/viperadnan-git/rss-feed-telegram-bot)：GPL-3.0，验证了 RSS 到频道发布的最小流程。

本项目不复制上述项目代码；依赖和许可证信息以各上游仓库为准。

## 开发

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

项目采用 MIT License，详见 [LICENSE](LICENSE)。提交前请确认不要将 `.env`、Bot Token 或状态文件提交到 Git。
