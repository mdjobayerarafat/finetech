#!/usr/bin/env bash
# Seed (or reset) the demo database.
# Usage: ./ops/seed.sh          # idempotent seed
#        ./ops/seed.sh --reset  # wipe + reseed
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.local/bin:$PATH"
export PYTHONPATH="$(pwd)/backend${PYTHONPATH:+:$PYTHONPATH}"
exec python3 -m app.seed "$@"
