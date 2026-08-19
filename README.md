# bmlrssbot

`bmlrssbot` 是一个面向 AniBT RSS 的 Telegram 频道发布服务。它定时读取订阅，把同一字幕组、同一番剧、同一集数在指定时间窗口内发布的不同语言版本合并成一条图文消息。

消息可包含：

- 番剧名称、集数和发布时间
- 字幕组与分辨率
- 简日内嵌、繁日内嵌、简繁日内封等版本标签
- 每个版本的 AniBT 详情页、infohash 和种子下载链接
- RSS 图片，或 AniBT 详情页中的 `og:image` 封面

服务使用 Rust 编写，推荐通过 Docker Compose 部署。容器以非 root 用户运行，发布状态持久化到本地 `data/` 目录。

## 工作方式

默认情况下，服务每 5 分钟读取一次 RSS，并按以下规则处理条目：

1. 使用“字幕组 + 番剧 + 集数”作为聚合键。番剧优先使用 AniBT 的 Bangumi ID，缺失时回退到番剧名。
2. 同组条目最早与最晚发布时间相差不超过 10 分钟时，合并为一个发布批次。
3. 从批次第一条发布开始等待 10 分钟，让其他语言或字幕版本进入同一帖子。
4. Telegram 发布成功后，批次内所有 RSS UID 才会写入状态文件。
5. 已记录的条目不会重复发布。窗口关闭后才出现的版本会作为新帖子发布。

聚合只发生在同一个 RSS 源内部，不会跨多个 `RSS_FEEDS` 合并条目。

## 部署前准备

需要：

- Docker Engine 及 Docker Compose 插件
- 一台能够访问 AniBT 和 Telegram Bot API 的主机
- Telegram Bot Token
- 目标频道的管理权限

### 1. 创建 Telegram Bot

