#!/usr/bin/env bash
# Quick-start both dev servers in one terminal.
#
#   - Rust backend on :8080  (from `backendrust/`, release build)
#   - Next.js frontend on :3000 (from `frontend/`, dev mode)
#
# Both processes are backgrounded; their logs go to `./.run/rust-backend.log`
# and `./.run/frontend.log` so you can `tail -f` either. The script traps
# Ctrl-C and kills both children before exiting.
#
# Usage:
#   ./ops/dev.sh              # start both
#   ./ops/dev.sh --build      # cargo build --release first (slower, then runs)
#   ./ops/dev.sh --no-frontend
#   ./ops/dev.sh --no-backend
#   PORT=9090 ./ops/dev.sh    # override frontend port
#
# Env (auto-loaded from `.env` if present):
#   DATABASE_URL   Postgres URL (default: from .env)
#   BACKEND_PORT   8080
#   FRONTEND_PORT  3000
#   BANKPILOT_HOST 0.0.0.0

set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$(pwd)"

# --- Args ----------------------------------------------------------------
BUILD=0
WITH_FRONTEND=1
WITH_BACKEND=1
for arg in "$@"; do
  case "$arg" in
    --build)        BUILD=1 ;;
    --no-frontend)  WITH_FRONTEND=0 ;;
    --no-backend)   WITH_BACKEND=0 ;;
    -h|--help)
      sed -n '2,18p' "$0"; exit 0 ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

