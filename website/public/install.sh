#!/bin/sh
# Conduit Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/Fuzzwah/conduit/master/website/public/install.sh | sh
#
# Environment variables:
#   CONDUIT_VERSION      Pin a specific release (e.g. v0.5.0). Defaults to latest.
#   CONDUIT_INSTALL_DIR  Override install directory. Defaults to ~/.local/bin.
#   CONDUIT_INSTALL_FILE Use a local archive file instead of downloading. Skips GitHub API.

set -e

REPO="Fuzzwah/conduit"
INSTALL_DIR="${CONDUIT_INSTALL_DIR:-$HOME/.local/bin}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() {
    printf "${BLUE}==>${NC} %s\n" "$1"
}

success() {
    printf "${GREEN}==>${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}Warning:${NC} %s\n" "$1"
}

error() {
    printf "${RED}Error:${NC} %s\n" "$1" >&2
    exit 1
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        *)       error "Unsupported operating system: $(uname -s)" ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        arm64|aarch64) echo "aarch64" ;;
        *)             error "Unsupported architecture: $(uname -m)" ;;
    esac
}

# Resolve target triple from OS + arch
resolve_target() {
    local os="$1"
    local arch="$2"

    case "${os}/${arch}" in
        linux/x86_64)   echo "x86_64-unknown-linux-musl" ;;
        linux/aarch64)  echo "aarch64-unknown-linux-musl" ;;
        macos/aarch64)  echo "aarch64-apple-darwin" ;;
        macos/x86_64)   echo "x86_64-apple-darwin" ;;
        *)              error "No pre-built binary for ${os}/${arch}. Build from source: https://github.com/${REPO}#build-from-source" ;;
    esac
}

# Fetch the release tag to install
resolve_version() {
    if [ -n "${CONDUIT_VERSION:-}" ]; then
        echo "$CONDUIT_VERSION"
        return
    fi

    local tag
    tag=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
          | grep '"tag_name":' \
          | sed -E 's/.*"([^"]+)".*/\1/')

    if [ -z "$tag" ]; then
        error "Failed to fetch latest release. Set CONDUIT_VERSION=vX.Y.Z to pin a version."
    fi

    echo "$tag"
}

# Show instructions for building from source
build_from_source() {
    echo ""
    warn "Pre-built binary not available for your platform."
    echo ""
    echo "Build Conduit from source:"
    echo ""
    echo "  bash scripts/preflight.sh   # check dependencies"
    echo "  git clone https://github.com/${REPO}.git"
    echo "  cd conduit && cargo build --release"
    echo "  cp target/release/conduit ~/.local/bin/"
    echo ""
    exit 0
}

# Verify sha256 of a downloaded file
verify_checksum() {
    local archive="$1"
    local sha_file="$2"

    local expected actual
    expected=$(awk '{print $1}' "$sha_file")

    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$archive" | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "$archive" | awk '{print $1}')
    else
        warn "Neither sha256sum nor shasum found — skipping checksum verification."
        return 0
    fi

    if [ "$actual" != "$expected" ]; then
        rm -f "$archive"
        error "Checksum mismatch for $(basename "$archive"). Expected $expected, got $actual. Aborting."
    fi
}

# Download a URL to a path using curl or wget
download() {
    local url="$1"
    local dest="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$url" -O "$dest"
    else
        error "Neither curl nor wget found. Please install one."
    fi
}

# Main installation
main() {
    echo ""
    printf "  ${GREEN} ░██████                               ░██            ░██   ░██${NC}\n"
    printf "  ${GREEN}░██   ░██                              ░██                  ░██${NC}\n"
    printf "  ${GREEN}░██        ░███████  ░████████   ░████████ ░██    ░██ ░██░████████${NC}\n"
    printf "  ${GREEN}░██       ░██    ░██ ░██    ░██ ░██    ░██ ░██    ░██ ░██   ░██${NC}\n"
    printf "  ${GREEN}░██       ░██    ░██ ░██    ░██ ░██    ░██ ░██    ░██ ░██   ░██${NC}\n"
    printf "  ${GREEN}░██   ░██ ░██    ░██ ░██    ░██ ░██   ░███ ░██   ░███ ░██   ░██${NC}\n"
    printf "  ${GREEN} ░██████   ░███████  ░██    ░██  ░█████░██  ░█████░██ ░██    ░████${NC}\n"
    echo ""
    echo "  Multi-Agent TUI for AI Coding Assistants"
    echo ""

    local os arch target version archive tmpdir

    # ── Local-file mode (CI smoke test / offline install) ─────────────────
    if [ -n "${CONDUIT_INSTALL_FILE:-}" ]; then
        if [ ! -f "$CONDUIT_INSTALL_FILE" ]; then
            error "CONDUIT_INSTALL_FILE is set but '$CONDUIT_INSTALL_FILE' does not exist."
        fi
        info "Using local archive: $CONDUIT_INSTALL_FILE"
        archive="$CONDUIT_INSTALL_FILE"
    else
        # ── Remote download mode ─────────────────────────────────────────
        os=$(detect_os)
        arch=$(detect_arch)
        target=$(resolve_target "$os" "$arch")
        version=$(resolve_version)

        info "Detected: $os/$arch → $target"
        info "Version:  $version"

        tmpdir=$(mktemp -d)
        trap "rm -rf '$tmpdir'" EXIT

        local base_url="https://github.com/${REPO}/releases/download/${version}"
        local asset="conduit-${target}.tar.gz"

        info "Downloading $asset..."
        download "${base_url}/${asset}" "$tmpdir/${asset}"

        info "Verifying checksum..."
        download "${base_url}/${asset}.sha256" "$tmpdir/${asset}.sha256"
        verify_checksum "$tmpdir/${asset}" "$tmpdir/${asset}.sha256"

        archive="$tmpdir/${asset}"
    fi

    # ── Extract ────────────────────────────────────────────────────────────
    info "Extracting..."
    local extract_dir
    extract_dir=$(mktemp -d)
    tar -xzf "$archive" -C "$extract_dir"

    # ── Install ────────────────────────────────────────────────────────────
    if [ ! -d "$INSTALL_DIR" ]; then
        info "Creating $INSTALL_DIR..."
        mkdir -p "$INSTALL_DIR"
    fi

    info "Installing to $INSTALL_DIR/conduit..."
    mv "$extract_dir/conduit" "$INSTALL_DIR/conduit"
    chmod +x "$INSTALL_DIR/conduit"

    success "Conduit installed successfully!"
    echo ""

    # Check if install directory is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            warn "$INSTALL_DIR is not in your PATH"
            echo ""
            echo "Add it to your shell configuration:"
            echo ""
            case "$(basename "${SHELL:-sh}")" in
                zsh)
                    echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc && source ~/.zshrc"
                    ;;
                bash)
                    echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc && source ~/.bashrc"
                    ;;
                fish)
                    echo "  fish_add_path ~/.local/bin"
                    ;;
                *)
                    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
                    ;;
            esac
            echo ""
            ;;
    esac

    echo "Get started:"
    echo ""
    echo "  conduit"
    echo ""
    echo "Documentation: https://github.com/${REPO}"
    echo ""
}

main "$@"
