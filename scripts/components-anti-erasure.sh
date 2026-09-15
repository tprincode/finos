#!/usr/bin/env bash
# Phase 1 anti-erasure gate (finos-reset-plan §1.1).
# Runs ui_modules + core_functions so Tools → Components / catalogs cannot
# fail open on PRs that touch desktop UI or architecture module catalogs.
#
# Live ui_modules checks need Profile A SQLite via profile_a_app_dir():
# LOCALAPPDATA/com.finos.desktop/local.sqlite (or ./com.finos.desktop when unset).
# Always set an absolute LOCALAPPDATA and seed before tests — cargo test CWD is
# the crate dir, so a relative "." path fails in CI.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export LOCALAPPDATA="${LOCALAPPDATA:-$ROOT}"
APP_DIR="$LOCALAPPDATA/com.finos.desktop"
mkdir -p "$APP_DIR"

echo "components-anti-erasure: LOCALAPPDATA=$LOCALAPPDATA"
echo "components-anti-erasure: seeding Profile A at $APP_DIR (npm run data-seed / data-seed bin)"
if command -v npm >/dev/null 2>&1 && [[ -f "$ROOT/package.json" ]]; then
  npm run data-seed
else
  cargo run -p golden-harness --bin data-seed
fi

if [[ ! -f "$APP_DIR/local.sqlite" ]]; then
  echo "components-anti-erasure: missing $APP_DIR/local.sqlite after data-seed" >&2
  exit 1
fi

echo "components-anti-erasure: cargo test -p golden-harness --test ui_modules --test core_functions"
cargo test -p golden-harness --test ui_modules --test core_functions
echo "components-anti-erasure: ok"
