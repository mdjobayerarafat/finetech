import os
files = [
    '/home/jobayer/Documents/bankpilot/backendrust/src/tools.rs',
    '/home/jobayer/Documents/bankpilot/backendrust/src/sessions.rs',
    '/home/jobayer/Documents/bankpilot/backendrust/src/audit.rs',
]
for p in files:
    with open(p) as fh:
        t = fh.read()
    t = t.replace('use sqlx::SqlitePool;', 'use sqlx::PgPool;')
    t = t.replace('use sqlx::{Row, SqlitePool};', 'use sqlx::{PgPool, Row};')
    t = t.replace('pool: &SqlitePool', 'pool: &PgPool')
    t = t.replace('pool: &sqlx::SqlitePool', 'pool: &sqlx::PgPool')
    with open(p, 'w') as fh:
        fh.write(t)
print('done')
