#!/bin/bash
# Test: Session tab persistence should not duplicate tabs across restarts.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

if ! command -v termwright >/dev/null 2>&1; then
  echo "termwright not found. Install with: cargo install termwright" >&2
  exit 1
fi

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "sqlite3 not found. Install with: brew install sqlite3" >&2
  exit 1
fi

if [ ! -f "$PROJECT_ROOT/target/release/conduit" ]; then
  echo "Building conduit release binary..."
  (cd "$PROJECT_ROOT" && cargo build --release)
fi

DATA_DIR=$(mktemp -d /tmp/conduit-test-tabs-XXXXXX)
ARTIFACT_DIR="$SCRIPT_DIR/artifacts/session-tabs"
mkdir -p "$ARTIFACT_DIR"

STEP_FILE_1=""
STEP_FILE_2=""

cleanup() {
  if [ -n "$STEP_FILE_1" ] && [ -f "$STEP_FILE_1" ]; then
    rm -f "$STEP_FILE_1"
  fi
  if [ -n "$STEP_FILE_2" ] && [ -f "$STEP_FILE_2" ]; then
    rm -f "$STEP_FILE_2"
  fi
  rm -rf "$DATA_DIR"
}
trap cleanup EXIT

# Build data dir structure
mkdir -p "$DATA_DIR/workspaces/conduit"
for ws in kind-mist live-jade pale-snow trim-moss; do
  mkdir -p "$DATA_DIR/workspaces/conduit/$ws"
done

# Provide a dummy codex executable to satisfy tool detection via PATH
mkdir -p "$DATA_DIR/bin"
cat > "$DATA_DIR/bin/codex" <<'EOF'
#!/bin/sh
exit 0
EOF
chmod +x "$DATA_DIR/bin/codex"

# Expand PATH for env injection in step files
ENV_PATH="$DATA_DIR/bin:$PATH"

# Write a minimal config to satisfy tool checks
cat > "$DATA_DIR/config.toml" <<EOF
[tools]
codex = "$DATA_DIR/bin/codex"
EOF

# Create database schema + seed repositories/workspaces
DB_PATH="$DATA_DIR/conduit.db"
sqlite3 "$DB_PATH" <<'SQL'
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

CREATE TABLE IF NOT EXISTS fork_seeds (
    id TEXT PRIMARY KEY,
    agent_type TEXT NOT NULL,
    parent_session_id TEXT,
    parent_workspace_id TEXT,
    created_at TEXT NOT NULL,
    seed_prompt_hash TEXT NOT NULL,
    seed_prompt_path TEXT,
    token_estimate INTEGER NOT NULL,
    context_window INTEGER NOT NULL,
    seed_ack_filtered INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (parent_workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);

INSERT INTO repositories (
    id, name, base_path, repository_url, workspace_mode,
    archive_delete_branch, archive_remote_prompt, created_at, updated_at
) VALUES (
    '11111111-1111-1111-1111-111111111111',
    'conduit',
    '/tmp/conduit-test',
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
  ('11111111-1111-1111-1111-111111111113','11111111-1111-1111-1111-111111111111','live-jade','test/live-jade','DATA_DIR_PLACEHOLDER/workspaces/conduit/live-jade',strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),0),
  ('11111111-1111-1111-1111-111111111114','11111111-1111-1111-1111-111111111111','pale-snow','test/pale-snow','DATA_DIR_PLACEHOLDER/workspaces/conduit/pale-snow',strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),0),
  ('11111111-1111-1111-1111-111111111115','11111111-1111-1111-1111-111111111111','trim-moss','test/trim-moss','DATA_DIR_PLACEHOLDER/workspaces/conduit/trim-moss',strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),0);

INSERT OR REPLACE INTO app_state(key,value,updated_at) VALUES('sidebar_visible','false',strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));
INSERT OR REPLACE INTO app_state(key,value,updated_at) VALUES('tree_collapsed_repos','',strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));
INSERT OR REPLACE INTO app_state(key,value,updated_at) VALUES('tree_selected_index','0',strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));

