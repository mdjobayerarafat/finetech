---
id: ops-deployment-zh
title: "0 · 部署 (中文)"
sidebar_label: "0 · 部署 (中文)"
sidebar_position: 1
---

# 0. 部署

> English: [00-deployment.md](../00-deployment.md)

本页是**生产运维**参考:如何搭建系统、放在它后面什么、如何
迁移数据库、如何备份、如何升级、如何回滚。开发快速上手在
[`../architecture/05-deployment.md`](../architecture/05-deployment.md)
— `./ops/dev.sh` 那一节先看。本页与它形态相同,但讲的是真实
主机。

## 0.1 拓扑

```mermaid
flowchart LR
    User["终端用户<br/>(浏览器)"] -- "HTTPS 443" --> Proxy["反向代理<br/>(nginx / Caddy)"]
    Proxy -- "/ " --> FE["Next.js<br/>(standalone Node)"]
    Proxy -- "/api/*" --> API["Rust API<br/>(systemd 单元)"]
    API -- "SQLx" --> DB[("PostgreSQL<br/>(Coolify / 托管)")]
    API -- "HTTPS" --> LLM[("LLM provider<br/>(Unsloth)")]
    API -. "/healthz" .-> HC["健康探针<br/>(k8s / uptime)"]
    FE -. "Prometheus" .-> Mon["指标<br/>(/metrics, 可选)"]
    API -. "audit_events<br/>(仅追加)" .-> Backup[("每日 pg_dump")]
    Backup --> S3["对象存储<br/>(异机)"]
```

| 进程 | 技术 | 端口 | 监管 |
|---|---|---|---|
| 反向代理 | nginx 或 Caddy | `:443` (TLS) | systemd |
| 前端 | Next.js 14 standalone 构建 | `:3000` (仅 loopback) | systemd |
| 后端 | Rust + Actix Web(release 构建) | `:8080` (仅 loopback) | systemd |
| 数据库 | PostgreSQL 14+ | `:5432` | systemd 或托管 |
| LLM provider | OpenAI 兼容 HTTPS | external | — |
| 备份 | `pg_dump` + cron | — | cron 或 systemd-timer |

> **仅绑定到 loopback** — 前端和后端都这样。**只有反向代理**
> 对外暴露。两个进程都从环境读密钥,而不是 `.env`。

## 0.2 主机与端口

| 变量 | 默认 | 说明 |
|---|---|---|
| `HOST` / `BANKPILOT_HOST` | `127.0.0.1` | 后端 bind。**没有**反向代理时再设为 `0.0.0.0`。 |
| `PORT` / `BACKEND_PORT` | `8080` | 后端 HTTP 端口。 |
| `FRONTEND_PORT` | `3000` | Next.js 端口。 |
| `DATABASE_URL` | _(必填)_ | `postgres://USER:PASS@HOST:5432/bankpilot`。 |
| `UNSLOTH_URL` | _(可选)_ | LLM 端点。空 = stub 模式。 |
| `UNSLOTH_API_KEY` | _(可选)_ | Bearer token。 |
| `UNSLOTH_MODEL` | _(可选)_ | 模型名。 |
| `DAILY_LIMIT_FEN` | `5000000`(¥50,000) | 单账户日上限。 |
| `RUST_LOG` | `info` | `env_logger` 过滤器。详细可设 `bankpilot=debug`。 |
| `SESSION_TTL_SECONDS` | `86400` | session 过期,24h。 |
| `PENDING_ACTION_TTL_SECONDS` | `300` | 挂起动作过期,5 分钟。 |

`Settings::unsloth_enabled()` 是唯一检查点:`UNSLOTH_URL`、
`UNSLOTH_API_KEY`、`UNSLOTH_MODEL` 任一为空 ⇒ 正则 stub。代码
库里没有其他地方 import provider SDK。

## 0.3 systemd 单元 — 后端

`/etc/systemd/system/fintech-pioneers-api.service`:

```ini
[Unit]
Description=FinTech Pioneers API (Rust + Actix Web)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=fintech
Group=fintech
WorkingDirectory=/opt/fintech-pioneers/backend
ExecStart=/opt/fintech-pioneers/backend/target/release/bankpilot
EnvironmentFile=/etc/fintech-pioneers/api.env
Restart=on-failure
RestartSec=2s
LimitNOFILE=65536

# 加固
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/fintech-pioneers/backend/.run
PrivateTmp=true

StandardOutput=append:/var/log/fintech-pioneers/api.log
StandardError=append:/var/log/fintech-pioneers/api.log

[Install]
WantedBy=multi-user.target
```

