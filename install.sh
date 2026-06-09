#!/bin/bash
#
# Pyroduct Installer
#
# Installs the pyroduct CLI tool and configures ~/.pyroduct as the global
# working directory for the daemon and cache.
#
# Usage:
#   From a release tarball:  ./install.sh
#   From the repo:           ./install.sh --build
#

set -euo pipefail

# =============================================================================
# Formatting helpers
# =============================================================================
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${CYAN}ℹ${NC}  $*"; }
ok()    { echo -e "${GREEN}✅${NC} $*"; }
warn()  { echo -e "${YELLOW}⚠${NC}  $*"; }
err()   { echo -e "${RED}❌${NC} $*" >&2; }
step()  { echo -e "\n${BOLD}==> $*${NC}"; }

# =============================================================================
# Install the CLI binary
# =============================================================================
install_binary() {
    step "Installing pyroduct CLI"

    local INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"

    # If a pre-built binary exists next to the script (release tarball), copy it.
    # Otherwise, build from source.
    local SCRIPT_DIR
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    if [ -f "$SCRIPT_DIR/pyroduct" ]; then
        cp "$SCRIPT_DIR/pyroduct" "$INSTALL_DIR/pyroduct"
        chmod +x "$INSTALL_DIR/pyroduct"
        ok "Installed pre-built binary to $INSTALL_DIR/pyroduct"
    elif [ "$BUILD_FROM_SOURCE" = true ]; then
        if ! command -v cargo &>/dev/null; then
            step "Installing Rust toolchain"
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            # shellcheck source=/dev/null
            source "$HOME/.cargo/env"
            ok "Rust toolchain installed"
        fi

        local CLI_PATH="$SCRIPT_DIR/lib/pyroduct"
        if [ ! -d "$CLI_PATH" ]; then
            err "Could not find '$CLI_PATH'. Run this script from the repository root."
            exit 1
        fi

        cargo install --path "$CLI_PATH" --features="cli"
        ok "pyroduct CLI built and installed"
        return
    else
        err "No pyroduct binary found. Use --build to compile from source."
        exit 1
    fi
}

# =============================================================================
# Set up ~/.pyroduct directory
# =============================================================================
setup_directory() {
    step "Setting up ~/.pyroduct"

    local PYRODUCT_DIR="$HOME/.pyroduct"
    mkdir -p "$PYRODUCT_DIR"

    if [ ! -f "$PYRODUCT_DIR/config.toml" ]; then
        cat > "$PYRODUCT_DIR/config.toml" <<EOF
author = "${USER:-pyroduct}"
build_slots = 4
EOF
        ok "Created default config at $PYRODUCT_DIR/config.toml"
    else
        info "Config already exists at $PYRODUCT_DIR/config.toml"
    fi
}

# =============================================================================
# Set up environment variables in shell rc
# =============================================================================
setup_environment() {
    step "Setting up environment variables"

    local PYRODUCT_DIR="$HOME/.pyroduct"

    local SHELL_NAME
    SHELL_NAME="$(basename "$SHELL")"

    local RC_FILE
    case "$SHELL_NAME" in
        zsh)  RC_FILE="$HOME/.zshrc" ;;
        bash) RC_FILE="$HOME/.bashrc" ;;
        fish) RC_FILE="$HOME/.config/fish/config.fish" ;;
        *)    RC_FILE="$HOME/.profile" ;;
    esac

    # Check if already configured
    if grep -q "PYRO_DAEMON_DIR" "$RC_FILE" 2>/dev/null; then
        info "Environment variables already configured in $RC_FILE"
        return
    fi

    # Ensure ~/.local/bin is on PATH
    local PATH_LINE=""
    if ! echo "$PATH" | tr ':' '\n' | grep -qx "$HOME/.local/bin"; then
        PATH_LINE='export PATH="$HOME/.local/bin:$PATH"'
    fi

    if [ "$SHELL_NAME" = "fish" ]; then
        cat >> "$RC_FILE" <<EOF

# Pyroduct environment variables
set -gx PYRODUCT "$PYRODUCT_DIR"
set -gx PYRO_DAEMON_DIR "$PYRODUCT_DIR"
fish_add_path "$HOME/.local/bin"
EOF
    else
        cat >> "$RC_FILE" <<EOF

# Pyroduct environment variables
export PYRODUCT="$PYRODUCT_DIR"
export PYRO_DAEMON_DIR="$PYRODUCT_DIR"
${PATH_LINE}
EOF
    fi

    ok "Environment variables added to $RC_FILE"
    warn "Run 'source $RC_FILE' or open a new terminal for changes to take effect"
}

# =============================================================================
# Main
# =============================================================================
main() {
    echo -e "${BOLD}"
    echo "  ╔═══════════════════════════════════════╗"
    echo "  ║        Pyroduct Installer             ║"
    echo "  ╚═══════════════════════════════════════╝"
    echo -e "${NC}"

    BUILD_FROM_SOURCE=false
    while [[ "$#" -gt 0 ]]; do
        case $1 in
            --build) BUILD_FROM_SOURCE=true ;;
            -h|--help)
                echo "Usage: $0 [--build] [-h|--help]"
                echo ""
                echo "Options:"
                echo "  --build    Build from source (requires Rust toolchain or installs it)"
                echo "  -h, --help Show this help message"
                exit 0
                ;;
            *) err "Unknown parameter: $1"; exit 1 ;;
        esac
        shift
    done

    install_binary
    setup_directory
    setup_environment

    echo ""
    echo -e "${BOLD}${GREEN}"
    echo "  ╔═══════════════════════════════════════╗"
    echo "  ║     Installation Complete! 🎉         ║"
    echo "  ╚═══════════════════════════════════════╝"
    echo -e "${NC}"

    echo "  Working directory: ~/.pyroduct"
    echo "  Config:            ~/.pyroduct/config.toml"
    echo ""
    echo "  Environment variables (added to shell rc):"
    echo "    PYRODUCT=~/.pyroduct"
    echo "    PYRO_DAEMON_DIR=~/.pyroduct"
    echo ""
    echo "  Run 'pyroduct --help' to get started."
    echo ""
}

main "$@"