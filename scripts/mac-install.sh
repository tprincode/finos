#!/usr/bin/env bash
# Automate docs/architecture/mac-dev-bootstrap.md on Apple Silicon.
# Idempotent: safe to re-run. Does not overwrite local.sqlite.
# First boot is the slow path; daily coding uses ./finos.sh (not this script).
#
# Usage (from any clone, or after load-from-usb.sh):
#   ./scripts/mac-install.sh
#   ./scripts/mac-install.sh --seed          # also npm run data-seed
#   ./scripts/mac-install.sh --launch        # start ./finos.sh when done
#   ./scripts/mac-install.sh --skip-brew     # do not install Homebrew packages
#   ./scripts/mac-install.sh --check         # Ready/doctor report only (dirs + toolchain)
#   ./scripts/mac-install.sh --repo ~/Developer/finos
set -euo pipefail

SEED=0
LAUNCH=0
SKIP_BREW=0
CHECK_ONLY=0
REPO_OVERRIDE=""

usage() {
  sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'
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

# shellcheck source=scripts/mac-env.sh
source "$SCRIPT_DIR/mac-env.sh"

path_looks_cloud() {
  local p="$1"
  case "$p" in
    *"/Library/Mobile Documents/"*|*/Dropbox/*|*/OneDrive*|*/Google\ Drive/*|*/iCloud*)
      return 0
      ;;
  esac
  return 1
}

# Append a marked block to a profile file once (idempotent).
append_profile_block() {
  local file="$1"
  local marker="$2"
  local body="$3"
  if [[ -f "$file" ]] && grep -Fq "$marker" "$file" 2>/dev/null; then
    echo "  ok already in $file ($marker)"
    return 0
  fi
  mkdir -p "$(dirname "$file")"
  {
    echo ""
    echo "$marker"
    printf '%s\n' "$body"
    echo "# end $marker"
  } >>"$file"
  echo "  wrote $marker → $file"
}

# Persist brew + cargo PATH so new Terminals see cargo without manual exports.
persist_shell_path() {
  say "Persist PATH in login shell (once)"
  local brew_block cargo_block
  brew_block='if [ -x /opt/homebrew/bin/brew ]; then
  eval "$(/opt/homebrew/bin/brew shellenv)"
fi'
  cargo_block='if [ -f "$HOME/.cargo/env" ]; then
  . "$HOME/.cargo/env"
fi'
  append_profile_block "$HOME/.zprofile" "# finos-mac-brew" "$brew_block"
  append_profile_block "$HOME/.zprofile" "# finos-mac-cargo" "$cargo_block"
  if [[ -f "$HOME/.bash_profile" ]]; then
    append_profile_block "$HOME/.bash_profile" "# finos-mac-brew" "$brew_block"
    append_profile_block "$HOME/.bash_profile" "# finos-mac-cargo" "$cargo_block"
  fi
}

ensure_rust_toolchain() {
  # shellcheck source=scripts/mac-env.sh
  source "$SCRIPT_DIR/mac-env.sh"
  if have rustc && have cargo; then
    return 0
  fi
  say "Finish Rust toolchain (rustup)"
  local rustup_prefix=""
  if have brew; then
    brew install rustup || brew install rustup-init
    rustup_prefix="$(brew --prefix rustup 2>/dev/null || true)"
    if [[ -n "$rustup_prefix" && -d "$rustup_prefix/bin" ]]; then
      export PATH="$rustup_prefix/bin:$PATH"
    fi
  fi
  if [[ ! -f "$HOME/.cargo/env" ]]; then
    if have rustup-init; then
      rustup-init -y --default-toolchain stable
    elif [[ -n "$rustup_prefix" && -x "$rustup_prefix/bin/rustup-init" ]]; then
      "$rustup_prefix/bin/rustup-init" -y --default-toolchain stable
    elif have rustup; then
      rustup default stable || rustup toolchain install stable
    else
      echo "rustup-init not on PATH after brew install rustup." >&2
      echo "  export PATH=\"\$(brew --prefix rustup)/bin:\$PATH\"" >&2
      echo "  rustup-init -y --default-toolchain stable" >&2
      echo "Then re-run: cd $ROOT && ./install-mac.sh" >&2
      return 1
    fi
  elif have rustup; then
    rustup default stable || rustup toolchain install stable
  fi
  # shellcheck source=scripts/mac-env.sh
  source "$SCRIPT_DIR/mac-env.sh"
  if ! have rustc || ! have cargo; then
    echo "cargo/rustc still missing after rustup." >&2
    echo "Next action: open a new Terminal, then: cd $ROOT && ./install-mac.sh" >&2
    return 1
  fi
  return 0
}

