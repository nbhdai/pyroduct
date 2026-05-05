#!/bin/bash

set -e

echo "=== Pyroduct Installer ==="

USE_DEFAULTS=false

while [[ "$#" -gt 0 ]]; do
    case $1 in
        (-d|--default) USE_DEFAULTS=true ;;
        (*) echo "Unknown parameter passed: $1"; exit 1 ;;
    esac
    shift
done

if [ "$USE_DEFAULTS" = true ]; then
    echo "Running in default mode. Skipping prompts..."
    USER_NAME="${USER:-default_user}"
    NUM_ENVS=4
else
    read -p "Enter your username [default: ${USER:-default_user}]: " INPUT_USER
    USER_NAME="${INPUT_USER:-${USER:-default_user}}"

    read -p "Enter the number of build environments [default: 4]: " INPUT_ENVS
    NUM_ENVS="${INPUT_ENVS:-4}"

    if ! [[ "$NUM_ENVS" =~ ^[0-9]+$ ]]; then
        echo "Error: The number of build environments must be a valid number."
        exit 1
    fi
fi

echo ""
echo "Welcome $USER_NAME! Preparing to install with $NUM_ENVS build environment(s)..."
echo ""

CONFIG_DIR="$HOME/.pyroduct"
CONFIG_FILE="$CONFIG_DIR/config.toml"
PYROPATH=$(pwd)

mkdir -p "$CONFIG_DIR"

cat <<EOF > "$CONFIG_FILE"
author = "$USER_NAME"
target = "../target"
pyroduct.path = "$PYROPATH/lib/pyroduct"
build_slots = $NUM_ENVS
EOF

echo "✅ Created configuration file at: $CONFIG_FILE"
echo ""

CLI_PATH="lib/pyroduct"

if [ -d "$CLI_PATH" ]; then
    echo "Compiling and installing the pyroduct CLI..."
    cd "$CLI_PATH"
    cargo install --path .
    
    echo ""
    echo "✅ Successfully installed Pyroduct!"
    echo "You can now run 'pyroduct' from your terminal."
else
    echo "❌ Error: Could not find the '$CLI_PATH' directory."
    echo "Please make sure you are running this script from the root of the repository."
    exit 1
fi