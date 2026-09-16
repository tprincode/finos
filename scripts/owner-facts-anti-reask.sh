#!/usr/bin/env bash
# Owner-facts anti-reask gate (OF-1..OF-4).
# Fail if tip lacks YBTC owner-facts or the skill/rule/board stop text.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
fail=0

check() {
  local id="$1" msg="$2"
  shift 2
  if "$@"; then
    echo "PASS $id $msg"
  else
    echo "FAIL $id $msg"
    fail=1
  fi
}

check OF-1 "docs/Authority/owner-facts/YBTC.md exists" \
  test -f "$root/docs/Authority/owner-facts/YBTC.md"

check OF-2 "finos-milestone skill requires read before ask" \
  rg -qi "before asking the owner about ROC" "$root/.cursor/skills/finos-milestone/SKILL.md"

check OF-3 "execution-framework stops on owner-facts" \
  rg -q "owner-facts" "$root/.cursor/rules/execution-framework.mdc"

# Board must not frame YBTC as the active unknown-ROC thin slice.
if rg -N "^\*\*Now:\*\*.*YBTC.*thin slice" "$root/docs/architecture/execution.md" >/dev/null; then
  echo "FAIL OF-4 execution.md Now still frames YBTC as thin-slice unknown ROC"
  fail=1
else
  echo "PASS OF-4 execution.md Now does not frame YBTC as thin-slice unknown ROC"
fi

check OF-YBTC-LOCK "YBTC.md forbids re-ask" \
  rg -q "Do not re-ask ROC" "$root/docs/Authority/owner-facts/YBTC.md"

if [[ "$fail" -ne 0 ]]; then
  echo "owner-facts-anti-reask: FAILED"
  exit 1
fi
echo "owner-facts-anti-reask: OK"
