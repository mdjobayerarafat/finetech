import os
files = [
    '/home/jobayer/Documents/bankpilot/backendrust/src/config.rs',
    '/home/jobayer/Documents/bankpilot/backendrust/src/ctx.rs',
    '/home/jobayer/Documents/bankpilot/backendrust/src/safety.rs',
]
for p in files:
    with open(p) as fh:
        t = fh.read()
    t = t.replace('use sqlx::SqlitePool;', 'use sqlx::PgPool;')
    t = t.replace('/// SQLAlchemy-style URL. For SQLite: `sqlite:////abs/path.db` or `sqlite:./file.db`.',
                  '/// PostgreSQL connection URL, e.g. `postgres://user:pass@host:port/db`.')
    t = t.replace("env_or(\"DATABASE_URL\", \"sqlite://./bankpilot.db\")",
                  "env_or(\"DATABASE_URL\", \"postgres://postgres:postgres@127.0.0.1:5432/postgres\")")
    with open(p, 'w') as fh:
        fh.write(t)
print('done')
