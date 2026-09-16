#!/usr/bin/env bash
# Installed-release Restart honesty + destroy-pattern gate.
# Linux CI can enforce source contracts; it cannot prove Windows NSIS relaunch.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "restart-owner-gate: static destroy-before-app.restart contract"
cargo test -p golden-harness --test desktop_menu \
  installed_release_restart_must_not_destroy_windows_before_app_restart \
  -- --exact

python3 - <<'PY'
import json, pathlib, re, sys

root = pathlib.Path(".")
catalog = json.loads((root / "docs/architecture/core-functions.json").read_text())
item = next(i for i in catalog["items"] if i["id"] == "file-restart-graceful")
verified = item.get("lastVerified", "")
attest_path = root / "docs/architecture/restart-owner-attestation.md"
if not attest_path.is_file():
    print("restart-owner-gate: missing docs/architecture/restart-owner-attestation.md", file=sys.stderr)
    sys.exit(1)
attest = attest_path.read_text()
# Only the first STATUS fence counts (ignore later prose).
fence = re.search(r"## STATUS\s*```(.*?)```", attest, re.S)
if not fence:
    print("restart-owner-gate: attestation missing ## STATUS fenced block", file=sys.stderr)
    sys.exit(1)
status = fence.group(1)
confirmed = bool(
    re.search(r"^OWNER_CONFIRMED_INSTALLED_RESTART:\s*true\s*$", status, re.M)
)
date_like = bool(re.fullmatch(r"\d{4}-\d{2}-\d{2}", verified))
if verified == "UNVERIFIED-owner-windows-installed":
    if confirmed:
        print(
            "restart-owner-gate: attestation STATUS says confirmed but lastVerified is still UNVERIFIED",
            file=sys.stderr,
        )
        sys.exit(1)
    print("restart-owner-gate: lastVerified UNVERIFIED (honest); static contract ok")
    sys.exit(0)
if date_like:
    if not confirmed:
        print(
            "restart-owner-gate: lastVerified={!r} looks like a date but "
            "STATUS OWNER_CONFIRMED_INSTALLED_RESTART is not true — false claim".format(verified),
            file=sys.stderr,
        )
        sys.exit(1)
    print("restart-owner-gate: owner attestation present; lastVerified={}".format(verified))
    sys.exit(0)
print(
    "restart-owner-gate: lastVerified={!r} must be UNVERIFIED-owner-windows-installed "
    "or YYYY-MM-DD with owner attestation".format(verified),
    file=sys.stderr,
)
sys.exit(1)
PY

echo "restart-owner-gate: ok"