# Ready/doctor report: locked dirs + toolchain. Exit 1 on hard fails.
ready_report() {
  local fail=0
  local next_actions=()
  say "Ready report"

  # shellcheck source=scripts/mac-env.sh
  source "$SCRIPT_DIR/mac-env.sh"

  echo "Xcode Command Line Tools:"
  if xcode-select -p >/dev/null 2>&1; then
    echo "  ok $(xcode-select -p)"
  else
    echo "  FAIL Xcode CLT missing"
    next_actions+=("Finish Apple's Command Line Tools installer, then: cd $ROOT && ./install-mac.sh")
    fail=1
  fi

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
      next_actions+=("Re-run: cd $ROOT && ./install-mac.sh  (creates $path)")
      fail=1
      continue
    fi
    if [[ ! -w "$path" ]]; then
      echo "  FAIL $label — not writable: $path"
      next_actions+=("Fix permissions on $path, then ./install-mac.sh --check")
      fail=1
      continue
    fi
    if path_looks_cloud "$path"; then
      echo "  FAIL $label — looks like cloud sync (use local path): $path"
      next_actions+=("Move $label off iCloud/Dropbox/OneDrive to a local path")
      fail=1
      continue
    fi
    echo "  ok $label — $path"
  done

  echo "Repo files:"
  for f in package.json finos.sh install-mac.sh scripts/mac-install.sh scripts/mac-env.sh scripts/golden.sh scripts/load-from-usb.sh; do
    if [[ -f "$ROOT/$f" ]]; then
      echo "  ok $f"
    else
      echo "  FAIL missing $f under $ROOT"
      next_actions+=("Restore missing $f (USB load-from-usb.sh or git pull)")
      fail=1
    fi
  done

  echo "Installed modules (toolchain, after mac-env PATH):"
  for cmd in node npm cargo rustc git; do
    if have "$cmd"; then
      echo "  ok $cmd ($($cmd --version 2>&1 | head -n 1))"
    else
      echo "  FAIL missing $cmd"
      next_actions+=("Missing $cmd — re-run: cd $ROOT && ./install-mac.sh")
      fail=1
    fi
  done

  if have rustup; then
    echo "  ok rustup present"
    rustup show 2>/dev/null | head -n 8 | sed 's/^/    /' || true
  elif have cargo; then
    echo "  note rustup not on PATH (cargo works; ok for daily ./finos.sh)"
  else
    echo "  note rustup not on PATH"
  fi

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
    if [[ "${#next_actions[@]}" -gt 0 ]]; then
      echo "Next action:" >&2
      local a
      for a in "${next_actions[@]}"; do
        echo "  - $a" >&2
      done
    fi
    return 1
  fi
  echo "Ready report: READY"
  echo "Daily coding (do not re-install): cd $ROOT && ./finos.sh"
  echo "Then: ./scripts/app-up.sh"
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
  ready_report
  exit $?
fi

say "Shell script bits"
chmod +x "$ROOT/finos.sh" "$ROOT/scripts/app-up.sh" "$ROOT/scripts/golden.sh" \
  "$ROOT/scripts/mac-install.sh" "$ROOT/scripts/mac-env.sh" "$ROOT/scripts/load-from-usb.sh" \
  "$ROOT/install-mac.sh" 2>/dev/null || true

say "Xcode Command Line Tools"
if xcode-select -p >/dev/null 2>&1; then
  echo "ok: $(xcode-select -p)"
else
  echo "Xcode CLT missing — opening Apple's installer (expected once on a new Mac)." >&2
  echo "When that installer finishes, re-run exactly:" >&2
  echo "  cd $ROOT && ./install-mac.sh" >&2
  xcode-select --install || true
  exit 1
fi

if [[ "$SKIP_BREW" -eq 0 ]]; then
  say "Homebrew"
  if ! have brew; then
    echo "Installing Homebrew (interactive password may be required)…"
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
  fi
  # shellcheck source=scripts/mac-env.sh
  source "$SCRIPT_DIR/mac-env.sh"
  if ! have brew; then
    echo "brew still not on PATH after install." >&2
    echo "Next action: eval \"\$(/opt/homebrew/bin/brew shellenv)\" then re-run ./install-mac.sh" >&2
    exit 1
  fi

  say "Brew packages (node, gh, rustup)"
  brew update
  brew install node gh
  if ! have rustc || ! have cargo; then
    # Homebrew ships `rustup` (keg-only). Older docs said `rustup-init`.
    brew install rustup || brew install rustup-init
  fi
fi

# shellcheck source=scripts/mac-env.sh
source "$SCRIPT_DIR/mac-env.sh"
ensure_rust_toolchain

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
  echo "Next action: cd $ROOT && ./install-mac.sh" >&2
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

persist_shell_path

cd "$ROOT"

if [[ -f rust-toolchain.toml ]] || [[ -f rust-toolchain ]]; then
  say "Rust toolchain (repo pin)"
  if have rustup; then
    rustup show
  else
    echo "note: rustup not on PATH; $(cargo --version 2>&1 || true)"
  fi
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
First-boot install finished. Daily coding does NOT need this script again.

Next:
  1. Open Cursor (Apple Silicon) → File → Open Folder → ${TARGET}
  2. Launch:  cd $ROOT && ./finos.sh
  3. Verify:  ./scripts/app-up.sh
  4. Tools look broken later: ./install-mac.sh --check
  5. Goldens (coding): ./scripts/golden.sh ui_modules

New Terminal windows pick up brew/cargo from ~/.zprofile (written once above).
Do not put local.sqlite in iCloud, Dropbox, or OneDrive.
Do not use finos.bat / app-up.ps1 on Mac.
EOF

if [[ "$LAUNCH" -eq 1 ]]; then
  say "Launching ./finos.sh"
  exec "$ROOT/finos.sh"
fi
