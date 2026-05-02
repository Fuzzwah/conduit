#!/usr/bin/env bash
# Seed a clean fixture environment for the single workflow demo.
# Always starts from scratch so the recording is deterministic.
set -euo pipefail

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES="$DEMO_DIR/fixtures"
TEMPLATE="$DEMO_DIR/project-template"

echo "==> Tearing down any existing fixtures..."
bash "$DEMO_DIR/teardown.sh"

echo ""
echo "==> Building base fixtures (db, config, gh shim, symlink)..."
bash "$DEMO_DIR/seed.sh"

echo ""
echo "==> Populating project from template..."

# seed.sh creates a stub project clone with a single "Initial commit".
# Replace it entirely with the template history.
rm -rf "$FIXTURES/project"

TMP=$(mktemp -d)
git -C "$TMP" init -q
git -C "$TMP" checkout -q -b main
git -C "$TMP" config user.email "demo@example.com"
git -C "$TMP" config user.name "Demo User"

# Commit 1: source files
cp "$TEMPLATE/Cargo.toml" "$TMP/"
mkdir -p "$TMP/src"
cp "$TEMPLATE/src/main.rs" "$TMP/src/"
git -C "$TMP" add Cargo.toml src/
git -C "$TMP" -c user.email=demo@example.com -c user.name="Demo User" \
    commit -q -m "feat: add greet CLI"

# Commit 2: README
cp "$TEMPLATE/README.md" "$TMP/"
git -C "$TMP" add README.md
git -C "$TMP" -c user.email=demo@example.com -c user.name="Demo User" \
    commit -q -m "docs: add README"

# Commit 3: OpenSpec change (all four artifacts)
cp -r "$TEMPLATE/openspec" "$TMP/"
git -C "$TMP" add openspec/
git -C "$TMP" -c user.email=demo@example.com -c user.name="Demo User" \
    commit -q -m "feat: add update-readme spec"

# Force-push to replace the bare remote's stub history
git -C "$TMP" remote add origin "$FIXTURES/remote.git"
git -C "$TMP" push -q --force origin main
echo "    [ok] pushed 3 commits to origin"

# Clone as the working project
git clone -q "$FIXTURES/remote.git" "$FIXTURES/project"
git -C "$FIXTURES/project" config user.email "demo@example.com"
git -C "$FIXTURES/project" config user.name "Demo User"
echo "    [ok] project clone ($(git -C "$FIXTURES/project" log --oneline | wc -l | tr -d ' ') commits)"

rm -rf "$TMP"

echo ""
echo "==> Workflow demo ready."
echo "    Run: cd demo && LD_LIBRARY_PATH=/home/linuxbrew/.linuxbrew/lib vhs workflow.tape"
