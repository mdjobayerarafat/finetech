---
id: ops-deployment
title: "0 · Deployment"
sidebar_label: "0 · Deployment"
sidebar_position: 1
---

# 0. Deployment

> 中文: [00-部署 (中文)](./zh/00-deployment.md)

This page is the **production operations** reference: how to
stand up the system, what to put behind it, how to migrate the
database, how to back it up, how to upgrade, and how to roll it
back. The dev quickstart is in
[`../architecture/05-deployment.md`](../architecture/05-deployment.md)
— start there for `./ops/dev.sh`. This page is the same shape
on a real host.

## 0.1 Topology

```mermaid
flowchart LR
    User["End user<br/>(browser)"] -- "HTTPS 443" --> Proxy["Reverse proxy<br/>(nginx / Caddy)"]
    Proxy -- "/ " --> FE["Next.js<br/>(standalone Node)"]
    Proxy -- "/api/*" --> API["Rust API<br/>(systemd unit)"]
    API -- "SQLx" --> DB[("PostgreSQL<br/>(Coolify / managed)")]
    API -- "HTTPS" --> LLM[("LLM provider<br/>(Unsloth)")]
    API -. "/healthz" .-> HC["Health probe<br/>(k8s / uptime)"]
    FE -. "Prometheus" .-> Mon["Metrics<br/>(/metrics, optional)"]
    API -. "audit_events<br/>(append-only)" .-> Backup[("Daily pg_dump")]
    Backup --> S3["Object storage<br/>(off-host)"]
```

| Process | Tech | Port | Supervisor |
|---|---|---|---|
| Reverse proxy | nginx or Caddy | `:443` (TLS) | systemd |
| Frontend | Next.js 14 standalone build | `:3000` (loopback only) | systemd |
| Backend | Rust + Actix Web (release build) | `:8080` (loopback only) | systemd |
| Database | PostgreSQL 14+ | `:5432` | systemd or managed |
| LLM provider | OpenAI-compatible HTTPS | external | — |
| Backups | `pg_dump` + cron | — | cron or systemd-timer |

> **Bind only to loopback** for the frontend and backend. The
> reverse proxy is the only thing exposed to the network. Both
> processes read secrets from the environment, not `.env`.

## 0.2 Hosts and ports

| Variable | Default | Notes |
|---|---|---|
| `HOST` / `BANKPILOT_HOST` | `127.0.0.1` | Backend bind. Set to `0.0.0.0` only if there is **no** reverse proxy. |
| `PORT` / `BACKEND_PORT` | `8080` | Backend HTTP port. |
| `FRONTEND_PORT` | `3000` | Next.js port. |
| `DATABASE_URL` | _(required)_ | `postgres://USER:PASS@HOST:5432/bankpilot`. |
| `UNSLOTH_URL` | _(optional)_ | LLM endpoint. Empty = stub mode. |
| `UNSLOTH_API_KEY` | _(optional)_ | Bearer token. |
| `UNSLOTH_MODEL` | _(optional)_ | Model name. |
| `DAILY_LIMIT_FEN` | `5000000` (¥50,000) | Single-account daily cap. |
| `RUST_LOG` | `info` | `env_logger` filter. Use `bankpilot=debug` for verbose. |
| `SESSION_TTL_SECONDS` | `86400` | Session expiry, 24h. |
| `PENDING_ACTION_TTL_SECONDS` | `300` | Pending action expiry, 5 min. |

`Settings::unsloth_enabled()` is the single check: any of
`UNSLOTH_URL`, `UNSLOTH_API_KEY`, `UNSLOTH_MODEL` empty ⇒ regex
stub. Nothing else imports the provider SDK.

## 0.3 systemd unit — backend

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

# Hardening
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

`/etc/fintech-pioneers/api.env` (chmod 640, owned by
`root:fintech`):

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

Reload + start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now fintech-pioneers-api
sudo systemctl status fintech-pioneers-api
sudo journalctl -u fintech-pioneers-api -f
```

## 0.4 systemd unit — frontend

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
# Next.js reads NEXT_PUBLIC_* at build time, not runtime.
# For backend URL the app uses a same-origin proxy — leave this empty.
```

Build the standalone bundle on the host once per release:

