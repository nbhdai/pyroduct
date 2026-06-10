#!/bin/bash
#
# Pyroduct Installer
#
# Installs the pyroduct CLI and pyro-daemond binaries, and configures
# ~/.pyroduct as the global working directory for the daemon and cache.
#
# Usage:
#   From a release tarball:  ./install.sh [--service]
#   From the repo:           ./install.sh --build [--service]
#
#   --service  additionally sets up the daemon as a launchd user agent (macOS)
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
# Install the CLI + daemon binaries
# =============================================================================
install_binary() {
    step "Installing pyroduct CLI + pyro-daemond"

    local INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"

    # If pre-built binaries exist next to the script (release tarball), copy
    # them. Otherwise, build from source.
    local SCRIPT_DIR
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    if [ -f "$SCRIPT_DIR/pyroduct" ]; then
        cp "$SCRIPT_DIR/pyroduct" "$INSTALL_DIR/pyroduct"
        chmod +x "$INSTALL_DIR/pyroduct"
        ok "Installed pre-built binary to $INSTALL_DIR/pyroduct"

        if [ -f "$SCRIPT_DIR/pyro-daemond" ]; then
            cp "$SCRIPT_DIR/pyro-daemond" "$INSTALL_DIR/pyro-daemond"
            chmod +x "$INSTALL_DIR/pyro-daemond"
            ok "Installed pre-built binary to $INSTALL_DIR/pyro-daemond"
        else
            warn "No pyro-daemond binary found next to the script — daemon not installed"
        fi
    elif [ "$BUILD_FROM_SOURCE" = true ]; then
        if ! command -v cargo &>/dev/null; then
            step "Installing Rust toolchain"
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            # shellcheck source=/dev/null
            source "$HOME/.cargo/env"
            ok "Rust toolchain installed"
        fi

        local CLI_PATH="$SCRIPT_DIR/lib/pyroduct"
        local DAEMON_PATH="$SCRIPT_DIR/lib/pyro-daemon"
        if [ ! -d "$CLI_PATH" ] || [ ! -d "$DAEMON_PATH" ]; then
            err "Could not find '$CLI_PATH' / '$DAEMON_PATH'. Run this script from the repository root."
            exit 1
        fi

        # --root keeps the binaries in ~/.local/bin, matching the tarball install
        cargo install --path "$CLI_PATH" --features="cli" --root "$HOME/.local"
        ok "pyroduct CLI built and installed to $INSTALL_DIR/pyroduct"

        cargo install --path "$DAEMON_PATH" --root "$HOME/.local"
        ok "pyro-daemond built and installed to $INSTALL_DIR/pyro-daemond"
        return
    else
        err "No pyroduct binary found. Use --build to compile from source."
        exit 1
    fi
}

# =============================================================================
# Ensure the wasm32 target needed by `pyroduct ship` / `pyroduct package`
# =============================================================================
ensure_wasm_target() {
    if command -v rustup &>/dev/null; then
        if ! rustup target list --installed 2>/dev/null | grep -qx "wasm32-unknown-unknown"; then
            step "Adding wasm32-unknown-unknown target (used to compile modules)"
            rustup target add wasm32-unknown-unknown
            ok "wasm32-unknown-unknown target installed"
        fi
    else
        info "rustup not found — install the wasm32-unknown-unknown target manually before running 'pyroduct ship'"
    fi
}

