#!/bin/bash
#
# Pyroduct Installer
#
# Installs the pyroduct CLI and pyro-daemond binary, and optionally sets up
# a systemd service running under a dedicated 'pyroduct' system user.
#
# Supports: Ubuntu/Debian, Arch Linux/Manjaro
# Usage:    ./install.sh [-d|--default]
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
# Distro detection
# =============================================================================
detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        case "$ID" in
            ubuntu|debian|pop|linuxmint|elementary)
                DISTRO="debian"
                ;;
            arch|manjaro|endeavouros|garuda)
                DISTRO="arch"
                ;;
            *)
                if [ -n "${ID_LIKE:-}" ]; then
                    case "$ID_LIKE" in
                        *debian*|*ubuntu*) DISTRO="debian" ;;
                        *arch*)            DISTRO="arch" ;;
                        *)                 DISTRO="unknown" ;;
                    esac
                else
                    DISTRO="unknown"
                fi
                ;;
        esac
    else
        DISTRO="unknown"
    fi
}

# =============================================================================
# Install system dependencies
# =============================================================================
install_deps() {
    step "Installing system dependencies ($DISTRO)"

    case "$DISTRO" in
        debian)
            sudo apt-get update -qq
            sudo apt-get install -y -qq \
                build-essential \
                pkg-config \
                libssl-dev \
                libsqlite3-dev \
                curl
            ;;
        arch)
            sudo pacman -Sy --needed --noconfirm \
                base-devel \
                openssl \
                sqlite \
                pkg-config \
                curl
            ;;
        *)
            warn "Unknown distribution. Please install manually:"
            warn "  - C compiler + build tools"
            warn "  - pkg-config"
            warn "  - OpenSSL development headers"
            warn "  - SQLite development headers"
            warn ""
            read -rp "Continue anyway? [y/N]: " CONTINUE
            if [[ ! "$CONTINUE" =~ ^[Yy] ]]; then
                err "Aborting."
                exit 1
            fi
            ;;
    esac

    ok "System dependencies installed"
}

# =============================================================================
# Ensure Rust toolchain is available
# =============================================================================
ensure_rust() {
    if command -v cargo &>/dev/null; then
        info "Rust toolchain found: $(rustc --version)"
        return
    fi

    step "Installing Rust toolchain"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
    ok "Rust toolchain installed: $(rustc --version)"
}

# =============================================================================
# Compile and install binaries
# =============================================================================
install_binaries() {
    step "Compiling and installing pyroduct CLI"

    local cli_path="lib/pyroduct"
    if [ ! -d "$cli_path" ]; then
        err "Could not find '$cli_path'. Please run this script from the repository root."
        exit 1
    fi

    cargo install --path "$cli_path" --features="cli"
    ok "pyroduct CLI installed"

    step "Compiling and installing pyro-daemond"

    local daemon_path="lib/pyro-daemon"
    if [ ! -d "$daemon_path" ]; then
        err "Could not find '$daemon_path'. Please run this script from the repository root."
        exit 1
    fi

    cargo install --path "$daemon_path"
    ok "pyro-daemond installed"
}

# =============================================================================
# Create pyroduct system user & group
# =============================================================================
create_system_user() {
    step "Creating pyroduct system user and group"

    if id pyroduct &>/dev/null; then
        info "User 'pyroduct' already exists"
    else
        sudo useradd \
            --system \
            --user-group \
            --shell /usr/sbin/nologin \
            --home-dir /var/lib/pyro-daemon \
            --no-create-home \
            pyroduct
        ok "Created system user 'pyroduct'"
    fi

    # Add the invoking user to the pyroduct group
    local REAL_USER="${SUDO_USER:-$USER}"
    if groups "$REAL_USER" 2>/dev/null | grep -qw pyroduct; then
        info "$REAL_USER is already in the 'pyroduct' group"
    else
        sudo usermod -aG pyroduct "$REAL_USER"
        ok "Added '$REAL_USER' to the 'pyroduct' group"
        warn "You may need to log out and back in for group membership to take effect"
    fi
}

