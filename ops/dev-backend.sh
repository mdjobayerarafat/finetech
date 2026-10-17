#!/usr/bin/env bash
# Run the BankPilot backend in development mode.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.local/bin:$PATH"
export PYTHONPATH="$(pwd)/backend${PYTHONPATH:+:$PYTHONPATH}"
cd backend
exec uvicorn app.main:app --host "${HOST:-0.0.0.0}" --port "${PORT:-8080}" --log-level info
