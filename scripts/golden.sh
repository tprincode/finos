#!/usr/bin/env bash
# Run golden suites and say up front how long each one takes (Mac parity with golden.ps1).
# Reads/writes docs/architecture/golden-durations.json.
#
# Usage:  ./scripts/golden.sh cart ui_modules
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
RECORD="$ROOT/docs/architecture/golden-durations.json"

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <suite> [suite…]" >&2
  echo "suites are the --test names, for example: cart accessibility ui_modules" >&2
  exit 2
fi

if [[ ! -f "$RECORD" ]]; then
  echo "missing $RECORD" >&2
  exit 1
fi

format_duration() {
  local seconds="$1"
  if [[ "$seconds" -lt 60 ]]; then
    echo "$seconds seconds"
    return
  fi
  python3 -c "print(f'{round($seconds/60, 1)} minutes')" 2>/dev/null || echo "$seconds seconds"
}

failed=0
for suite in "$@"; do
  known="$(python3 -c "
import json
d=json.load(open(r'''$RECORD'''))
v=d.get('suites',{}).get('''$suite''', None)
print('' if v is None else int(v))
" 2>/dev/null || true)"

  if [[ -z "$known" ]]; then
    echo "running the $suite golden, duration not measured on this machine yet"
  else
    echo "running a $(format_duration "$known") golden: $suite"
  fi

  start="$(date +%s)"
  set +e
  cargo test -p golden-harness --test "$suite"
  code=$?
  set -e
  end="$(date +%s)"
  elapsed=$((end - start))

  if [[ -z "$known" ]]; then
    echo "the $suite golden took $(format_duration "$elapsed")"
  else
    echo "the $suite golden took $(format_duration "$elapsed"), expected $(format_duration "$known")"
  fi
  if [[ "$code" -ne 0 ]]; then
    echo "$suite FAILED (exit $code)"
    failed=1
  fi

  python3 -c "
import json
path=r'''$RECORD'''
d=json.load(open(path))
d.setdefault('suites',{})['''$suite'''] = int('''$elapsed''')
open(path,'w',encoding='utf-8',newline='\n').write(json.dumps(d, indent=4)+'\n')
"
done

exit "$failed"
