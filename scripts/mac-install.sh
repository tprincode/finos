#!/usr/bin/env bash
# Automate docs/architecture/mac-dev-bootstrap.md on Apple Silicon.
# Idempotent: safe to re-run. Does not overwrite local.sqlite.
#
# Usage (from any clone, or after load-from-usb.sh):
#   ./scripts/mac-install.sh
#   ./scripts/mac-install.sh --seed          # also npm run data-seed
#   ./scripts/mac-install.sh --launch        # start ./finos.sh when done
#   ./scripts/mac-install.sh --skip-brew     # do not install Homebrew packages
#   ./scripts/mac-install.sh --check         # Ready report only (dirs + toolchain)
#   ./scripts/mac-install.sh --repo ~/Developer/finos
set -euo pipefail

SEED=0
LAUNCH=0
SKIP_BREW=0
CHECK_ONLY=0
REPO_OVERRIDE=""

usage() {
  sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'
  exit 2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --seed) SEED=1; shift ;;
    --launch) LAUNCH=1; shift ;;
    --skip-brew) SKIP_BREW=1; shift ;;
    --check) CHECK_ONLY=1; shift ;;
    --repo)
      REPO_OVERRIDE="${2:-}"
      if [[ -z "$REPO_OVERRIDE" ]]; then usage; fi
      shift 2
      ;;
    -h|--help) usage ;;
    *)
      echo "unknown flag: $1" >&2
      usage
      ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "mac-install.sh runs on macOS only (got $(uname -s))." >&2
  exit 1
fi

ARCH="$(uname -m)"
if [[ "$ARCH" != "arm64" ]]; then
  echo "warning: bootstrap is locked for Apple Silicon (arm64); this machine is $ARCH" >&2
fi

say() { printf '\n==> %s\n' "$*"; }
have() { command -v "$1" >/dev/null 2>&1; }

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEFAULT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT="${REPO_OVERRIDE:-$DEFAULT_ROOT}"
TARGET="$HOME/Developer/finos"
APP_DATA="${FINOS_APP_DATA:-$HOME/Library/Application Support/com.finos.desktop}"
DOWNLOAD_DIR="${FINOS_DOWNLOAD_DIR:-$HOME/Documents/Financial}"

path_looks_cloud() {
  local p="$1"
  case "$p" in
    *"/Library/Mobile Documents/"*|*/Dropbox/*|*/OneDrive*|*/Google\ Drive/*|*/iCloud*)
      return 0
      ;;
  esac
  return 1
}

# Ready report: locked dirs + toolchain. Exit 1 on hard fails.
ready_report() {
  local fail=0
  say "Ready report"

  echo "Locked directories:"
  for label_path in \
    "Repo|$TARGET" \
    "App data|$APP_DATA" \
    "Downloads|$DOWNLOAD_DIR"
  do
    local label="${label_path%%|*}"
    local path="${label_path#*|}"
    if [[ ! -d "$path" ]]; then
      echo "  FAIL $label — missing: $path"
      fail=1
      continue
    fi
    if [[ ! -w "$path" ]]; then
      echo "  FAIL $label — not writable: $path"
      fail=1
      continue
    fi
    if path_looks_cloud "$path"; then
      echo "  FAIL $label — looks like cloud sync (use local path): $path"
      fail=1
      continue
    fi
    echo "  ok $label — $path"
  done

  echo "Repo files:"
  for f in package.json finos.sh install-mac.sh scripts/mac-install.sh scripts/golden.sh scripts/load-from-usb.sh; do
    if [[ -f "$ROOT/$f" ]]; then
      echo "  ok $f"
    else
      echo "  FAIL missing $f under $ROOT"
      fail=1
    fi
  done

  echo "Installed modules (toolchain):"
  for cmd in node npm cargo rustc git; do
    if have "$cmd"; then
      echo "  ok $cmd ($($cmd --version 2>&1 | head -n 1))"
    else
      echo "  FAIL missing $cmd"
      fail=1
    fi
  done
  if have gh; then
    if gh auth status >/dev/null 2>&1; then
      echo "  ok gh authenticated"
    else
      echo "  note gh installed but not logged in (run: gh auth login)"
    fi
  else
    echo "  note gh not installed (optional until you push)"
  fi

  if [[ "$fail" -ne 0 ]]; then
    echo "Ready report: NOT READY" >&2
    return 1
  fi
  echo "Ready report: READY"
  return 0
}

if [[ ! -f "$ROOT/package.json" ]] || [[ ! -f "$ROOT/finos.sh" ]]; then
  echo "repo root not found at $ROOT (need package.json + finos.sh)." >&2
  echo "Use USB Start here: bash /Volumes/<drive>/Finos/load-from-usb.sh" >&2
  exit 1
