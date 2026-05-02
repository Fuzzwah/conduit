#!/bin/bash
# Test: Settings dialogs — model selector (Ctrl+O), theme picker (Alt+T), session import (Alt+I)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
if [ ! -f "$PROJECT_ROOT/target/release/conduit" ]; then
  echo "Building conduit release binary..."
  (cd "$PROJECT_ROOT" && cargo build --release)
fi

# Model selector: opens on Ctrl+O, closes on Escape
test_model_selector() {
    local sock="$1"

    ctrl "$sock" "o"
    wait_idle "$sock" 300 5000 > /dev/null

    assert_contains "$sock" "Model" "Model selector dialog appeared" || return 1

    press "$sock" "Escape"
    wait_idle "$sock" 300 3000 > /dev/null

    assert_not_contains "$sock" "Model" "Model selector gone after Escape" || return 1

    return 0
}

# Theme picker: opens on Alt+T, closes on Escape
test_theme_picker() {
    local sock="$1"

    alt "$sock" "t"
    wait_idle "$sock" 300 5000 > /dev/null

    assert_contains "$sock" "Theme" "Theme picker dialog appeared" || return 1

    press "$sock" "Escape"
    wait_idle "$sock" 300 3000 > /dev/null

    assert_not_contains "$sock" "Theme" "Theme picker gone after Escape" || return 1

    return 0
}

# Session import picker: opens on Alt+I, closes on Escape
test_session_import() {
    local sock="$1"

    alt "$sock" "i"
    wait_idle "$sock" 300 5000 > /dev/null

    assert_contains "$sock" "Import Session" "Session import dialog appeared" || return 1

    press "$sock" "Escape"
    wait_idle "$sock" 300 3000 > /dev/null

    assert_not_contains "$sock" "Import Session" "Session import gone after Escape" || return 1

    return 0
}

run_test "settings_model_selector" test_model_selector
run_test "settings_theme_picker" test_theme_picker
run_test "settings_session_import" test_session_import
