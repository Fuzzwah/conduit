#!/usr/bin/env bash
# Wrapper that launches conduit pointed at the demo fixtures directory.
# Prepends fixtures/bin/ to PATH so the gh shim is used instead of real gh.
DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export PATH="$DEMO_DIR/fixtures/bin:$PATH"
exec "$DEMO_DIR/../target/debug/conduit" --data-dir "$DEMO_DIR/fixtures/data" "$@"
