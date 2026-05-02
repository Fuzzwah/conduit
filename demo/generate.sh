#!/usr/bin/env bash
# Generate all demo GIFs by running each tape in order.
# Must be run from the demo/ directory (or it will cd there automatically).
set -euo pipefail

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DEMO_DIR"

TAPES=(
    01-add-project.tape
    02-create-workspace.tape
    03-make-change.tape
    04-commit.tape
    05-pr.tape
    06-merge-archive.tape
)

echo "==> Seeding demo fixtures..."
bash seed.sh

echo ""
echo "==> Generating GIFs..."

pass=0
fail=0
failed_clips=()

for tape in "${TAPES[@]}"; do
    name="${tape%.tape}"
    echo ""
    echo "--- $name ---"
    if vhs "$tape"; then
        echo "[ok] output/$name.gif"
        pass=$((pass + 1))
    else
        echo "[FAIL] $tape"
        fail=$((fail + 1))
        failed_clips+=("$tape")
    fi
done

echo ""
echo "==> Results: $pass passed, $fail failed"

if [ "${#failed_clips[@]}" -gt 0 ]; then
    echo ""
    echo "Failed clips:"
    for clip in "${failed_clips[@]}"; do
        echo "  - $clip"
    done
fi

echo ""
echo "==> Checking GIF sizes..."
for tape in "${TAPES[@]}"; do
    gif="output/${tape%.tape}.gif"
    if [ -f "$gif" ]; then
        size_kb=$(du -k "$gif" | cut -f1)
        size_mb=$(awk "BEGIN {printf \"%.1f\", $size_kb/1024}")
        warn=""
        [ "$size_kb" -gt 8192 ] && warn=" [WARNING: exceeds 8 MB — tune PlaybackSpeed/Sleep]"
        echo "  $gif: ${size_mb} MB${warn}"
    fi
done

[ "$fail" -eq 0 ]
