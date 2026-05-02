#!/bin/bash
# Test: Slash command menu and command palette (Ctrl+P)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
source "$SCRIPT_DIR/lib.sh"

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "sqlite3 not found. Install with: brew install sqlite3" >&2
  exit 1
fi

if [ ! -f "$PROJECT_ROOT/target/release/conduit" ]; then
  echo "Building conduit release binary..."
  (cd "$PROJECT_ROOT" && cargo build --release)
fi

DATA_DIR=$(create_data_dir "slash-commands")

cleanup_local() {
  local sock="$1"
  if [ -n "$sock" ]; then
    close_daemon "$sock"
  fi
  rm -rf "$DATA_DIR"
}

mkdir -p "$DATA_DIR/workspaces/conduit/kind-mist"

(
  cd "$DATA_DIR/workspaces/conduit"
  git init
  git config user.email "test@test.com"
  git config user.name "Test"
  touch .gitkeep
  git add .gitkeep
  git commit -m "initial"
  git checkout -b test/kind-mist
) >/dev/null 2>&1

cat > "$DATA_DIR/config.toml" <<EOF_CONFIG
[tools]
codex = "/bin/true"
EOF_CONFIG

DB_PATH="$DATA_DIR/conduit.db"
sqlite3 "$DB_PATH" <<'EOF_SQL'
CREATE TABLE IF NOT EXISTS repositories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    base_path TEXT,
    repository_url TEXT,
    workspace_mode TEXT,
    archive_delete_branch INTEGER,
    archive_remote_prompt INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    name TEXT NOT NULL,
    branch TEXT NOT NULL,
    path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_accessed TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    archived_at TEXT,
    archived_commit_sha TEXT,
    FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS app_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS session_tabs (
    id TEXT PRIMARY KEY,
    tab_index INTEGER NOT NULL,
    is_open INTEGER NOT NULL DEFAULT 1,
    workspace_id TEXT,
    agent_type TEXT NOT NULL,
    agent_mode TEXT DEFAULT 'build',
    agent_session_id TEXT,
    model TEXT,
    pr_number INTEGER,
    created_at TEXT NOT NULL,
    pending_user_message TEXT,
    queued_messages TEXT NOT NULL DEFAULT '[]',
    input_history TEXT NOT NULL DEFAULT '[]',
    fork_seed_id TEXT,
    title TEXT,
    title_generated INTEGER NOT NULL DEFAULT 0,
    model_invalid INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);

INSERT INTO repositories (
    id, name, base_path, repository_url, workspace_mode,
    archive_delete_branch, archive_remote_prompt, created_at, updated_at
) VALUES (
    '11111111-1111-1111-1111-111111111111',
    'conduit',
    'DATA_DIR_PLACEHOLDER/workspaces/conduit',
    NULL,
    'checkout',
    0,
    0,
    strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
);

INSERT INTO workspaces (id, repository_id, name, branch, path, created_at, last_accessed, is_default)
VALUES
  ('11111111-1111-1111-1111-111111111112','11111111-1111-1111-1111-111111111111','kind-mist','test/kind-mist','DATA_DIR_PLACEHOLDER/workspaces/conduit/kind-mist',strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),0);

INSERT OR REPLACE INTO app_state(key,value,updated_at) VALUES('sidebar_visible','false',strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));

INSERT INTO session_tabs (
    id, tab_index, is_open, workspace_id, agent_type, agent_mode,
    created_at, queued_messages, input_history
) VALUES
  ('22222222-2222-2222-2222-222222222221', 0, 1, '11111111-1111-1111-1111-111111111112', 'codex', 'build',
   strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), '[]', '[]');
EOF_SQL

python3 - <<PY
import sqlite3
from pathlib import Path

db = Path("$DB_PATH")
data_dir = Path("$DATA_DIR")

conn = sqlite3.connect(db)
cur = conn.cursor()
cur.execute("UPDATE workspaces SET path = REPLACE(path, 'DATA_DIR_PLACEHOLDER', ?)", (str(data_dir),))
cur.execute("UPDATE repositories SET base_path = REPLACE(base_path, 'DATA_DIR_PLACEHOLDER', ?)", (str(data_dir),))
conn.commit()
conn.close()
PY

sock=""
trap 'cleanup_local "$sock"' EXIT

sock=$(start_conduit "$DATA_DIR" 200 40)
wait_idle "$sock" 500 5000 > /dev/null

assert_contains "$sock" "kind-mist" "Workspace tab active on startup"

# Open slash menu by typing "/" — built-in commands (/model, /new, etc.) always appear
type_text "$sock" "/"
wait_idle "$sock" 100 2000 > /dev/null

# Built-in commands include /model and /new; at least one should be visible
assert_contains "$sock" "/model" "Slash menu shows /model command"
assert_contains "$sock" "/new" "Slash menu shows /new command"

# Dismiss with Escape
press "$sock" "Escape"
wait_idle "$sock" 300 3000 > /dev/null

assert_not_contains "$sock" "/model" "Slash menu gone after Escape"

# Clear the input box (Escape leaves "/" in input), then test filter
ctrl "$sock" "u"
wait_idle "$sock" 100 1000 > /dev/null

# Type "/mod" — should filter to /model only
type_text "$sock" "/mod"
wait_idle "$sock" 200 3000 > /dev/null

assert_contains "$sock" "/model" "/model visible when filtering with /mod"
assert_not_contains "$sock" "/new" "/new hidden when filtering with /mod"

# Dismiss
press "$sock" "Escape"
wait_idle "$sock" 300 3000 > /dev/null
ctrl "$sock" "u"
wait_idle "$sock" 100 1000 > /dev/null

# Open command palette with Ctrl+P
ctrl "$sock" "p"
wait_idle "$sock" 300 3000 > /dev/null

assert_contains "$sock" "Command Palette" "Command palette dialog appeared"

# Dismiss with Escape
press "$sock" "Escape"
wait_idle "$sock" 300 3000 > /dev/null

assert_not_contains "$sock" "Command Palette" "Command palette gone after Escape"

log_pass "Slash commands: menu, filtering, and command palette all work correctly"