`/etc/fintech-pioneers/api.env`(chmod 640,owner 为 `root:fintech`):

```bash
DATABASE_URL=postgres://fintech:REPLACE_ME@127.0.0.1:5432/bankpilot
HOST=127.0.0.1
PORT=8080
RUST_LOG=info,bankpilot=debug
UNSLOTH_URL=https://api.unsloth.example/v1
UNSLOTH_API_KEY=REPLACE_ME
UNSLOTH_MODEL=unsloth/Meta-Llama-3.1-8B-Instruct
DAILY_LIMIT_FEN=5000000
```

重载并启动:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now fintech-pioneers-api
sudo systemctl status fintech-pioneers-api
sudo journalctl -u fintech-pioneers-api -f
```

## 0.4 systemd 单元 — 前端

`/etc/systemd/system/fintech-pioneers-web.service`:

```ini
[Unit]
Description=FinTech Pioneers Web (Next.js standalone)
After=network-online.target fintech-pioneers-api.service
Wants=network-online.target

[Service]
Type=simple
User=fintech
Group=fintech
WorkingDirectory=/opt/fintech-pioneers/web
ExecStart=/usr/bin/node /opt/fintech-pioneers/web/server.js
EnvironmentFile=/etc/fintech-pioneers/web.env
Restart=on-failure
RestartSec=2s
LimitNOFILE=65536

NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true

StandardOutput=append:/var/log/fintech-pioneers/web.log
StandardError=append:/var/log/fintech-pioneers/web.log

[Install]
WantedBy=multi-user.target
```

`/etc/fintech-pioneers/web.env`:

```bash
PORT=3000
HOSTNAME=127.0.0.1
# Next.js 读 NEXT_PUBLIC_* 是在 build 期,不是运行期。
# 后端 URL 由应用走同源代理,这里留空。
```

每个发布版本在主机上构建一次 standalone 包:

```bash
cd /opt/fintech-pioneers/frontend
pnpm install --frozen-lockfile
pnpm build
# 产物在 .next/standalone/ — 把它复制到 /opt/fintech-pioneers/web
# 并把 .next/static + public 一并复制过去。
```

## 0.5 反向代理(nginx)

`/etc/nginx/sites-available/fintech-pioneers.conf`:

```nginx
upstream fintec_api  { server 127.0.0.1:8080; keepalive 16; }
upstream fintec_web  { server 127.0.0.1:3000; keepalive 16; }

server {
    listen 443 ssl http2;
    server_name  fintec.example;

    ssl_certificate     /etc/letsencrypt/live/fintec.example/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/fintec.example/privkey.pem;
    ssl_protocols       TLSv1.2 TLSv1.3;
    ssl_ciphers         HIGH:!aNULL:!MD5;

    # 加固默认头
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "DENY" always;
    add_header Referrer-Policy "no-referrer" always;

    # 同源 API。CSRF double-submit cookie 由 API 设置。
    location /api/ {
        proxy_pass         http://fintec_api;
        proxy_http_version 1.1;
        proxy_set_header   Connection            "";
        proxy_set_header   Host                  $host;
        proxy_set_header   X-Real-IP             $remote_addr;
        proxy_set_header   X-Forwarded-For       $proxy_add_x_forwarded_for;
        proxy_set_header   X-Forwarded-Proto     $scheme;
        proxy_read_timeout 30s;
        client_max_body_size 4m;
    }

    # 其余全部给 Next.js。
    location / {
        proxy_pass         http://fintec_web;
        proxy_http_version 1.1;
        proxy_set_header   Connection            "";
        proxy_set_header   Host                  $host;
        proxy_set_header   X-Real-IP             $remote_addr;
        proxy_set_header   X-Forwarded-For       $proxy_add_x_forwarded_for;
        proxy_set_header   X-Forwarded-Proto     $scheme;
        proxy_read_timeout 30s;
    }
}