# =============================================================================
# macOS: install the daemon as a launchd user agent
# =============================================================================
install_launchd_service() {
    step "Installing launchd user agent"

    local PYRODUCT_DIR="$HOME/.pyroduct"

    # Locate the installed daemon binary
    local DAEMOND_PATH="$HOME/.local/bin/pyro-daemond"
    if [ ! -x "$DAEMOND_PATH" ]; then
        DAEMOND_PATH="$(command -v pyro-daemond 2>/dev/null || echo "")"
    fi
    if [ -z "$DAEMOND_PATH" ]; then
        err "Could not find the pyro-daemond binary. Skipping service installation."
        return 1
    fi
    info "Using daemon binary: $DAEMOND_PATH"

    local PLIST_DIR="$HOME/Library/LaunchAgents"
    local PLIST_PATH="$PLIST_DIR/com.pyroduct.daemon.plist"
    local LOG_DIR="$HOME/Library/Logs/pyro-daemon"
    mkdir -p "$PLIST_DIR" "$LOG_DIR" "$PYRODUCT_DIR"

    cat > "$PLIST_PATH" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.pyroduct.daemon</string>

    <key>ProgramArguments</key>
    <array>
        <string>$DAEMOND_PATH</string>
        <string>--working-dir</string>
        <string>$PYRODUCT_DIR</string>
    </array>

    <key>EnvironmentVariables</key>
    <dict>
        <key>PYRODUCT</key>
        <string>$PYRODUCT_DIR</string>
        <key>PYRO_DAEMON_DIR</key>
        <string>$PYRODUCT_DIR</string>
    </dict>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>

    <key>StandardOutPath</key>
    <string>$LOG_DIR/stdout.log</string>

    <key>StandardErrorPath</key>
    <string>$LOG_DIR/stderr.log</string>
</dict>
</plist>
EOF
    ok "LaunchAgent plist created at $PLIST_PATH"

    # (Re)load the service
    launchctl bootout "gui/$(id -u)" "$PLIST_PATH" 2>/dev/null || true
    launchctl bootstrap "gui/$(id -u)" "$PLIST_PATH"
    ok "Service loaded and started"

    info "Check status with: launchctl print gui/$(id -u)/com.pyroduct.daemon"
    info "View logs at:      $LOG_DIR/"
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
    INSTALL_SERVICE=false
    while [[ "$#" -gt 0 ]]; do
        case $1 in
            --build)   BUILD_FROM_SOURCE=true ;;
            --service) INSTALL_SERVICE=true ;;
            -h|--help)
                echo "Usage: $0 [--build] [--service] [-h|--help]"
                echo ""
                echo "Options:"
                echo "  --build    Build from source (requires Rust toolchain or installs it)"
                echo "  --service  Set up pyro-daemond as a background service (macOS launchd)"
                echo "  -h, --help Show this help message"
                exit 0
                ;;
            *) err "Unknown parameter: $1"; exit 1 ;;
        esac
        shift
    done

    install_binary
    ensure_wasm_target
    setup_directory
    setup_environment

    if [ "$INSTALL_SERVICE" = true ]; then
        if [ "$(uname -s)" = "Darwin" ]; then
            install_launchd_service
        else
            warn "--service is currently only implemented for macOS (launchd)."
            warn "On Linux, use the NixOS module (services.pyro-daemon) or run 'pyro-daemond --working-dir ~/.pyroduct' manually."
        fi
    fi

    echo ""
    echo -e "${BOLD}${GREEN}"
    echo "  ╔═══════════════════════════════════════╗"
    echo "  ║     Installation Complete! 🎉         ║"
    echo "  ╚═══════════════════════════════════════╝"
    echo -e "${NC}"

    echo "  Installed binaries (in ~/.local/bin):"
    echo "    pyroduct        CLI for building and managing playbooks"
    echo "    pyro-daemond    Background daemon (required by the GUI)"
    echo ""
    echo "  Working directory: ~/.pyroduct"
    echo "  Config:            ~/.pyroduct/config.toml"
    echo ""
    echo "  Environment variables (added to shell rc):"
    echo "    PYRODUCT=~/.pyroduct"
    echo "    PYRO_DAEMON_DIR=~/.pyroduct"
    echo ""
    if [ "$INSTALL_SERVICE" = true ] && [ "$(uname -s)" = "Darwin" ]; then
        echo "  Daemon service (launchd):"
        echo "    Status:  launchctl print gui/$(id -u)/com.pyroduct.daemon"
        echo "    Logs:    ~/Library/Logs/pyro-daemon/"
    else
        echo "  Start the daemon manually:"
        echo "    pyro-daemond --working-dir ~/.pyroduct"
        echo "  (on macOS, re-run with --service to install it as a launchd service)"
    fi
    echo ""
    echo "  Run 'pyroduct --help' to get started."
    echo ""
}

main "$@"