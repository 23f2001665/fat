#!/usr/bin/env bash
# Run the Flask API and Vite dev server together.
set -euo pipefail

cd "$(dirname "$0")"

if ! command -v uv >/dev/null 2>&1; then
  echo "uv is required: https://docs.astral.sh/uv/" >&2
  exit 1
fi

if ! command -v pnpm >/dev/null 2>&1; then
  echo "pnpm is required: https://pnpm.io/" >&2
  exit 1
fi

if [ ! -d frontend/node_modules ]; then
  echo "Installing frontend dependencies..."
  pnpm --dir frontend install
fi

backend_pid=""

cleanup() {
  if [ -n "$backend_pid" ] && kill -0 "$backend_pid" 2>/dev/null; then
    kill "$backend_pid" 2>/dev/null || true
    wait "$backend_pid" 2>/dev/null || true
  fi
}

trap cleanup EXIT INT TERM

echo "Starting Flask on http://127.0.0.1:5000 ..."
uv run python -m backend.app &
backend_pid=$!

for _ in $(seq 1 60); do
  if curl -fsS http://127.0.0.1:5000/api/health >/dev/null 2>&1; then
    break
  fi
  if ! kill -0 "$backend_pid" 2>/dev/null; then
    echo "The Flask server stopped before becoming ready." >&2
    exit 1
  fi
  sleep 0.25
done

if ! curl -fsS http://127.0.0.1:5000/api/health >/dev/null 2>&1; then
  echo "The Flask server did not become healthy on port 5000." >&2
  exit 1
fi

echo "Starting Vite on http://127.0.0.1:5173 ..."
pnpm --dir frontend dev