server {
    listen 80;
    server_name fintec.example;
    return 301 https://$host$request_uri;
}
```

重载:`sudo nginx -t && sudo systemctl reload nginx`。

## 0.6 Postgres

后端使用 **SQLx** + **编译期检查的迁移**。迁移文件位于
`backendrust/migrations/`,在 API 启动时自动执行(见
`backendrust/src/main.rs::run_migrations`);通常不需要手动跑
—— 除非你在预热一个全新的 DB。

### 0.6.1 首次设置

```bash
sudo -u postgres psql -c "CREATE USER fintech WITH PASSWORD 'REPLACE_ME';"
sudo -u postgres psql -c "CREATE DATABASE bankpilot OWNER fintech;"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE bankpilot TO fintech;"
# 后端会在首次启动时跑迁移。
sudo systemctl start fintech-pioneers-api
```

### 0.6.2 迁移

文件位于 `backendrust/migrations/`,命名为
`YYYYMMDDHHMMSS_description.sql`。按顺序执行,**幂等**(`IF NOT EXISTS`),
首次接触时还会触发 `seed_if_empty` 创建 demo 数据。

查看已应用的迁移:

```bash
sudo -u postgres psql -d bankpilot -c "SELECT version FROM _sqlx_migrations ORDER BY version;"
```

不重启 API 直接应用新迁移(仅热修复;常规做法是启动 API 让它
在同一事务中执行):

```bash
cd /opt/fintech-pioneers/backend
DATABASE_URL=$DATABASE_URL cargo run --bin bankpilot -- migrate
```

### 0.6.3 备份

底线是每日 `pg_dump` 到异机对象存储。
`/etc/cron.daily/fintech-pioneers-pgdump`:

```bash
#!/usr/bin/env bash
set -euo pipefail
TS=$(date -u +%Y%m%dT%H%M%SZ)
OUT=/var/backups/fintech-pioneers/${TS}.sql.gz
mkdir -p "$(dirname "$OUT")"
pg_dump --no-owner --no-privileges \
    -U fintech -h 127.0.0.1 bankpilot | gzip -9 > "$OUT"
# 推到 S3(或别的)。默认 90 天保留。
aws s3 cp "$OUT" "s3://fintec-backups/postgres/${TS}.sql.gz" \
    --storage-class STANDARD_IA
find /var/backups/fintech-pioneers -type f -mtime +90 -delete
```

对任何持有真实客户数据的环境,都强烈建议**每周全量** + **每日
增量**(WAL 归档)。模拟器的 demo 数据是可重建的 ——
`TRUNCATE … RESTART IDENTITY CASCADE` 后重启即可重新 seed。

### 0.6.4 恢复

```bash
# 先停掉 API,避免恢复期间有写入。
sudo systemctl stop fintech-pioneers-api
# 解压并灌入。
gunzip -c /var/backups/fintech-pioneers/20261018T030000Z.sql.gz \
    | sudo -u postgres psql -d bankpilot
