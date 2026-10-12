#!/usr/bin/env bash
# Owner Mac start: copy FinOS from this USB kit with almost no git commands.
# Idempotent. Does not overwrite local.sqlite.
#
# Usage (plug in the USB, open Terminal, paste — replace VolumeName if Finder shows another name):
#   bash /Volumes/<VolumeName>/Finos/load-from-usb.sh
#   bash /Volumes/<VolumeName>/Finos/load-from-usb.sh --seed
#   bash /Volumes/<VolumeName>/Finos/load-from-usb.sh --launch
#
# When run from a repo checkout: ./scripts/load-from-usb.sh (looks for /Volumes/*/Finos/finos.bundle)
set -euo pipefail

SEED=0
LAUNCH=0
BRANCH="cursor/core-functions-decl-color"
TARGET="$HOME/Developer/finos"

usage() {
  sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
  exit 2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --seed) SEED=1; shift ;;
    --launch) LAUNCH=1; shift ;;
    -h|--help) usage ;;
    *)
      echo "unknown flag: $1" >&2
      usage
      ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "load-from-usb.sh runs on macOS only." >&2
  exit 1
fi

say() { printf '\n==> %s\n' "$*"; }

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
KIT=""
if [[ -f "$SCRIPT_DIR/finos.bundle" ]]; then
  KIT="$SCRIPT_DIR"
elif [[ -f "$SCRIPT_DIR/../finos.bundle" ]]; then
  KIT="$(cd "$SCRIPT_DIR/.." && pwd)"
else
  for cand in /Volumes/*/Finos; do
    if [[ -f "$cand/finos.bundle" ]]; then
      KIT="$cand"
      break
    fi
  done
fi

if [[ -z "$KIT" ]] || [[ ! -f "$KIT/finos.bundle" ]]; then
  echo "Could not find Finos/finos.bundle on a USB volume." >&2
  echo "Plug in the stick, open it in Finder, note the name under /Volumes, then run:" >&2
  echo "  bash /Volumes/<ThatName>/Finos/load-from-usb.sh" >&2
  exit 1
fi

say "USB kit: $KIT"
BUNDLE="$KIT/finos.bundle"
mkdir -p "$(dirname "$TARGET")"

if [[ ! -d "$TARGET/.git" ]]; then
  say "Copying FinOS into $TARGET (first time)"
  git clone "$BUNDLE" "$TARGET"
else
  say "FinOS already at $TARGET — leaving your copy; re-run install only"
fi

cd "$TARGET"
git checkout "$BRANCH" 2>/dev/null || git checkout -B "$BRANCH"
chmod +x install-mac.sh scripts/mac-install.sh scripts/mac-env.sh scripts/load-from-usb.sh scripts/golden.sh finos.sh scripts/app-up.sh 2>/dev/null || true

say "Running installer"
# Avoid "${arr[@]}" under `set -u` when empty (bash nounset unbound error).
if [[ "$SEED" -eq 1 && "$LAUNCH" -eq 1 ]]; then
  exec ./install-mac.sh --seed --launch
elif [[ "$SEED" -eq 1 ]]; then
  exec ./install-mac.sh --seed
elif [[ "$LAUNCH" -eq 1 ]]; then
  exec ./install-mac.sh --launch
else
  exec ./install-mac.sh
fi
