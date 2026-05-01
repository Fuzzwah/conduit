#!/bin/bash
# Test arrow key navigation in the base directory dialog

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Create unique temp directory
DATA_DIR=$(mktemp -d /tmp/conduit-test-arrows-XXXXXX)
echo "Using data directory: $DATA_DIR"

# Cleanup on exit
cleanup() {
    rm -rf "$DATA_DIR"
    echo "Cleaned up: $DATA_DIR"
}
trap cleanup EXIT

# Create the step file with the actual data directory
STEP_FILE=$(mktemp /tmp/test-arrows-XXXXXXXX.yaml)

cat > "$STEP_FILE" << EOF
session:
  command: ["$PROJECT_ROOT/target/release/conduit", "--data-dir", "$DATA_DIR"]
  cols: 120
  rows: 40

steps:
  # Wait for initial startup and main screen
  - waitForIdle: {idleMs: 1000, timeoutMs: 10000}
  - waitForText: {text: "C-n new project", timeoutMs: 5000}

  # Open the base directory dialog with Ctrl+N
  - hotkey: {ctrl: true, ch: "n"}
  - waitForText: {text: "Set Projects Directory", timeoutMs: 5000}
  - screenshot: {name: "01-initial-dialog"}

  # The default text should be ~/code
  - expectText: {text: "~/code"}

  # Clear the input and type a test path
  # First go to end, then delete all
  - press: {key: End}
  - waitForIdle: {idleMs: 100}

  # Delete existing content (~/code = 6 chars)
  - press: {key: Backspace}
  - press: {key: Backspace}
  - press: {key: Backspace}
  - press: {key: Backspace}
  - press: {key: Backspace}
  - press: {key: Backspace}
  - waitForIdle: {idleMs: 100}

  # Type a new test path
  - type: {text: "~/test"}
  - waitForIdle: {idleMs: 200}
  - screenshot: {name: "02-typed-test"}

  # Now test LEFT arrow - move cursor left 2 positions
  - press: {key: Left}
  - press: {key: Left}
  - waitForIdle: {idleMs: 100}
  - screenshot: {name: "03-after-left-arrows"}

  # Type a character at cursor position (should insert 'X' before 'st')
  - type: {text: "X"}
  - waitForIdle: {idleMs: 200}
  - screenshot: {name: "04-after-insert"}

  # Should now show ~/teXst
  - expectText: {text: "~/teXst"}

  # Now test RIGHT arrow - move right one position and insert Y
  - press: {key: Right}
  - waitForIdle: {idleMs: 100}
  - type: {text: "Y"}
  - waitForIdle: {idleMs: 200}
  - screenshot: {name: "05-after-right-insert"}

  # Should now show ~/teXsYt
  - expectText: {text: "~/teXsYt"}

  # Test Home key - go to start
  - press: {key: Home}
  - waitForIdle: {idleMs: 100}
  - type: {text: "START"}
  - waitForIdle: {idleMs: 200}
  - screenshot: {name: "06-after-home-insert"}

  # Should now show START~/teXsYt
  - expectText: {text: "START~/teXsYt"}

  # Test End key - go to end
  - press: {key: End}
  - waitForIdle: {idleMs: 100}
  - type: {text: "END"}
  - waitForIdle: {idleMs: 200}
  - screenshot: {name: "07-final"}

  # Should now show START~/teXsYtEND
  - expectText: {text: "START~/teXsYtEND"}

  # Cancel the dialog
  - press: {key: Escape}
  - waitForIdle: {idleMs: 500}

artifacts:
  mode: always
  dir: $SCRIPT_DIR/artifacts/base-dir-arrows
EOF

echo "Running termwright test..."
echo "Step file: $STEP_FILE"
echo ""

# Ensure artifacts dir exists
mkdir -p "$SCRIPT_DIR/artifacts/base-dir-arrows"

# Run the test
termwright run-steps --trace "$STEP_FILE"

echo ""
echo "Test completed! Check artifacts in: $SCRIPT_DIR/artifacts/base-dir-arrows"

# Cleanup step file
rm -f "$STEP_FILE"
