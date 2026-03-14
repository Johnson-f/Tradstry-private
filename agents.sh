#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVICE_DIR="$ROOT_DIR/backend/agents_service"

cd "$SERVICE_DIR"

export PYTHONPATH="src${PYTHONPATH:+:$PYTHONPATH}"

exec .venv/bin/uvicorn main:app \
  --app-dir src \
  --host "${AGENTS_SERVICE_HOST:-0.0.0.0}" \
  --port "${AGENTS_SERVICE_PORT:-8091}"
