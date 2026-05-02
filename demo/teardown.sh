#!/usr/bin/env bash
# Remove all demo fixture state so the demo can be re-recorded from scratch.
set -euo pipefail

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES="$DEMO_DIR/fixtures"

if [ -d "$FIXTURES" ]; then
    rm -rf "$FIXTURES"
    echo "==> Removed $FIXTURES"
else
    echo "==> Nothing to clean (fixtures not found)"
fi

SYMLINK="/tmp/conduit-demo"
if [ -L "$SYMLINK" ]; then
    rm "$SYMLINK"
    echo "==> Removed symlink $SYMLINK"
fi
