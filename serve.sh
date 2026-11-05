#!/usr/bin/env bash
# Build the Vue app and serve it through Flask.
set -euo pipefail

cd "$(dirname "$0")"

if ! command -v pnpm >/dev/null 2>&1; then
  echo "pnpm is required: https://pnpm.io/" >&2
  exit 1
fi

pnpm --dir frontend build
exec uv run python -m backend.app