# =============================================================================
# Set up daemon working directory and shared cache
# =============================================================================
setup_directories() {
    step "Setting up daemon working directory and shared cache"

    local DAEMON_DIR="/var/lib/pyro-daemon"
    local CACHE_DIR="$DAEMON_DIR/cache"

    # Create the daemon working directory
    sudo mkdir -p "$DAEMON_DIR"
    sudo chown pyroduct:pyroduct "$DAEMON_DIR"
    sudo chmod 0750 "$DAEMON_DIR"

    # Create the shared cache directory with setgid
    sudo mkdir -p "$CACHE_DIR"
    sudo chown pyroduct:pyroduct "$CACHE_DIR"
    sudo chmod 2775 "$CACHE_DIR"

    # Create cache subdirectories
    for subdir in capabilities interfaces modules; do
        sudo mkdir -p "$CACHE_DIR/$subdir"
        sudo chown pyroduct:pyroduct "$CACHE_DIR/$subdir"
        sudo chmod 2775 "$CACHE_DIR/$subdir"
    done

    # Create data directory
    sudo mkdir -p "$DAEMON_DIR/data"
    sudo chown pyroduct:pyroduct "$DAEMON_DIR/data"
    sudo chmod 0750 "$DAEMON_DIR/data"

    # Write cache config.toml
    sudo tee "$CACHE_DIR/config.toml" > /dev/null <<EOF
author = "$AUTHOR_NAME"
build_slots = $NUM_ENVS
EOF
    sudo chown pyroduct:pyroduct "$CACHE_DIR/config.toml"
    sudo chmod 0664 "$CACHE_DIR/config.toml"

    ok "Daemon working directory created at $DAEMON_DIR"
    ok "Shared cache created at $CACHE_DIR"
}

# =============================================================================
# Set up environment variables
# =============================================================================
setup_environment() {
    step "Setting up environment variables"

    # Create the environment file for systemd
    sudo tee /etc/pyroduct.env > /dev/null <<'EOF'
PYRODUCT=/var/lib/pyro-daemon/cache
PYRO_DAEMON_DIR=/var/lib/pyro-daemon
EOF
    ok "Created /etc/pyroduct.env"

    # Create a profile.d script for interactive shells
    sudo tee /etc/profile.d/pyroduct.sh > /dev/null <<'PROFILE'
# Pyroduct environment variables
export PYRODUCT="/var/lib/pyro-daemon/cache"
export PYRO_DAEMON_DIR="/var/lib/pyro-daemon"
PROFILE
    sudo chmod 0644 /etc/profile.d/pyroduct.sh
    ok "Created /etc/profile.d/pyroduct.sh"
}

