#!/bin/sh
# Conduit build-dependency preflight check
# Usage: bash scripts/preflight.sh
#
# Checks all tools required to build conduit from source and prints
# copy-paste install commands for any missing or outdated dependencies.

RUST_MIN_MAJOR=1
RUST_MIN_MINOR=87
NODE_MIN_MAJOR=18

FAILED=0

# ── Color helpers ───────────────────────────────────────────────────────────

if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    NC='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    NC=''
fi

info()    { printf "${BLUE}[INFO]${NC}    %s\n" "$1"; }
ok()      { printf "${GREEN}[OK]${NC}      %s\n" "$1"; }
warn()    { printf "${YELLOW}[WARN]${NC}    %s\n" "$1"; }
missing() { printf "${RED}[MISSING]${NC} %s\n" "$1"; FAILED=1; }
outdated(){ printf "${RED}[OUTDATED]${NC} %s\n" "$1"; FAILED=1; }

# ── OS / package-manager detection ──────────────────────────────────────────

OS="$(uname -s)"
PM=""

case "$OS" in
    Linux*)
        if command -v brew >/dev/null 2>&1; then
            PM="brew"
        elif command -v apt-get >/dev/null 2>&1; then
            PM="apt"
        elif command -v dnf >/dev/null 2>&1; then
            PM="dnf"
        elif command -v pacman >/dev/null 2>&1; then
            PM="pacman"
        fi
        ;;
    Darwin*)
        PM="brew"
        ;;
esac

pkg_install() {
    # $1 = package name
    local pkg="$1"
    case "$PM" in
        brew)    echo "    brew install $pkg" ;;
        apt)     echo "    sudo apt-get install -y $pkg" ;;
        dnf)     echo "    sudo dnf install -y $pkg" ;;
        pacman)  echo "    sudo pacman -S $pkg" ;;
        *)       echo "    (install $pkg via your system package manager)" ;;
    esac
}

rust_install_hint() {
    if command -v rustup >/dev/null 2>&1; then
        echo "    rustup update stable"
    else
        echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    fi
}

node_install_hint() {
    case "$PM" in
        brew)   echo "    brew install node" ;;
        apt)    echo "    curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash - && sudo apt-get install -y nodejs" ;;
        dnf)    echo "    sudo dnf install -y nodejs" ;;
        pacman) echo "    sudo pacman -S nodejs npm" ;;
        *)      echo "    https://nodejs.org/en/download/" ;;
    esac
}

# ── Version comparison helpers ───────────────────────────────────────────────

# Returns 0 if major.minor >= required_major.required_minor
version_ge() {
    local maj="$1" min="$2" req_maj="$3" req_min="$4"
    if [ "$maj" -gt "$req_maj" ]; then return 0; fi
    if [ "$maj" -eq "$req_maj" ] && [ "$min" -ge "$req_min" ]; then return 0; fi
    return 1
}

# ── Checks ───────────────────────────────────────────────────────────────────

echo ""
info "Checking conduit build dependencies..."
echo ""

# git
if command -v git >/dev/null 2>&1; then
    git_ver="$(git --version | awk '{print $3}')"
    ok "git $git_ver"
else
    missing "git — not found"
    pkg_install git
fi

# rustc
if command -v rustc >/dev/null 2>&1; then
    rustc_ver="$(rustc --version | awk '{print $2}')"
    rustc_maj="$(echo "$rustc_ver" | cut -d. -f1)"
    rustc_min="$(echo "$rustc_ver" | cut -d. -f2)"
    if version_ge "$rustc_maj" "$rustc_min" "$RUST_MIN_MAJOR" "$RUST_MIN_MINOR"; then
        ok "rustc $rustc_ver"
    else
        outdated "rustc $rustc_ver (need >= $RUST_MIN_MAJOR.$RUST_MIN_MINOR)"
        rust_install_hint
    fi
else
    missing "rustc — not found (need >= $RUST_MIN_MAJOR.$RUST_MIN_MINOR)"
    rust_install_hint
fi

# cargo
if command -v cargo >/dev/null 2>&1; then
    cargo_ver="$(cargo --version | awk '{print $2}')"
    cargo_maj="$(echo "$cargo_ver" | cut -d. -f1)"
    cargo_min="$(echo "$cargo_ver" | cut -d. -f2)"
    if version_ge "$cargo_maj" "$cargo_min" "$RUST_MIN_MAJOR" "$RUST_MIN_MINOR"; then
        ok "cargo $cargo_ver"
    else
        outdated "cargo $cargo_ver (need >= $RUST_MIN_MAJOR.$RUST_MIN_MINOR)"
        rust_install_hint
    fi
else
    missing "cargo — not found (need >= $RUST_MIN_MAJOR.$RUST_MIN_MINOR)"
    rust_install_hint
fi

# node
if command -v node >/dev/null 2>&1; then
    node_ver="$(node -v | sed 's/^v//')"
    node_maj="$(echo "$node_ver" | cut -d. -f1)"
    if [ "$node_maj" -ge "$NODE_MIN_MAJOR" ]; then
        ok "node v$node_ver"
    else
        outdated "node v$node_ver (need >= $NODE_MIN_MAJOR)"
        node_install_hint
    fi
else
    missing "node — not found (need >= v$NODE_MIN_MAJOR)"
    node_install_hint
fi

# npm
if command -v npm >/dev/null 2>&1; then
    npm_ver="$(npm --version)"
    ok "npm $npm_ver"
else
    missing "npm — not found"
    node_install_hint
fi

# ── Agent CLI (non-fatal) ────────────────────────────────────────────────────

echo ""
AGENT_FOUND=0
for agent in claude codex gemini opencode copilot pi dirac; do
    if command -v "$agent" >/dev/null 2>&1; then
        AGENT_FOUND=1
        ok "agent: $agent found"
        break
    fi
done

if [ "$AGENT_FOUND" -eq 0 ]; then
    warn "No agent CLI found (claude/codex/gemini/opencode/copilot/pi/dirac)"
    echo "    Install at least one agent CLI to use conduit."
fi

# ── Result ───────────────────────────────────────────────────────────────────

echo ""
if [ "$FAILED" -eq 0 ]; then
    ok "All required build dependencies satisfied. You're good to go!"
    echo ""
    echo "    cargo build --release"
    echo ""
    exit 0
else
    printf "${RED}[FAIL]${NC}    One or more required dependencies are missing or outdated.\n"
    echo "           Install the items listed above, then re-run this script."
    echo ""
    exit 1
fi
