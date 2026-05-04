#!/bin/bash
# Test: Work Complete dialog — open, review, commit, archive-with-force-confirm

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

DATA_DIR=$(create_data_dir "work-complete")

cleanup_local() {
  local sock="$1"
  if [ -n "$sock" ]; then
    close_daemon "$sock"
  fi
  rm -rf "$DATA_DIR"
}

# --- Git repo setup ---
mkdir -p "$DATA_DIR/workspaces/conduit/kind-mist/openspec/changes/my-spec"

(
  cd "$DATA_DIR/workspaces/conduit"
  git init
  git config user.email "test@test.com"
  git config user.name "Test"
  echo "# Conduit" > README.md
  git add README.md
  git commit -m "initial"
  git checkout -b test/kind-mist
  # Add an openspec change with one incomplete task so the spec section appears
  cat > kind-mist/openspec/changes/my-spec/tasks.md <<'TASKS'
## Tasks
- [ ] Task 1: do something
- [x] Task 2: already done
TASKS
  git add kind-mist/
  git commit -m "add my-spec tasks"
) >/dev/null 2>&1

# Untracked file makes the workspace dirty, triggers the Commit action
echo "work in progress" > "$DATA_DIR/workspaces/conduit/kind-mist/work.txt"

cat > "$DATA_DIR/config.toml" <<EOF_CONFIG
[tools]
codex = "/bin/true"
EOF_CONFIG

# --- Database setup ---
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
    active_change_id TEXT,
    active_issue_number INTEGER,
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

INSERT INTO workspaces (
    id, repository_id, name, branch, path,
    created_at, last_accessed, is_default,
    active_change_id, active_issue_number
) VALUES (
    '11111111-1111-1111-1111-111111111112',
    '11111111-1111-1111-1111-111111111111',
    'kind-mist',
    'test/kind-mist',
    'DATA_DIR_PLACEHOLDER/workspaces/conduit/kind-mist',
    strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
    0,
    'my-spec',
    NULL
);

INSERT OR REPLACE INTO app_state(key, value, updated_at)
VALUES('sidebar_visible', 'false', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));

INSERT INTO session_tabs (
    id, tab_index, is_open, workspace_id, agent_type, agent_mode,
    created_at, queued_messages, input_history
) VALUES (
    '22222222-2222-2222-2222-222222222221',
    0, 1,
    '11111111-1111-1111-1111-111111111112',
    'codex', 'build',
    strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
    '[]', '[]'
);
EOF_SQL

python3 - <<PY
import sqlite3
from pathlib import Path

db   = Path("$DB_PATH")
ddir = Path("$DATA_DIR")

conn = sqlite3.connect(db)
cur  = conn.cursor()
cur.execute("UPDATE workspaces   SET path      = REPLACE(path,      'DATA_DIR_PLACEHOLDER', ?)", (str(ddir),))
cur.execute("UPDATE repositories SET base_path = REPLACE(base_path, 'DATA_DIR_PLACEHOLDER', ?)", (str(ddir),))
conn.commit()
conn.close()
PY

# --- Start conduit ---
sock=""
trap 'cleanup_local "$sock"' EXIT

sock=$(start_conduit "$DATA_DIR" 200 40)
wait_idle "$sock" 500 5000 > /dev/null

assert_contains "$sock" "kind-mist" "Workspace tab visible on startup"

# -----------------------------------------------------------------------
# Step 1: Open Work Complete dialog (Alt+Shift+X → alt "$sock" "X")
# -----------------------------------------------------------------------
alt "$sock" "X"
wait_idle "$sock" 500 8000 > /dev/null

# Give the async preflight a moment to load
sleep 2
wait_idle "$sock" 500 5000 > /dev/null

assert_contains "$sock" "Work Complete" "Work Complete dialog opened"
assert_contains "$sock" "Spec in progress" "Scenario badge shows spec_incomplete"
assert_contains "$sock" "my-spec" "Spec section shows change ID"
assert_contains "$sock" "Commit" "Commit action is available"

# -----------------------------------------------------------------------
# Step 2: Select Commit → enter pre-filled message phase
# -----------------------------------------------------------------------
press "$sock" "Enter"
wait_idle "$sock" 300 5000 > /dev/null

assert_contains "$sock" "Commit Message" "Commit message dialog title"
assert_contains "$sock" "Enter commit message:" "Commit message prompt"
assert_contains "$sock" "Implement my-spec" "Pre-filled message contains change ID"

# -----------------------------------------------------------------------
# Step 3: Accept the pre-filled message → execute commit
# -----------------------------------------------------------------------
press "$sock" "Enter"
wait_idle "$sock" 1000 10000 > /dev/null

# Executing spinner may show briefly; wait for it to settle
sleep 2
wait_idle "$sock" 500 8000 > /dev/null

# After commit the dialog returns to reviewing state (workspace no longer dirty)
assert_contains "$sock" "Work Complete" "Dialog still open after commit"
assert_not_contains "$sock" "modified" "No uncommitted changes after commit"

# -----------------------------------------------------------------------
# Step 4: Cancel and verify workspace tab is still present
# -----------------------------------------------------------------------
press "$sock" "Escape"
wait_idle "$sock" 500 5000 > /dev/null

assert_contains "$sock" "kind-mist" "Workspace tab intact after commit + escape"

# -----------------------------------------------------------------------
# Step 5: Re-open dialog and archive (force-confirm required for spec_incomplete)
# -----------------------------------------------------------------------
alt "$sock" "X"
wait_idle "$sock" 500 8000 > /dev/null
sleep 2
wait_idle "$sock" 500 5000 > /dev/null

assert_contains "$sock" "Work Complete" "Work Complete dialog re-opened"

# Navigate to Archive action (it's the last item; Down arrow gets there)
press "$sock" "Down"
wait_idle "$sock" 200 3000 > /dev/null
press "$sock" "Down"
wait_idle "$sock" 200 3000 > /dev/null
press "$sock" "Down"
wait_idle "$sock" 200 3000 > /dev/null
press "$sock" "Down"
wait_idle "$sock" 200 3000 > /dev/null

press "$sock" "Enter"
wait_idle "$sock" 300 5000 > /dev/null

# Force-confirm dialog should appear (spec still has incomplete tasks)
assert_contains "$sock" "incomplete tasks" "Force-confirm warns about incomplete spec"

# Confirm the force-archive
press "$sock" "Enter"
wait_idle "$sock" 500 10000 > /dev/null
sleep 2
wait_idle "$sock" 500 8000 > /dev/null

assert_not_contains "$sock" "kind-mist" "Workspace tab gone after archive"

log_pass "Work Complete: dialog, commit pre-fill, force-confirm, and archive all work correctly"
