#!/usr/bin/env bash
# Owner-facts anti-reask gate — global fleet policy.
# OF-1..OF-4 (legacy) + OF-G1..OF-G4 (fleet framework).
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
fail=0

FLEET=(
  AMDW AMDY AMZY BITO BTCI CEFS CLM CONY CRF EFC
  EPD ET FDRXX GLAD HAKY IGLD JEPQ MPLX MSTY NFLY
  NVDW ORC PLTW QDTE QDVO QQQI QYLD RDTE SPAXX SPYI
  SVOL SWVXX TOPW TRIN TSLW TSPY XDTE XPAY YBTC YMAX
)

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

if rg -N "^\*\*Now:\*\*.*YBTC.*thin slice" "$root/docs/architecture/execution.md" >/dev/null; then
  echo "FAIL OF-4 execution.md Now still frames YBTC as thin-slice unknown ROC"
  fail=1
else
  echo "PASS OF-4 execution.md Now does not frame YBTC as thin-slice unknown ROC"
fi

check OF-YBTC-LOCK "YBTC.md forbids re-ask" \
  rg -q "Do not re-ask ROC" "$root/docs/Authority/owner-facts/YBTC.md"

check OF-G1 "owner-facts README is global policy" \
  rg -q "global policy" "$root/docs/Authority/owner-facts/README.md"

check OF-G2 "skill recreate rule present" \
  rg -qi "Recreate / first-enable" "$root/.cursor/skills/finos-milestone/SKILL.md"

check OF-G3 "execution-framework recreate rule present" \
  rg -qi "Recreate / first-enable" "$root/.cursor/rules/execution-framework.mdc"

check OF-G4 "TEMPLATE.md exists" \
  test -f "$root/docs/Authority/owner-facts/TEMPLATE.md"

check OF-G5 "INDEX.md exists" \
  test -f "$root/docs/Authority/owner-facts/INDEX.md"

missing=0
for sym in "${FLEET[@]}"; do
  if [[ ! -f "$root/docs/Authority/owner-facts/${sym}.md" ]]; then
    echo "FAIL OF-FLEET missing owner-facts/${sym}.md"
    missing=1
    fail=1
  fi
done
if [[ "$missing" -eq 0 ]]; then
  echo "PASS OF-FLEET all ${#FLEET[@]} income-fleet owner-facts files exist"
fi

# Recreate pass check documentation (binary wording in README)
check OF-RECREATE "README states recreate must create owner-facts" \
  rg -qi "recreate.*first-enable|first-enable.*owner-facts" "$root/docs/Authority/owner-facts/README.md"

if [[ "$fail" -ne 0 ]]; then
  echo "owner-facts-anti-reask: FAILED"
  exit 1
fi
echo "owner-facts-anti-reask: OK"
