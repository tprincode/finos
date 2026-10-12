#!/bin/bash
# FinOS daily Mac launch — double-click in Finder (Mac counterpart to finos.bat).
# Runs ./finos.sh from this repo root.
# Named finos.command (not go.command) so it does not collide with GO.command on
# case-insensitive volumes (Windows NTFS, default macOS APFS).
set +e
cd "$(dirname "$0")" || {
  echo "Could not cd to $(dirname "$0")" >&2
  echo "Press Return to close…"
  read -r _
  exit 1
}

if [[ ! -f "./finos.sh" ]]; then
  echo "finos.sh not found next to finos.command." >&2
  echo "Expected ~/Developer/finos after USB GO.command (or a git clone)." >&2
  echo "Press Return to close…"
  read -r _
  exit 1
fi

chmod +x ./finos.sh ./scripts/mac-env.sh 2>/dev/null || true
echo "Starting FinOS desktop…"
./finos.sh
status=$?
if [[ "$status" -ne 0 ]]; then
  echo ""
  echo "finos.sh exited with code $status."
  echo "If tools look broken: ./install-mac.sh --check"
  echo "Press Return to close…"
  read -r _
  exit "$status"
fi
exit 0
