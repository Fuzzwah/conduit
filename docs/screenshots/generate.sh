#!/usr/bin/env bash
# Generate all mock screenshots for README.md and FORK_CHANGES.md.
#
# Prerequisites:
#   VHS — https://github.com/charmbracelet/vhs
#     Homebrew:  brew install vhs
#     Releases:  https://github.com/charmbracelet/vhs/releases
#
#   ffmpeg — composites VHS frame layers into a single PNG
#     Homebrew:  brew install ffmpeg
#
#   Playwright (installed automatically via npm ci below)
#
# Usage:
#   bash docs/screenshots/generate.sh

set -e
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

if ! command -v vhs &>/dev/null; then
  echo "Error: 'vhs' not found. Install it first:" >&2
  echo "  Homebrew:  brew install vhs" >&2
  echo "  Releases:  https://github.com/charmbracelet/vhs/releases" >&2
  exit 1
fi

if ! command -v ffmpeg &>/dev/null; then
  echo "Error: 'ffmpeg' not found." >&2
  echo "  Homebrew:  brew install ffmpeg" >&2
  exit 1
fi

# Patch a tape file to replace placeholder PROJECT_ROOT with the actual path.
# VHS 0.x always writes frame directories to /tmp/<output-name>.png/ regardless
# of CWD; the Output directive only sets the directory name under /tmp.
run_tape_and_compose() {
  local tape_rel="$1"   # tape path relative to project root
  local dest="$2"       # destination PNG, relative to project root

  local tape_name
  tape_name="$(basename "$tape_rel")"
  local out_name
  out_name="$(basename "$dest")"

  # Materialise a patched tape with the real project root substituted in
  local patched_tape
  patched_tape="$(mktemp /tmp/vhs-XXXXXX.tape)"
  sed "s|PROJECT_ROOT|${ROOT}|g" "$tape_rel" > "$patched_tape"

  # VHS only creates frames when CWD is /tmp; frame dir is /tmp/<output-name>/
  local frames_dir="/tmp/${out_name}"
  rm -rf "$frames_dir"

  (cd /tmp && vhs "$patched_tape")
  rm -f "$patched_tape"

  # Find the last frame
  local last_text
  last_text="$(ls "$frames_dir"/frame-text-*.png 2>/dev/null | sort | tail -1)"
  if [[ -z "$last_text" ]]; then
    echo "Error: VHS produced no frames for $tape_rel" >&2
    exit 1
  fi
  local idx="${last_text##*frame-text-}"
  idx="${idx%.png}"
  local cursor_frame="$frames_dir/frame-cursor-${idx}.png"

  # Composite text + cursor layers into a single PNG
  if [[ -f "$cursor_frame" ]]; then
    ffmpeg -y -i "$last_text" -i "$cursor_frame" \
      -filter_complex "overlay=0:0" -frames:v 1 "$dest" -loglevel error
  else
    ffmpeg -y -i "$last_text" -frames:v 1 "$dest" -loglevel error
  fi

  rm -rf "$frames_dir"
  echo "    written: $dest"
}

echo "==> Building conduit binary..."
cargo build

echo "==> Generating TUI screenshots with VHS..."
run_tape_and_compose docs/screenshots/tapes/tui-main.tape        docs/screenshots/tui-main.png
run_tape_and_compose docs/screenshots/tapes/tui-ahead-behind.tape docs/screenshots/tui-ahead-behind.png

echo "==> Generating web UI screenshot with Playwright..."
cd web
npm ci --silent
if npx playwright install chromium --with-deps 2>/dev/null; then
  : # system deps installed
else
  echo "  (--with-deps failed; trying without — install libnspr4, libnss3, libgbm1 if this fails)"
  npx playwright install chromium
fi
npx playwright test screenshots.spec.ts --reporter=line
cd ..
echo "    written: docs/screenshots/web-main.png"

echo ""
echo "Done. Screenshots written to docs/screenshots/:"
ls -lh docs/screenshots/*.png