1. 在 Telegram 打开 [@BotFather](https://t.me/BotFather)。
2. 发送 `/newbot` 并按提示创建 Bot。
3. 保存 BotFather 返回的完整 Token。Token 格式类似 `123456789:AA...`，不要提交到 Git 或发送到公开位置。

### 2. 授予频道权限

将 Bot 加入目标频道并设为管理员，至少授予“发布消息”权限。

公开频道可以使用 `@channelusername` 作为 `TELEGRAM_CHAT_ID`。私有频道需要使用类似 `-1001234567890` 的数字 Chat ID。

如需让帖子显示评论入口，在 Telegram 中进入：

`频道管理 -> 讨论 -> 关联群组`

用户评论不要求 Bot 加入讨论群。只有需要自动读取、回复或管理评论时，才需要把 Bot 同时加入关联群组。

## Docker Compose 部署

### 1. 获取代码

```bash
git clone https://github.com/microseventh/bmlrssbot.git
cd bmlrssbot
```

如果已经位于项目目录，可以直接从下一步开始。

### 2. 创建配置

```bash
cp .env.example .env
```

编辑 `.env`，下面是一份 AniBT 字幕组 RSS 示例：

```dotenv
TELEGRAM_BOT_TOKEN=123456789:replace_with_your_token
TELEGRAM_CHAT_ID=@your_channel
RSS_FEEDS=https://anibt.net/rss/group/billion-meta-lab.xml

POLL_INTERVAL_SECONDS=300
STATE_FILE=/data/state.json
MAX_ITEMS_PER_FEED=100
MAX_POSTS_PER_FEED=5
GROUP_WINDOW_SECONDS=600
DRY_RUN=false
```

`.env` 已被 `.gitignore` 排除，不会被正常的 Git 提交包含。

### 3. 预览发布内容

首次启动前建议执行一次 dry-run。它会抓取 RSS、完成聚合并在终端打印消息，但不会调用 Telegram：

```bash
docker compose build
docker compose run --rm \
  -e DRY_RUN=true \
  -e STATE_FILE=/tmp/bmlrssbot-dry-run.json \
  bmlrssbot --once
```

这里使用临时状态文件，避免预览过的条目被正式运行误认为已经发布。

### 4. 启动服务

```bash
mkdir -p data
docker compose up -d --build
```

查看容器状态和日志：

```bash
docker compose ps
docker compose logs -f --tail=100
```

日志中的 `published=N` 表示本轮成功发布了多少个合并批次。`published=0` 通常表示没有新内容、条目仍在聚合窗口内，或最新批次已经发布。

## 配置说明

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `TELEGRAM_BOT_TOKEN` | 无 | BotFather 提供的完整 Token；非 dry-run 时必填 |
| `TELEGRAM_CHAT_ID` | 无 | 目标频道或群组，例如 `@channel` 或 `-100...`；非 dry-run 时必填 |
| `RSS_FEEDS` | 无 | RSS/Atom 地址；多个地址使用英文逗号分隔 |
| `POLL_INTERVAL_SECONDS` | `300` | 两次轮询之间的等待时间，单位为秒 |
| `STATE_FILE` | `/data/state.json` | 已发布 UID 状态文件在容器内的路径 |
| `MAX_ITEMS_PER_FEED` | `100` | 每轮从单个源读取的最大条目数，应足够覆盖聚合窗口 |
| `MAX_POSTS_PER_FEED` | `5` | 每个源只观察最新的几个批次，避免首次启动倒灌全部历史 |
| `GROUP_WINDOW_SECONDS` | `600` | 同字幕组、番剧和集数的版本聚合与等待时间 |
| `DRY_RUN` | `false` | `true` 时只抓取和渲染，不调用 Telegram API |

多个 RSS 示例：

```dotenv
RSS_FEEDS=https://example.com/feed-a.xml,https://example.com/feed-b.xml
```

## 首次启动与状态文件

服务会解析 `MAX_ITEMS_PER_FEED` 条 RSS 数据，但只观察其中最新的 `MAX_POSTS_PER_FEED` 个聚合批次。因此首次部署不会在后续轮询中逐步发送整个 RSS 历史。

每个 RSS 条目使用 guid、id 或链接生成稳定 UID。只有 Telegram 明确返回成功后，UID 才会保存到状态文件。默认宿主机文件为：

```text
./data/state.json
```

请把 `data/` 纳入备份。丢失状态文件可能导致当前观察窗口中的条目再次发布。

如果 Linux 主机提示无法写入 `/data/state.json`，将挂载目录所有者调整为容器用户 UID `10001`：

```bash
sudo chown -R 10001:10001 data
```

## 日常操作

启动或应用配置变更：

```bash
docker compose up -d --build
```

查看最近日志：

```bash
docker compose logs --tail=100
```

重启服务：

```bash
docker compose restart
```

停止并移除容器和 Compose 网络：

```bash
docker compose down
```

该命令不会删除本地镜像、`.env` 或 `data/` 中的状态文件。

更新代码：

```bash
git pull --ff-only
docker compose up -d --build
```

## 常见问题

### 启动时提示 Token 不完整

`TELEGRAM_BOT_TOKEN` 必须是 BotFather 返回的完整字符串，不能只填写开头的数字 Bot ID。

### Telegram 返回无权限错误

确认 Bot 已加入目标频道并拥有发布消息权限，同时检查 `TELEGRAM_CHAT_ID` 是否指向正确频道。

### RSS 有新条目，但没有立即发布

这是聚合等待的预期行为。默认从批次第一条开始等待 600 秒，再将同字幕组、番剧和集数的版本合并发布。也可以检查日志中是否有 RSS 请求或解析错误。

### 消息没有图片

服务优先读取 RSS 图片，其次尝试 AniBT 详情页的 `og:image`。两处都没有有效图片时，会发送纯文本消息。合并正文超过 Telegram 图片 caption 限制时，也会自动使用纯文本消息以避免截断内容。

### 帖子没有评论按钮

评论区由 Telegram 频道设置控制，与 Bot 发布格式无关。请确认频道已关联讨论群。Bot API 中频道的 `linked_chat_id` 非空时，说明关联已经生效。

## 不使用 Docker

需要 Rust 1.88 或更高版本：

```bash
cp .env.example .env
set -a
source .env
set +a
cargo run --release -- --once
```

长期运行时移除 `--once`。本地运行还需要把 `STATE_FILE` 改成当前用户可写的路径，例如 `./data/state.json`。

## 开发与验证

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
docker compose build
```

## 安全提示

- 不要提交 `.env`、Bot Token 或 `data/` 状态文件。
- 如果 Token 曾出现在公开提交、日志或聊天记录中，请立即通过 BotFather 撤销并重新生成。
- 生产环境建议限制 `.env` 的文件权限，例如执行 `chmod 600 .env`。

## License

本项目采用 MIT License，详见 [LICENSE](LICENSE)。
