#!/usr/bin/env bash
# Seed demo fixtures for VHS workflow recording.
# Safe to re-run: skips steps whose output already exists.
set -euo pipefail

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES="$DEMO_DIR/fixtures"

echo "==> Seeding demo fixtures in $FIXTURES"

# 1. Bare remote repo
if [ ! -d "$FIXTURES/remote.git" ]; then
    TMP=$(mktemp -d)
    git -C "$TMP" init -q
    git -C "$TMP" checkout -q -b main
    echo "# Demo Project" > "$TMP/README.md"
    git -C "$TMP" add README.md
    git -C "$TMP" -c user.email=demo@example.com -c user.name="Demo User" \
        commit -q -m "Initial commit"
    git init --bare -q "$FIXTURES/remote.git"
    git -C "$FIXTURES/remote.git" symbolic-ref HEAD refs/heads/main
    git -C "$TMP" remote add origin "$FIXTURES/remote.git"
    git -C "$TMP" push -q origin main
    rm -rf "$TMP"
    echo "    [ok] remote.git"
else
    echo "    [skip] remote.git already exists"
fi

# 2. Working clone
if [ ! -d "$FIXTURES/project" ]; then
    git clone -q "$FIXTURES/remote.git" "$FIXTURES/project"
    git -C "$FIXTURES/project" config user.email "demo@example.com"
    git -C "$FIXTURES/project" config user.name "Demo User"
    echo "    [ok] project clone"
else
    echo "    [skip] project clone already exists"
fi

# 3. gh shim
mkdir -p "$FIXTURES/bin"
if [ ! -f "$FIXTURES/bin/gh" ]; then
    cat > "$FIXTURES/bin/gh" << 'SHIM'
#!/usr/bin/env bash
# Demo gh shim — no network required
if [ "${1-}" = "--version" ]; then
    echo "gh version 2.0.0-demo (2024-01-01)"
    exit 0
elif [ "${1-}" = "auth" ] && [ "${2-}" = "status" ]; then
    echo "Logged in to github.com as demo (demo)"
    exit 0
elif [ "${1-}" = "pr" ] && [ "${2-}" = "create" ]; then
    echo "https://github.com/demo/project/pull/1"
    exit 0
elif [ "${1-}" = "pr" ] && [ "${2-}" = "merge" ]; then
    exit 0
else
    echo "gh: not implemented in demo shim: $*" >&2
    exit 1
fi
SHIM
    chmod +x "$FIXTURES/bin/gh"
    echo "    [ok] gh shim"
else
    echo "    [skip] gh shim already exists"
fi

# 4. Stable symlink so tape scripts can reference fixtures at a known absolute path
SYMLINK="/tmp/conduit-demo"
if [ ! -L "$SYMLINK" ] || [ "$(readlink "$SYMLINK")" != "$DEMO_DIR" ]; then
    ln -sfn "$DEMO_DIR" "$SYMLINK"
    echo "    [ok] symlink $SYMLINK -> $DEMO_DIR"
else
    echo "    [skip] symlink already correct"
fi

# 5. Conduit config — pre-configure providers and model to skip first-launch onboarding
mkdir -p "$FIXTURES/data"
if [ ! -f "$FIXTURES/data/config.toml" ]; then
    cat > "$FIXTURES/data/config.toml" << CONFIG
[providers]
enabled = ["claude", "codex", "gemini", "opencode", "copilot", "pi"]

[model]
agent = "claude"
model = "sonnet"

[tools]
gh = "$FIXTURES/bin/gh"
CONFIG
    echo "    [ok] config.toml (skips onboarding)"
else
    echo "    [skip] config.toml already exists"
fi

# 6. Conduit DB — pre-seed schema + app_state so BaseDirDialog is skipped
if [ ! -f "$FIXTURES/data/conduit.db" ]; then
    sqlite3 "$FIXTURES/data/conduit.db" << SQL
CREATE TABLE repositories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    base_path TEXT,
    repository_url TEXT,
    workspace_mode TEXT,
    archive_delete_branch INTEGER,
    archive_remote_prompt INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    theme_name TEXT,
    position INTEGER
);
CREATE TABLE workspaces (
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
CREATE INDEX idx_workspaces_repository ON workspaces(repository_id);
CREATE TABLE app_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE session_tabs (
    id TEXT PRIMARY KEY,
    tab_index INTEGER NOT NULL,
    is_open INTEGER NOT NULL DEFAULT 1,
    workspace_id TEXT,
    agent_type TEXT NOT NULL,
    agent_mode TEXT DEFAULT 'build',
    agent_session_id TEXT,
    model TEXT,
    model_invalid INTEGER NOT NULL DEFAULT 0,
    pr_number INTEGER,
    created_at TEXT NOT NULL,
    pending_user_message TEXT,
    queued_messages TEXT NOT NULL DEFAULT '[]',
    input_history TEXT NOT NULL DEFAULT '[]',
    fork_seed_id TEXT,
    title TEXT,
    title_generated INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);
CREATE INDEX idx_session_tabs_order ON session_tabs(tab_index);
CREATE INDEX idx_session_tabs_workspace_open
    ON session_tabs(workspace_id, is_open, created_at DESC);
CREATE TABLE fork_seeds (
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
CREATE INDEX idx_fork_seeds_parent_session ON fork_seeds(parent_session_id);

-- Pre-set the projects base directory so Ctrl+N opens the picker directly
INSERT INTO app_state (key, value, updated_at)
VALUES ('projects_base_dir', '$FIXTURES', datetime('now'));
SQL
    echo "    [ok] conduit.db (pre-seeded schema + projects_base_dir)"
else
    echo "    [skip] conduit.db already exists"
fi

# 7. Verify conduit binary
CONDUIT_BIN="$DEMO_DIR/../target/debug/conduit"
if [ ! -f "$CONDUIT_BIN" ]; then
    echo ""
    echo "WARNING: $CONDUIT_BIN not found. Run 'cargo build' first."
else
    "$CONDUIT_BIN" --data-dir "$FIXTURES/data" --help > /dev/null 2>&1 \
        && echo "    [ok] conduit binary works with --data-dir" \
        || echo "    [warn] conduit --data-dir check failed"
fi

echo ""
echo "==> Done. Run 'bash demo/generate.sh' to generate GIFs."
