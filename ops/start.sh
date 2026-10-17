#!/usr/bin/env bash
# One-shot: install deps (if needed), seed, then start the backend.
# Idempotent. Use this for the demo.
set -euo pipefail
cd "$(dirname "$0")/.."

# 1) Ensure Python deps are present.
export PATH="$HOME/.local/bin:$PATH"
if ! ~/.local/bin/pip show fastapi >/dev/null 2>&1; then
  echo "==> Installing Python dependencies (first run only)..."
  ~/.local/bin/pip install --user --break-system-packages --only-binary=:all: \
    -r backend/requirements.txt
fi

# 2) Seed (idempotent — does nothing if already seeded).
echo "==> Seeding demo data..."
./ops/seed.sh

# 3) Start the backend.
echo "==> Starting backend on :${PORT:-8080}..."
exec ./ops/dev-backend.sh