# =============================================================================
# Install systemd service
# =============================================================================
install_systemd_service() {
    step "Installing systemd service"

    # Locate the installed binary
    local DAEMOND_PATH
    DAEMOND_PATH="$(command -v pyro-daemond 2>/dev/null || echo "")"

    if [ -z "$DAEMOND_PATH" ]; then
        # Check common cargo install paths
        for candidate in \
            "$HOME/.cargo/bin/pyro-daemond" \
            "/usr/local/bin/pyro-daemond" \
            "/usr/bin/pyro-daemond"; do
            if [ -x "$candidate" ]; then
                DAEMOND_PATH="$candidate"
                break
            fi
        done
    fi

    if [ -z "$DAEMOND_PATH" ]; then
        err "Could not find pyro-daemond binary. Skipping service installation."
        return 1
    fi

    info "Using daemon binary: $DAEMOND_PATH"

    sudo tee /etc/systemd/system/pyro-daemon.service > /dev/null <<EOF
[Unit]
Description=Pyroduct Daemon - Background Playbook and Process Supervisor
Documentation=https://github.com/nbhdai/pyroduct
After=network.target

[Service]
Type=simple
User=pyroduct
Group=pyroduct

ExecStart=$DAEMOND_PATH --working-dir /var/lib/pyro-daemon
EnvironmentFile=/etc/pyroduct.env

# Working directory & state
StateDirectory=pyro-daemon
WorkingDirectory=/var/lib/pyro-daemon

# File permissions: group-accessible socket and files
UMask=0007

# Restart policy
Restart=on-failure
RestartSec=5

# Sandboxing: daemon only has access to /var/lib/pyro-daemon
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
NoNewPrivileges=true
ReadWritePaths=/var/lib/pyro-daemon

[Install]
WantedBy=multi-user.target
EOF

    ok "Service unit installed at /etc/systemd/system/pyro-daemon.service"

    sudo systemctl daemon-reload
    sudo systemctl enable pyro-daemon.service
    ok "Service enabled"

    sudo systemctl start pyro-daemon.service
    ok "Service started"

    info "Check status with: systemctl status pyro-daemon"
    info "View logs with:    journalctl -u pyro-daemon -f"
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

    # Parse flags
    USE_DEFAULTS=false
    while [[ "$#" -gt 0 ]]; do
        case $1 in
            -d|--default) USE_DEFAULTS=true ;;
            -h|--help)
                echo "Usage: $0 [-d|--default] [-h|--help]"
                echo ""
                echo "Options:"
                echo "  -d, --default   Skip all prompts and use default values"
                echo "  -h, --help      Show this help message"
                exit 0
                ;;
            *) err "Unknown parameter: $1"; exit 1 ;;
        esac
        shift
    done

    # Detect distro
    detect_distro
    info "Detected distribution: $DISTRO"

    # Collect user input
    if [ "$USE_DEFAULTS" = true ]; then
        info "Running in default mode (skipping prompts)"
        AUTHOR_NAME="${USER:-pyroduct}"
        NUM_ENVS=4
        INSTALL_DAEMON=true
    else
        echo ""
        read -rp "$(echo -e "${CYAN}?${NC}") Author name [default: ${USER:-pyroduct}]: " INPUT_USER
        AUTHOR_NAME="${INPUT_USER:-${USER:-pyroduct}}"

        read -rp "$(echo -e "${CYAN}?${NC}") Number of build slots [default: 4]: " INPUT_ENVS
        NUM_ENVS="${INPUT_ENVS:-4}"
        if ! [[ "$NUM_ENVS" =~ ^[0-9]+$ ]]; then
            err "Build slots must be a number."
            exit 1
        fi

        read -rp "$(echo -e "${CYAN}?${NC}") Install systemd daemon service? [Y/n]: " INPUT_DAEMON
        case "$INPUT_DAEMON" in
            [Nn]*) INSTALL_DAEMON=false ;;
            *)     INSTALL_DAEMON=true ;;
        esac
    fi

    echo ""
    info "Author:         $AUTHOR_NAME"
    info "Build slots:    $NUM_ENVS"
    info "Install daemon: $INSTALL_DAEMON"
    echo ""

    # Step 1: Install system dependencies
    install_deps

    # Step 2: Ensure Rust is available
    ensure_rust

    # Step 3: Compile and install binaries
    install_binaries

    # Step 4: Optionally set up the daemon
    if [ "$INSTALL_DAEMON" = true ]; then
        create_system_user
        setup_directories
        setup_environment
        install_systemd_service
    else
        # Even without daemon, create user-local cache
        step "Setting up user-local cache"

        CONFIG_DIR="$HOME/.pyroduct"
        mkdir -p "$CONFIG_DIR"
        cat <<EOF > "$CONFIG_DIR/config.toml"
author = "$AUTHOR_NAME"
build_slots = $NUM_ENVS
EOF
        ok "Created configuration at $CONFIG_DIR/config.toml"
    fi

    # Done!
    echo ""
    echo -e "${BOLD}${GREEN}"
    echo "  ╔═══════════════════════════════════════╗"
    echo "  ║     Installation Complete! 🎉         ║"
    echo "  ╚═══════════════════════════════════════╝"
    echo -e "${NC}"

    echo "  Available commands:"
    echo "    pyroduct        CLI for building and managing playbooks"
    if [ "$INSTALL_DAEMON" = true ]; then
        echo "    pyro-daemond    Background daemon (running as systemd service)"
        echo ""
        echo "  Daemon paths:"
        echo "    Working dir:    /var/lib/pyro-daemon"
        echo "    Shared cache:   /var/lib/pyro-daemon/cache"
        echo "    Control socket: /var/lib/pyro-daemon/control"
        echo ""
        echo "  Environment variables (set via /etc/profile.d/pyroduct.sh):"
        echo "    PYRODUCT=/var/lib/pyro-daemon/cache"
        echo "    PYRO_DAEMON_DIR=/var/lib/pyro-daemon"
        echo ""
        warn "Log out and back in for group membership and environment changes to take effect."
    fi
    echo ""
}

main "$@"