#!/usr/bin/env bash
# Shared PATH bootstrap for Mac FinOS scripts (install, launch, goldens).
# Source only — do not exec. Safe to source multiple times.
#
#   # shellcheck source=scripts/mac-env.sh
#   source "$(cd "$(dirname "$0")" && pwd)/mac-env.sh"
#   # or from repo root scripts: source "$ROOT/scripts/mac-env.sh"

# Homebrew (Apple Silicon default prefix).
if [[ -x /opt/homebrew/bin/brew ]]; then
  eval "$(/opt/homebrew/bin/brew shellenv)"
elif command -v brew >/dev/null 2>&1; then
  eval "$(brew shellenv 2>/dev/null || true)"
fi

# rustup is keg-only — put it ahead of any other rustc on PATH.
if command -v brew >/dev/null 2>&1; then
  _FINOS_RUSTUP_PREFIX="$(brew --prefix rustup 2>/dev/null || true)"
  if [[ -n "${_FINOS_RUSTUP_PREFIX:-}" && -d "$_FINOS_RUSTUP_PREFIX/bin" ]]; then
    export PATH="$_FINOS_RUSTUP_PREFIX/bin:$PATH"
  fi
  unset _FINOS_RUSTUP_PREFIX
fi

# Cargo / rustc from rustup-init (or prior install).
if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi
