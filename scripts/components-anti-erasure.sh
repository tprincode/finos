#!/usr/bin/env bash
# Phase 1 anti-erasure gate (finos-reset-plan §1.1).
# Runs ui_modules + core_functions so Tools → Components / catalogs cannot
# fail open on PRs that touch desktop UI or architecture module catalogs.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "components-anti-erasure: cargo test -p golden-harness --test ui_modules --test core_functions"
cargo test -p golden-harness --test ui_modules --test core_functions
echo "components-anti-erasure: ok"
