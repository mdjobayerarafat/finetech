#!/usr/bin/env bash
# Wipe the mockbank schema and let the server re-seed on next boot.
# Run with the same DATABASE_URL you use for the service.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -f .env ]; then
  set -a
  # shellcheck disable=SC1091
  . ./.env
  set +a
fi

: "${DATABASE_URL:?DATABASE_URL is required}"

echo "dropping schema mockbank ..."
psql "$DATABASE_URL" -c "DROP SCHEMA IF EXISTS mockbank CASCADE;"
echo "done. restart the service to re-apply schema + seed."