```bash
cd /opt/fintech-pioneers/frontend
pnpm install --frozen-lockfile
pnpm build
# Output goes to .next/standalone — copy that to /opt/fintech-pioneers/web
# and copy .next/static + public alongside it.
```

## 0.5 Reverse proxy (nginx)

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

    # Tighten defaults
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "DENY" always;
    add_header Referrer-Policy "no-referrer" always;

    # Same-origin API. The CSRF double-submit cookie is set by the API.
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

    # Everything else to Next.js.
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

Reload: `sudo nginx -t && sudo systemctl reload nginx`.

## 0.6 Postgres

The backend uses **SQLx** with **compile-time-checked
migrations**. Migrations are run automatically at backend start
(see `backendrust/src/main.rs::run_migrations`); you don't need
to call them manually unless you're pre-warming a fresh DB.

### 0.6.1 First-time setup

```bash
sudo -u postgres psql -c "CREATE USER fintech WITH PASSWORD 'REPLACE_ME';"
sudo -u postgres psql -c "CREATE DATABASE bankpilot OWNER fintech;"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE bankpilot TO fintech;"
# The backend will run the migrations on first start.
sudo systemctl start fintech-pioneers-api
```

### 0.6.2 Migrations

Files are in `backendrust/migrations/`, named
`YYYYMMDDHHMMSS_description.sql`. They run in order, are
**idempotent** (`IF NOT EXISTS`), and on first contact they also
trigger `seed_if_empty` to create the demo data.

To check what's been applied:

```bash
sudo -u postgres psql -d bankpilot -c "SELECT version FROM _sqlx_migrations ORDER BY version;"
```

To apply a new migration without restarting the API (only for
hotfixes — the normal path is to start the API and let it run
them in the same transaction):

```bash
cd /opt/fintech-pioneers/backend
DATABASE_URL=$DATABASE_URL cargo run --bin bankpilot -- migrate
```

### 0.6.3 Backups

Daily `pg_dump` to off-host object storage is the minimum bar.
`/etc/cron.daily/fintech-pioneers-pgdump`:

```bash
#!/usr/bin/env bash
set -euo pipefail
TS=$(date -u +%Y%m%dT%H%M%SZ)
OUT=/var/backups/fintech-pioneers/${TS}.sql.gz
mkdir -p "$(dirname "$OUT")"
pg_dump --no-owner --no-privileges \
    -U fintech -h 127.0.0.1 bankpilot | gzip -9 > "$OUT"
# Push to S3 (or whatever). 90-day retention is the default.
aws s3 cp "$OUT" "s3://fintec-backups/postgres/${TS}.sql.gz" \
    --storage-class STANDARD_IA
find /var/backups/fintech-pioneers -type f -mtime +90 -delete
```

A weekly **full** + daily **incremental** (WAL archiving) is
strongly recommended for any environment that holds real
customer data. The simulator demo data is replaceable — a
`TRUNCATE … RESTART IDENTITY CASCADE` followed by a restart
re-seeds it.

### 0.6.4 Restore

```bash
# Stop the API so nothing writes during restore.
sudo systemctl stop fintech-pioneers-api
# Decompress and feed in.
gunzip -c /var/backups/fintech-pioneers/20261018T030000Z.sql.gz \
    | sudo -u postgres psql -d bankpilot
# If the target DB is fresh: drop and recreate first.
sudo systemctl start fintech-pioneers-api
```

> **WAL-replay restore.** If you run `archive_mode = on` with a
> `restore_command`, you can PITR to any second in the WAL
> retention window. See [Postgres docs §26.3][pg-pitr]. The
> default `pg_dump` setup is enough for the demo; PITR is what
> you want for production.

[pg-pitr]: https://www.postgresql.org/docs/current/continuous-archiving.html

## 0.7 Build and release

### 0.7.1 Backend

```bash
# In the backend crate, on the build host.
cd /opt/fintech-pioneers/backend
cargo build --release --bin bankpilot
# Artifact: target/release/bankpilot
# Copy to the deploy host or build directly on it.
```

For a reproducible build, pin the toolchain with
`rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.83.0"
components = ["rustfmt", "clippy"]
```

### 0.7.2 Frontend

```bash
cd /opt/fintech-pioneers/frontend
pnpm install --frozen-lockfile
pnpm build
# Standalone output: .next/standalone/
# Copy .next/standalone + .next/static + public to the deploy host.
```

