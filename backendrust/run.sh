#!/usr/bin/env bash
# Convenience launcher: build + run the Rust backend.
set -euo pipefail
cd "$(dirname "$0")"
exec cargo run --release --bin bankpilot
