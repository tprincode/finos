#!/usr/bin/env bash
# Thin entry: automate Mac bootstrap (see docs/architecture/mac-dev-bootstrap.md).
exec "$(cd "$(dirname "$0")" && pwd)/scripts/mac-install.sh" "$@"