fi

say "Repo: $ROOT"
say "App data: $APP_DATA"
say "Downloads: $DOWNLOAD_DIR"

if [[ "$ROOT" != "$TARGET" ]] && [[ ! -e "$TARGET" ]]; then
  say "Linking $TARGET → $ROOT (locked Cursor workspace path)"
  mkdir -p "$(dirname "$TARGET")"
  ln -s "$ROOT" "$TARGET"
elif [[ "$ROOT" != "$TARGET" ]] && [[ -e "$TARGET" ]]; then
  echo "note: locked path $TARGET already exists; continuing with $ROOT" >&2
fi

say "Directories"
mkdir -p "$HOME/Developer" "$DOWNLOAD_DIR" "$APP_DATA"

if [[ "$CHECK_ONLY" -eq 1 ]]; then
  # Ensure cargo env for check-only when already installed.
  if [[ -f "$HOME/.cargo/env" ]]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
  fi
  if [[ -x /opt/homebrew/bin/brew ]]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
  fi
  ready_report
  exit $?
fi

say "Shell script bits"
chmod +x "$ROOT/finos.sh" "$ROOT/scripts/app-up.sh" "$ROOT/scripts/golden.sh" \
  "$ROOT/scripts/mac-install.sh" "$ROOT/scripts/load-from-usb.sh" "$ROOT/install-mac.sh" 2>/dev/null || true

say "Xcode Command Line Tools"
if xcode-select -p >/dev/null 2>&1; then
  echo "ok: $(xcode-select -p)"
else
  echo "Xcode CLT missing — opening installer. Re-run this script when it finishes." >&2
  xcode-select --install || true
  exit 1
fi

if [[ "$SKIP_BREW" -eq 0 ]]; then
  say "Homebrew"
  if ! have brew; then
    echo "Installing Homebrew (interactive password may be required)…"
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    if [[ -x /opt/homebrew/bin/brew ]]; then
      eval "$(/opt/homebrew/bin/brew shellenv)"
    fi
  else
    eval "$(brew shellenv 2>/dev/null || true)"
  fi
  if ! have brew; then
    echo "brew still not on PATH after install." >&2
    exit 1
  fi

  say "Brew packages (node, gh, and Tauri desktop deps)"
  brew update
  brew install node gh || true
  if ! have rustc || ! have cargo; then
    brew install rustup-init
    if [[ ! -f "$HOME/.cargo/env" ]]; then
      rustup-init -y --default-toolchain stable
    fi
  fi
fi

if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi
if [[ -x /opt/homebrew/bin/brew ]]; then
  eval "$(/opt/homebrew/bin/brew shellenv)"
fi

say "Toolchain check"
need_fail=0
for cmd in node npm cargo rustc git; do
  if have "$cmd"; then
    echo "ok: $cmd ($($cmd --version 2>&1 | head -n 1))"
  else
    echo "missing: $cmd" >&2
    need_fail=1
  fi
done
if [[ "$need_fail" -ne 0 ]]; then
  echo "Install the missing tools (or re-run without --skip-brew), then try again." >&2
  exit 1
fi

if have gh; then
  if gh auth status >/dev/null 2>&1; then
    echo "ok: gh authenticated"
  else
    echo "note: run 'gh auth login' (GitHub.com, HTTPS, browser) before git push"
  fi
else
  echo "note: gh not installed — 'brew install gh' then 'gh auth login' for HTTPS push"
fi

cd "$ROOT"

if [[ -f rust-toolchain.toml ]] || [[ -f rust-toolchain ]]; then
  say "Rust toolchain (repo pin)"
  rustup show
fi

say "npm install"
npm install

say "cargo check (workspace)"
cargo check

if [[ "$SEED" -eq 1 ]]; then
  say "data-seed into $APP_DATA"
  export FINOS_APP_DATA="$APP_DATA"
  export FINOS_DOWNLOAD_DIR="$DOWNLOAD_DIR"
  npm run data-seed
else
  echo "skip seed (pass --seed to load fixtures into app data)"
fi

ready_report || true

say "Done"
cat <<EOF
Next:
  1. Open Cursor (Apple Silicon) → File → Open Folder → ${TARGET}
  2. Launch:  cd $ROOT && ./finos.sh
  3. Verify:  ./scripts/app-up.sh
  4. Optional check later: ./install-mac.sh --check
  5. Goldens (coding): ./scripts/golden.sh ui_modules

Do not put local.sqlite in iCloud, Dropbox, or OneDrive.
Do not use finos.bat / app-up.ps1 on Mac.
EOF

if [[ "$LAUNCH" -eq 1 ]]; then
  say "Launching ./finos.sh"
  exec "$ROOT/finos.sh"
fi
