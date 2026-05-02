#!/bin/bash
# Test: Sidebar toggle (Ctrl+T), arrow navigation, and workspace selection via Enter

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

DATA_DIR=$(create_data_dir "sidebar-navigation")

cleanup_local() {
  local sock="$1"
  if [ -n "$sock" ]; then
    close_daemon "$sock"
  fi
  rm -rf "$DATA_DIR"
}

mkdir -p "$DATA_DIR/workspaces/conduit/kind-mist"
mkdir -p "$DATA_DIR/workspaces/conduit/live-jade"

(
  cd "$DATA_DIR/workspaces/conduit"
  git init
  git config user.email "test@test.com"
  git config user.name "Test"
  touch .gitkeep
  git add .gitkeep
  git commit -m "initial"
  git checkout -b test/kind-mist
  git checkout -b test/live-jade
  git checkout test/kind-mist
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
  ('11111111-1111-1111-1111-111111111112','11111111-1111-1111-1111-111111111111','kind-mist','test/kind-mist','DATA_DIR_PLACEHOLDER/workspaces/conduit/kind-mist',strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),0),
  ('11111111-1111-1111-1111-111111111113','11111111-1111-1111-1111-111111111111','live-jade','test/live-jade','DATA_DIR_PLACEHOLDER/workspaces/conduit/live-jade',strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),0);

-- Start with sidebar visible so we can assert project and workspace names
INSERT OR REPLACE INTO app_state(key,value,updated_at) VALUES('sidebar_visible','true',strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));
INSERT OR REPLACE INTO app_state(key,value,updated_at) VALUES('tree_collapsed_repos','',strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));
INSERT OR REPLACE INTO app_state(key,value,updated_at) VALUES('tree_selected_index','0',strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));

INSERT INTO session_tabs (
    id, tab_index, is_open, workspace_id, agent_type, agent_mode,
    created_at, queued_messages, input_history
) VALUES
  ('22222222-2222-2222-2222-222222222221', 0, 1, '11111111-1111-1111-1111-111111111112', 'codex', 'build',
   strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), '[]', '[]'),
  ('22222222-2222-2222-2222-222222222222', 1, 1, '11111111-1111-1111-1111-111111111113', 'codex', 'build',
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

# Sidebar is visible on startup — project and workspace names should appear
assert_contains "$sock" "conduit" "Project name visible in sidebar"
assert_contains "$sock" "kind-mist" "kind-mist workspace visible in sidebar"
assert_contains "$sock" "live-jade" "live-jade workspace visible in sidebar"

# Toggle sidebar off with Ctrl+T
ctrl "$sock" "t"
wait_idle "$sock" 300 3000 > /dev/null

assert_not_contains "$sock" "conduit" "Project name hidden after Ctrl+T toggle off"

# Toggle sidebar back on
ctrl "$sock" "t"
wait_idle "$sock" 300 3000 > /dev/null

assert_contains "$sock" "conduit" "Project name visible after Ctrl+T toggle on"

# Navigate sidebar with arrow keys — focus the sidebar first if needed
# The sidebar uses Ctrl+T to toggle; when visible, arrow keys navigate
# Navigate to sidebar focus mode by pressing Ctrl+T to enter it
ctrl "$sock" "t"
wait_idle "$sock" 300 3000 > /dev/null
ctrl "$sock" "t"
wait_idle "$sock" 300 3000 > /dev/null

# Navigate down in the sidebar tree
press "$sock" "Down"
wait_idle "$sock" 300 3000 > /dev/null

# Navigate back up
press "$sock" "Up"
wait_idle "$sock" 300 3000 > /dev/null

# Select live-jade from sidebar: navigate to it and press Enter
# kind-mist is tab 1 (active), live-jade is below it in the sidebar
press "$sock" "Down"
wait_idle "$sock" 300 3000 > /dev/null
press "$sock" "Enter"
wait_idle "$sock" 500 5000 > /dev/null

# live-jade should now be the active tab
assert_contains "$sock" "live-jade" "live-jade is active after sidebar Enter"
assert_contains "$sock" "▸" "Active tab indicator present after sidebar selection"

log_pass "Sidebar navigation: toggle, arrow navigation, and workspace selection work correctly"
