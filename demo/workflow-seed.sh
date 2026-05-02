#!/usr/bin/env bash
# Seed a clean fixture environment for the single workflow demo.
# Always starts from scratch so the recording is deterministic.
set -euo pipefail

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES="$DEMO_DIR/fixtures"

echo "==> Tearing down any existing fixtures..."
bash "$DEMO_DIR/teardown.sh"

echo ""
echo "==> Building base fixtures..."
bash "$DEMO_DIR/seed.sh"

echo ""
echo "==> Adding OpenSpec spec to project..."

PROJECT="$FIXTURES/project"

mkdir -p "$PROJECT/openspec/changes/update-readme"

cat > "$PROJECT/openspec/changes/update-readme/tasks.md" << 'TASKS'
## Update README

- [ ] Add a "## Recent Updates" section to README.md
- [ ] Add a brief description of the project to the README
TASKS

# Commit the spec to the repo so conduit can read it via git ls-tree
git -C "$PROJECT" add openspec/
git -C "$PROJECT" \
    -c user.email=demo@example.com \
    -c user.name="Demo User" \
    commit -q -m "feat: add update-readme spec"
git -C "$PROJECT" push -q origin main

echo "    [ok] update-readme spec committed and pushed to origin"

echo ""
echo "==> Workflow demo ready."
echo "    Run: cd demo && LD_LIBRARY_PATH=/home/linuxbrew/.linuxbrew/lib vhs workflow.tape"