# --- Env -----------------------------------------------------------------
# Use a strict assignment-only loader so a stray line in `.env` (e.g. an
# unquoted space that turns `UNSLOTH_URL=…` into a shell command) can't
# run anything. We only honour lines whose left-hand side is a valid
# variable name; everything else (comments, blank lines, accidental
# command-shaped text) is silently ignored.
if [[ -f .env ]]; then
  while IFS= read -r line; do
    # Trim leading whitespace, drop comments / blanks.
    line="${line#"${line%%[![:space:]]*}"}"
    [[ -z "$line" || "$line" =~ ^# ]] && continue
    # Only accept strict KEY=VALUE (no shell metacharacters in the name).
    if [[ "$line" =~ ^([A-Za-z_][A-Za-z0-9_]*)=(.*)$ ]]; then
      key="${BASH_REMATCH[1]}"
      val="${BASH_REMATCH[2]}"
      # Strip optional surrounding quotes.
      if [[ "$val" =~ ^\"(.*)\"$ ]] || [[ "$val" =~ ^\'(.*)\'$ ]]; then
        val="${BASH_REMATCH[1]}"
      fi
      export "$key"="$val"
    fi
    # Lines that don't match the assignment form are dropped on purpose.
  done < .env
fi
BACKEND_PORT="${BACKEND_PORT:-8080}"
FRONTEND_PORT="${FRONTEND_PORT:-3000}"
BANKPILOT_HOST="${BANKPILOT_HOST:-0.0.0.0}"

# Sanity: .env defaults point at a remote Postgres. If DATABASE_URL is
# unset, fall back to the local server the seed script expects.
: "${DATABASE_URL:=postgres://postgres@127.0.0.1:5432/bankpilot}"

mkdir -p .run

# --- Stop any prior runs on our ports (idempotent re-runs) --------------
stop_listeners() {
  local port="$1"
  local pids
  pids="$(ss -tnlp 2>/dev/null | awk -v p=":$port" '$4 ~ p {print $0}' | grep -oE 'pid=[0-9]+' | cut -d= -f2 | sort -u || true)"
  for pid in $pids; do
    echo "  killing pid $pid (was on :$port)"
    kill "$pid" 2>/dev/null || true
  done
  sleep 1
}

PIDS=()
cleanup() {
  echo
  echo "==> Stopping (PIDs: ${PIDS[*]:-none})"
  for pid in "${PIDS[@]:-}"; do
    kill "$pid" 2>/dev/null || true
  done
  wait 2>/dev/null || true
  echo "done."
}
trap cleanup INT TERM EXIT

# --- Backend -------------------------------------------------------------
if (( WITH_BACKEND )); then
  echo "==> Stopping any prior backend on :$BACKEND_PORT"
  stop_listeners "$BACKEND_PORT"

  if (( BUILD )); then
    echo "==> Building Rust backend (release) — first run takes a while"
    ( cd "$ROOT/backendrust" && cargo build --release --bin bankpilot )
    BIN="$ROOT/backendrust/target/release/bankpilot"
  else
    # If the debug binary doesn't exist yet, do a debug build (much faster
    # than release and good enough for dev).
    if [[ ! -x "$ROOT/backendrust/target/debug/bankpilot" ]]; then
      echo "==> Building Rust backend (debug, one-time)"
      ( cd "$ROOT/backendrust" && cargo build --bin bankpilot )
    fi
    BIN="$ROOT/backendrust/target/debug/bankpilot"
  fi

  echo "==> Starting Rust backend on $BANKPILOT_HOST:$BACKEND_PORT"
  (
    cd "$ROOT/backendrust"
    DATABASE_URL="$DATABASE_URL" \
    HOST="$BANKPILOT_HOST" PORT="$BACKEND_PORT" \
      "$BIN" > "$ROOT/.run/rust-backend.log" 2>&1
  ) &
  PIDS+=("$!")
  echo "    pid=${PIDS[-1]}    log=$ROOT/.run/rust-backend.log"
fi

# --- Frontend ------------------------------------------------------------
if (( WITH_FRONTEND )); then
  echo "==> Stopping any prior frontend on :$FRONTEND_PORT"
  stop_listeners "$FRONTEND_PORT"

  # The project pins Next via pnpm. We try a few common entry points so
  # the script works whether the user installed with pnpm or npm.
  FRONT="$ROOT/frontend"
  cd "$FRONT"

  if [[ -x node_modules/.bin/next ]]; then
    NEXT_BIN=./node_modules/.bin/next
  elif command -v pnpm >/dev/null 2>&1; then
    NEXT_BIN="pnpm next"
  else
    echo "next not found; falling back to npx" >&2
    NEXT_BIN="npx --no-install next"
  fi

  echo "==> Starting Next.js on :$FRONTEND_PORT"
  (
    cd "$FRONT"
    PORT="$FRONTEND_PORT" \
      $NEXT_BIN dev > "$ROOT/.run/frontend.log" 2>&1
  ) &
  PIDS+=("$!")
  echo "    pid=${PIDS[-1]}    log=$ROOT/.run/frontend.log"
  cd "$ROOT"
fi

# --- Wait & health check -------------------------------------------------
echo
echo "==> Waiting for both to be ready..."
for i in {1..60}; do
  ready=0
  if (( WITH_BACKEND )); then
    code="$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$BACKEND_PORT/healthz" || echo 000)"
    [[ "$code" == "200" ]] && ready=$((ready+1))
  fi
  if (( WITH_FRONTEND )); then
    code="$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$FRONTEND_PORT/" || echo 000)"
    [[ "$code" == "200" ]] && ready=$((ready+1))
  fi
  total=$(( WITH_BACKEND + WITH_FRONTEND ))
  if (( ready == total )); then
    break
  fi
  sleep 1
done

echo
echo "==> BankPilot dev stack is up:"
(( WITH_BACKEND  )) && echo "    backend  http://127.0.0.1:$BACKEND_PORT  (pid ${PIDS[0]:-?})"
(( WITH_FRONTEND )) && echo "    frontend http://127.0.0.1:$FRONTEND_PORT  (pid ${PIDS[1]:-?})"
echo
echo "    demo login: demo / demo1234"
echo
echo "Press Ctrl-C to stop both. Tailing combined output..."
echo "----------------------------------------------------------------"

# Tail both logs in the foreground; the trap will kill the children on exit.
LOGS=()
(( WITH_BACKEND  )) && LOGS+=("$ROOT/.run/rust-backend.log")
(( WITH_FRONTEND )) && LOGS+=("$ROOT/.run/frontend.log")
tail -n +1 -F "${LOGS[@]}"
