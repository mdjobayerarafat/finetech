#!/usr/bin/env bash
# Run the mockbank Go service. Reads .env from this directory if present.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -f .env ]; then
  set -a
  # shellcheck disable=SC1091
  . ./.env
  set +a
fi

if ! command -v go >/dev/null 2>&1; then
  echo "go not found in PATH. Install Go 1.22+ and try again." >&2
  exit 1
fi

if [ ! -d vendor ] && [ -f go.mod ]; then
  go mod download
fi

exec go run ./cmd/server
