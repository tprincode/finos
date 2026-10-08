#!/usr/bin/env bash
# Mac / Unix coding launch — counterpart to finos.bat.
# Opens the Tauri desktop with npm workspaces. Prefer this from Cursor on macOS.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

if ! command -v node >/dev/null 2>&1; then
  echo "Node.js is required (install an LTS release)." >&2
  exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust/cargo is required (install via rustup)." >&2
  exit 1
fi

# Locked Mac kit defaults (override before calling if needed).
export FINOS_DOWNLOAD_DIR="${FINOS_DOWNLOAD_DIR:-$HOME/Documents/Financial}"
# App data: leave unset so the Tauri host sets FINOS_APP_DATA from app_local_data_dir.
# Seed/tools before first launch can set:
#   export FINOS_APP_DATA="$HOME/Library/Application Support/com.finos.desktop"

mkdir -p "$FINOS_DOWNLOAD_DIR"

if [[ ! -d node_modules ]]; then
  echo "npm install (first run)…"
  npm install
fi

# Free prior desktop if still running (best-effort).
if command -v pkill >/dev/null 2>&1; then
  pkill -f 'finos-desktop' 2>/dev/null || true
fi

echo "starting finos desktop (npm run desktop)…"
exec npm run desktop