`next.config.mjs` should set `output: 'standalone'`. Verify
after build that `server.js` exists in `.next/standalone/`.

### 0.7.3 Rollout

The recommended path is a **blue/green** swap behind the reverse
proxy:

1. Build the new release to a parallel path
   (`/opt/fintech-pioneers/backend.v2/`).
2. Run migrations on the database (idempotent — no harm if
   already applied).
3. Stop `fintech-pioneers-api.v1`, start
   `fintech-pioneers-api.v2`, both pointing at the same
   `DATABASE_URL` and `/var/log/fintech-pioneers/`.
4. Hit `GET /healthz` to confirm. If it returns `200`, switch
   the reverse proxy's `upstream fintec_api` to the new socket.
   The frontend doesn't need a restart unless its env changed.
5. Watch `journalctl -u fintech-pioneers-api -f` for 5 minutes.
6. If anything goes wrong, swap the upstream back. Migrations
   are additive, so rollback is **data-safe** as long as no
   destructive migration ran.

### 0.7.4 Schema migrations that are not additive

The `IF NOT EXISTS` discipline keeps most migrations safe. The
ones to watch:

- Any `DROP COLUMN` or `ALTER TABLE … DROP` — must be a two-step:
  deprecate in release N, drop in release N+1.
- Any `RENAME COLUMN` — same: new name first, old column
  removed later.
- Renames that break the API contract — coordinate with the
  frontend.

## 0.8 Log rotation

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

`copytruncate` is the right choice for `actix_web` and `next`
because neither reopens log files on SIGHUP.

## 0.9 Observability

The shipped build is intentionally light on metrics. The
minimum bar is:

- `GET /healthz` — liveness probe. Returns `200` if the process
  is up and SQLx can `SELECT 1`.
- Logs to journald (`StandardOutput=append:`) and rotated
  `*.log` files.
- The `audit_events` table is the single source of truth for
  *what happened to whom*. An admin query for the last 24h of
  `risk_tier = 'Red'` events is the closest thing to a business
  metric this build ships.

If you wire Prometheus, the natural counters are in
`backendrust/src/audit.rs` and `routes/agent.rs` — add a
`/metrics` route, expose `audit_events_total{risk_tier,result}`,
`pending_actions_total{status}`, and `llm_proposal_total{source}`
(provider vs stub).

## 0.10 Disaster recovery (RPO / RTO)

For a demo / pilot deployment, the target is:

- **RPO**: 24h (one missed backup window). `pg_dump` daily.
- **RTO**: ~1h (time to spin up a new host, install Rust
  toolchain, build, copy dump, restore, swap DNS).

For a production-grade deployment, switch to WAL archiving +
PITR (`archive_mode = on`, `archive_command`, base backup +
restore to point in time) and target:

- **RPO**: 5 min.
- **RTO**: 30 min.

## 0.11 Security hardening checklist

- [ ] Both processes bind to `127.0.0.1` only.
- [ ] TLS terminated at the reverse proxy; HSTS enabled.
- [ ] `DATABASE_URL` and `UNSLOTH_API_KEY` are in
  `/etc/fintech-pioneers/api.env` with mode `0640`, owner
  `root:fintech`.
- [ ] `firewall` allows only `:443` from the public network; the
  DB and LLM are reachable on loopback or private subnet only.
- [ ] `NoNewPrivileges`, `ProtectSystem=strict`, `PrivateTmp`
  are set in the unit files.
- [ ] A non-root `fintech` user owns the install tree.
- [ ] `fail2ban` on `nginx` 4xx/5xx bursts (optional).
- [ ] The `audit_events` table is **append-only** at the DB
  role level: `REVOKE UPDATE, DELETE ON audit_events FROM
  fintech;` after the migration runs. The backend's role
  doesn't need to update or delete rows; the role only needs
  `INSERT` and `SELECT`.

## 0.12 Where to read next

- [01-troubleshooting.md](./01-troubleshooting.md) — when
  something is on fire.
- [02-test-cases.md](./02-test-cases.md) — what's covered and
  what isn't.
- [03-ci.md](./03-ci.md) — recommended CI pipeline.
- [../architecture/05-deployment.md](../architecture/05-deployment.md) — dev quickstart and log layout.
