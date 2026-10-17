import os
routes = '/home/jobayer/Documents/bankpilot/backendrust/src/routes'
for f in os.listdir(routes):
    p = os.path.join(routes, f)
    if not f.endswith('.rs'):
        continue
    with open(p) as fh:
        t = fh.read()
    t = t.replace('use sqlx::SqlitePool;', 'use sqlx::PgPool;')
    t = t.replace('web::Data<SqlitePool>', 'web::Data<PgPool>')
    t = t.replace('&SqlitePool', '&PgPool')
    t = t.replace('&sqlx::SqlitePool', '&sqlx::PgPool')
    with open(p, 'w') as fh:
        fh.write(t)
print('done')
