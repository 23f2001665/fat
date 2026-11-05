#!/usr/bin/env bash
# Build the Vue app when needed, start Flask if it is not already running, and
# open a Cloudflare quick tunnel to the local tracker.
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v cloudflared >/dev/null 2>&1; then
  echo "cloudflared is required: https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/" >&2
  exit 1
fi

if ! command -v uv >/dev/null 2>&1; then
  echo "uv is required: https://docs.astral.sh/uv/" >&2
  exit 1
fi

if [ "${FAT_SKIP_BUILD:-0}" != "1" ] && [ ! -f frontend/dist/index.html ]; then
  if ! command -v pnpm >/dev/null 2>&1; then
    echo "pnpm is required to build the frontend." >&2
    exit 1
  fi
  echo "Building the Vue frontend..."
  pnpm --dir frontend build
fi

port="${FAT_PORT:-5000}"
base_url="http://127.0.0.1:${port}"
backend_pid=""

cleanup() {
  if [ -n "$backend_pid" ] && kill -0 "$backend_pid" 2>/dev/null; then
    kill "$backend_pid" 2>/dev/null || true
    wait "$backend_pid" 2>/dev/null || true
  fi
}

trap cleanup EXIT INT TERM

if curl -fsS "${base_url}/api/health" >/dev/null 2>&1; then
  echo "Using the Flask server already running at ${base_url}."
else
  echo "Starting Flask on ${base_url} ..."
  FAT_PORT="${port}" uv run python -m backend.app &
  backend_pid=$!

  for _ in $(seq 1 60); do
    if curl -fsS "${base_url}/api/health" >/dev/null 2>&1; then
      break
    fi
    if ! kill -0 "$backend_pid" 2>/dev/null; then
      echo "The Flask server stopped before becoming ready." >&2
      exit 1
    fi
    sleep 0.25
  done

  if ! curl -fsS "${base_url}/api/health" >/dev/null 2>&1; then
    echo "The Flask server did not become healthy." >&2
    exit 1
  fi
fi

echo
echo "Opening a Cloudflare quick tunnel to ${base_url}."
echo "Look for the https://<random>.trycloudflare.com URL below."
echo "The URL is public and stays active only while this command runs."
echo

cloudflared tunnel --url "${base_url}"
