#!/bin/bash
# FinOS USB Start here — double-click in Finder (Mac counterpart to typing bash load-from-usb).
# Copies the kit into ~/Developer/finos and runs the installer.
set +e
cd "$(dirname "$0")" || {
  echo "Could not cd to $(dirname "$0")" >&2
  echo "Press Return to close…"
  read -r _
  exit 1
}

if [[ ! -f "./load-from-usb.sh" ]]; then
  echo "load-from-usb.sh not found next to GO.command." >&2
  echo "Open the Finos folder on the USB stick, then double-click GO.command again." >&2
  echo "Press Return to close…"
  read -r _
  exit 1
fi

chmod +x ./load-from-usb.sh 2>/dev/null || true
echo "Starting FinOS USB load…"
bash ./load-from-usb.sh "$@"
status=$?
if [[ "$status" -ne 0 ]]; then
  echo ""
  echo "USB load/install exited with code $status."
  echo "If Xcode Command Line Tools just finished installing, double-click GO.command again."
  echo "Press Return to close…"
  read -r _
  exit "$status"
fi
echo ""
echo "Done. Daily launch: open ~/Developer/finos and double-click finos.command"
echo "Press Return to close…"
read -r _
exit 0
