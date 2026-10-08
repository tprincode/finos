#!/usr/bin/env bash
# Soft Mac counterpart to scripts/app-up.ps1.
# Passes when: at least one finos-desktop process, localhost:1420 answers,
# and recent page-loaded / console log has no panic / Failed to setup / npm error.
set -euo pipefail

APP_DATA="${FINOS_APP_DATA:-$HOME/Library/Application Support/com.finos.desktop}"
CONSOLE="$APP_DATA/dev-console.log"
PAGE="$APP_DATA/page-loaded.log"

count="$(pgrep -fl 'finos-desktop' 2>/dev/null | wc -l | tr -d ' ')"
echo "finos-desktop count: $count"
if [[ "$count" -lt 1 ]]; then
  echo "no finos-desktop process" >&2
  exit 1
fi

http_code="$(curl -s -o /dev/null -w '%{http_code}' http://localhost:1420/ || true)"
echo "http $http_code"
if [[ "$http_code" != "200" ]]; then
  echo "localhost:1420 not ready" >&2
  exit 1
fi

if [[ -f "$PAGE" ]]; then
  echo "page-loaded: $(tail -n 1 "$PAGE")"
else
  echo "warning: no page-loaded.log yet under $APP_DATA" >&2
fi

if [[ -f "$CONSOLE" ]]; then
  if grep -Eiq 'panicked|Failed to setup|npm error' "$CONSOLE"; then
    echo "dev-console.log has a hard failure" >&2
    tail -n 40 "$CONSOLE" >&2
    exit 1
  fi
fi

echo "app-up.sh ok"
exit 0