# 若是新库:先 drop + recreate。
sudo systemctl start fintech-pioneers-api
```

> **WAL 重放恢复.** 若启用 `archive_mode = on` 与
> `restore_command`,可以 PITR 到 WAL 保留窗口内的任一秒。详见
> [Postgres 文档 §26.3][pg-pitr]。默认 `pg_dump` 够 demo 用;
> 生产环境请用 PITR。

[pg-pitr]: https://www.postgresql.org/docs/current/continuous-archiving.html

## 0.7 构建与发布

### 0.7.1 后端

```bash
cd /opt/fintech-pioneers/backend
cargo build --release --bin bankpilot
# 产物:target/release/bankpilot
# 复制到部署主机,或直接在它上面 build。
```

要可复现构建,用 `rust-toolchain.toml` 钉住工具链:

```toml
[toolchain]
channel = "1.83.0"
components = ["rustfmt", "clippy"]
```

### 0.7.2 前端

```bash
cd /opt/fintech-pioneers/frontend
pnpm install --frozen-lockfile
pnpm build
# standalone 产物:.next/standalone/
# 把 .next/standalone + .next/static + public 复制到部署主机。
```

`next.config.mjs` 应设置 `output: 'standalone'`。构建后确认
`.next/standalone/server.js` 存在。

### 0.7.3 上线

推荐**蓝绿**切换走反向代理:

1. 把新版本 build 到并行路径
   (`/opt/fintech-pioneers/backend.v2/`)。
2. 在数据库上跑迁移(幂等 —— 已跑过也无害)。
3. 停 `fintech-pioneers-api.v1`,起
   `fintech-pioneers-api.v2`,都指向同一 `DATABASE_URL` 和
   `/var/log/fintech-pioneers/`。
4. 探 `GET /healthz` 确认。返回 `200` 后,把反向代理的
   `upstream fintec_api` 切到新 socket。前端不需要重启,除非
   环境变了。
5. `journalctl -u fintech-pioneers-api -f` 盯 5 分钟。
6. 出问题把 upstream 切回即可。迁移是增量的,所以只要没跑
   破坏性迁移,回滚**对数据安全**。

### 0.7.4 非增量迁移

`IF NOT EXISTS` 约束保证大多数迁移是安全的。要注意的:

- 任何 `DROP COLUMN` / `ALTER TABLE … DROP` —— 必须是两步:
  N 版本弃用,N+1 真正 drop。
- 任何 `RENAME COLUMN` —— 同上:先加新名,后续再删旧列。
- 会破坏 API 契约的 rename —— 与前端协调。

## 0.8 日志轮转

`/etc/logrotate.d/fintech-pioneers`:

```
/var/log/fintech-pioneers/*.log {
    daily
    rotate 14
    compress
    delaycompress
    missingok
    notifempty
    copytruncate
    sharedscripts
    postrotate
        systemctl reload fintech-pioneers-api >/dev/null 2>&1 || true
        systemctl reload fintech-pioneers-web >/dev/null 2>&1 || true
    endscript
}
```

`copytruncate` 对 `actix_web` 和 `next` 都合适 —— 二者都不会
在 SIGHUP 时重新打开日志文件。

## 0.9 可观测性

本发布有意保持指标轻量。最低线是:

- `GET /healthz` —— 存活探针。进程在线 + SQLx 能 `SELECT 1`
  时返回 `200`。
- 日志写 journald(`StandardOutput=append:`)并轮转成 `*.log`。
- `audit_events` 表是"对谁发生了什么"的唯一事实来源。查最近
  24h `risk_tier = 'Red'` 的事件数,这就是最接近业务指标的东西。

如果要接 Prometheus,自然计数器都在
`backendrust/src/audit.rs` 和 `routes/agent.rs` —— 加一个
`/metrics` 路由,暴露
`audit_events_total{risk_tier,result}`、
`pending_actions_total{status}`、
`llm_proposal_total{source}`(provider vs stub)。

## 0.10 灾备(RPO / RTO)

对 demo / pilot 环境,目标是:

- **RPO**:24h(漏一次备份窗口)。`pg_dump` 每日。
- **RTO**:~1h(开新机、装 Rust 工具链、build、灌 dump、恢复、
  切 DNS 的总时间)。

生产级环境,改用 WAL 归档 + PITR(`archive_mode = on`、
`archive_command`、基础备份 + 恢复到任一时间点),目标:

- **RPO**:5 分钟。
- **RTO**:30 分钟。

## 0.11 安全加固清单

- [ ] 两个进程都只 bind 到 `127.0.0.1`。
- [ ] TLS 在反向代理终止;启用 HSTS。
- [ ] `DATABASE_URL` 与 `UNSLOTH_API_KEY` 放在
  `/etc/fintech-pioneers/api.env`,mode `0640`,owner
  `root:fintech`。
- [ ] `firewall` 仅允许公网 `:443`;DB 与 LLM 仅 loopback 或
  私有子网可达。
- [ ] 单元文件里设了 `NoNewPrivileges`、`ProtectSystem=strict`、
  `PrivateTmp`。
- [ ] 非 root `fintech` 用户拥有安装目录。
- [ ] `fail2ban` 监控 nginx 4xx/5xx 突增(可选)。
- [ ] `audit_events` 在 DB 角色层面**仅追加**:迁移跑完后
  `REVOKE UPDATE, DELETE ON audit_events FROM fintech;`。
  后端角色不需要 update / delete 行;它只需要 `INSERT` 与
  `SELECT`。

## 0.12 接下来读哪里

- [01-troubleshooting.md](./01-troubleshooting.md) — 着火时。
- [02-test-cases.md](./02-test-cases.md) — 覆盖了什么、没
  覆盖什么。
- [03-ci.md](./03-ci.md) — 推荐的 CI 流水线。
- [../architecture/05-deployment.md](../architecture/05-deployment.md) — 开发快速上手与日志布局。
