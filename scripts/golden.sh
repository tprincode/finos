#!/usr/bin/env bash
# Thin Mac wrapper around cargo test --test <suite>. Prefer scripts/golden.ps1 on Windows
# (that wrapper records durations). Here we only announce and run.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <suite> [suite…]" >&2
  exit 1
fi

for suite in "$@"; do
  echo "running golden: $suite"
  start="$(date +%s)"
  cargo test --test "$suite" -- --nocapture
  end="$(date +%s)"
  echo "the $suite golden took $((end - start)) seconds"
done