INSERT INTO session_tabs (
    id, tab_index, is_open, workspace_id, agent_type, agent_mode, agent_session_id,
    model, pr_number, created_at, pending_user_message, queued_messages, input_history,
    fork_seed_id, title
) VALUES
  ('22222222-2222-2222-2222-222222222221', 0, 1, '11111111-1111-1111-1111-111111111112', 'codex', 'build', NULL, NULL, NULL, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), NULL, '[]', '[]', NULL, NULL),
  ('22222222-2222-2222-2222-222222222222', 1, 1, '11111111-1111-1111-1111-111111111113', 'codex', 'build', NULL, NULL, NULL, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), NULL, '[]', '[]', NULL, NULL),
  ('22222222-2222-2222-2222-222222222223', 2, 1, '11111111-1111-1111-1111-111111111114', 'codex', 'build', NULL, NULL, NULL, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), NULL, '[]', '[]', NULL, NULL),
  ('22222222-2222-2222-2222-222222222224', 3, 1, '11111111-1111-1111-1111-111111111115', 'codex', 'build', NULL, NULL, NULL, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), NULL, '[]', '[]', NULL, NULL);
SQL

# Replace placeholder path with actual data dir path
python3 - <<PY
import sqlite3
from pathlib import Path

db = Path("$DB_PATH")
data_dir = Path("$DATA_DIR")

conn = sqlite3.connect(db)
cur = conn.cursor()
cur.execute("UPDATE workspaces SET path = REPLACE(path, 'DATA_DIR_PLACEHOLDER', ?)", (str(data_dir),))
conn.commit()
conn.close()
PY

STEP_FILE_1=$(mktemp /tmp/test-tabs-open-XXXXXXXX)
STEP_FILE_2=$(mktemp /tmp/test-tabs-reopen-XXXXXXXX)

cat > "$STEP_FILE_1" << EOF
session:
  command: ["/usr/bin/env", "PATH=$ENV_PATH", "$PROJECT_ROOT/target/release/conduit", "--data-dir", "$DATA_DIR"]
  cols: 200
  rows: 40

steps:
  - waitForIdle: {idleMs: 1000, timeoutMs: 10000}
  - waitForText: {text: "(kind-mist)", timeoutMs: 5000}
  - expectText: {text: "(live-jade)"}
  - expectText: {text: "(pale-snow)"}
  - expectText: {text: "(trim-moss)"}

  # Quit
  - hotkey: {ctrl: true, ch: "q"}
  - waitForIdle: {idleMs: 500}

artifacts:
  mode: onFailure
  dir: $ARTIFACT_DIR
EOF

cat > "$STEP_FILE_2" << EOF
session:
  command: ["/usr/bin/env", "PATH=$ENV_PATH", "$PROJECT_ROOT/target/release/conduit", "--data-dir", "$DATA_DIR"]
  cols: 200
  rows: 40

steps:
  - waitForIdle: {idleMs: 1000, timeoutMs: 10000}
  - waitForText: {text: "(kind-mist)", timeoutMs: 5000}
  - expectText: {text: "(live-jade)"}
  - expectText: {text: "(pale-snow)"}
  - expectText: {text: "(trim-moss)"}
  - expectText: {text: "(live-jade)"}
  - expectText: {text: "(pale-snow)"}
  - expectText: {text: "(trim-moss)"}

  # Quit
  - hotkey: {ctrl: true, ch: "q"}
  - waitForIdle: {idleMs: 500}

artifacts:
  mode: onFailure
  dir: $ARTIFACT_DIR
EOF

echo "Running session tab persistence test (first run)..."
termwright run-steps --trace "$STEP_FILE_1"

open_count=$(sqlite3 "$DB_PATH" "select count(*) from session_tabs where is_open=1;")
if [ "$open_count" != "4" ]; then
  echo "Expected 4 open session tabs after first run, got $open_count" >&2
  exit 1
fi

dupes=$(sqlite3 "$DB_PATH" "select count(*) from (select workspace_id, count(*) c from session_tabs where is_open=1 group by workspace_id having c != 1);")
if [ "$dupes" != "0" ]; then
  echo "Expected exactly 1 open tab per workspace after first run" >&2
  exit 1
fi

echo "Running session tab persistence test (second run)..."
termwright run-steps --trace "$STEP_FILE_2"

open_count=$(sqlite3 "$DB_PATH" "select count(*) from session_tabs where is_open=1;")
if [ "$open_count" != "4" ]; then
  echo "Expected 4 open session tabs after second run, got $open_count" >&2
  exit 1
fi

dupes=$(sqlite3 "$DB_PATH" "select count(*) from (select workspace_id, count(*) c from session_tabs where is_open=1 group by workspace_id having c != 1);")
if [ "$dupes" != "0" ]; then
  echo "Expected exactly 1 open tab per workspace after second run" >&2
  exit 1
fi

echo "Session tab persistence test passed."
