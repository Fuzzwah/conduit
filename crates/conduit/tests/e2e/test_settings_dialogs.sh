#!/bin/bash
# Test: Settings dialogs — model selector (Ctrl+O), theme picker (Alt+T), session import (Alt+I)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "sqlite3 not found. Install with: brew install sqlite3" >&2
  exit 1
fi

if [ ! -f "$PROJECT_ROOT/target/release/conduit" ]; then
  echo "Building conduit release binary..."
  (cd "$PROJECT_ROOT" && cargo build --release)
fi

DATA_DIR=$(create_data_dir "settings-dialogs")

cleanup_local() {
  local sock="$1"
  if [ -n "$sock" ]; then
    close_daemon "$sock"
  fi
  rm -rf "$DATA_DIR"
}

mkdir -p "$DATA_DIR/workspaces/conduit"

(
  cd "$DATA_DIR/workspaces/conduit"
  git init
  git config user.email "test@test.com"
  git config user.name "Test"
  touch .gitkeep
  git add .gitkeep
  git commit -m "initial"
  git branch test/kind-mist
  git worktree add kind-mist test/kind-mist
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

# Model selector: opens on Ctrl+O, closes on Escape
ctrl "$sock" "o"
wait_idle "$sock" 300 5000 > /dev/null

assert_contains "$sock" "Model" "Model selector dialog appeared"

press "$sock" "Escape"
wait_idle "$sock" 300 3000 > /dev/null

assert_not_contains "$sock" "Model" "Model selector gone after Escape"

# Theme picker: opens on Alt+T, closes on Escape
alt "$sock" "t"
wait_idle "$sock" 300 5000 > /dev/null

assert_contains "$sock" "Theme" "Theme picker dialog appeared"

press "$sock" "Escape"
wait_idle "$sock" 300 3000 > /dev/null

assert_not_contains "$sock" "Theme" "Theme picker gone after Escape"

# Session import picker: opens on Alt+I, closes on Escape
alt "$sock" "i"
wait_idle "$sock" 300 5000 > /dev/null

assert_contains "$sock" "Import Session" "Session import dialog appeared"

press "$sock" "Escape"
wait_idle "$sock" 300 3000 > /dev/null

assert_not_contains "$sock" "Import Session" "Session import gone after Escape"

log_pass "Settings dialogs: model selector, theme picker, and session import work correctly"
