#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -e

echo "=== Pyroduct Installer ==="

# Default flag variable
USE_DEFAULTS=false

# Parse command line arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        (-d|--default) USE_DEFAULTS=true ;;
        (*) echo "Unknown parameter passed: $1"; exit 1 ;;
    esac
    shift
done

if [ "$USE_DEFAULTS" = true ]; then
    echo "Running in default mode. Skipping prompts..."
    # Use the system's current username, fallback to "default_user" if not found
    USER_NAME="${USER:-default_user}"
    NUM_ENVS=4
else
    # 1. Prompt for username
    read -p "Enter your username [default: ${USER:-default_user}]: " INPUT_USER
    USER_NAME="${INPUT_USER:-${USER:-default_user}}"

    # 2. Prompt for number of build environments
    read -p "Enter the number of build environments [default: 4]: " INPUT_ENVS
    NUM_ENVS="${INPUT_ENVS:-4}"

    # Validate that the number of environments is actually a number
    if ! [[ "$NUM_ENVS" =~ ^[0-9]+$ ]]; then
        echo "Error: The number of build environments must be a valid number."
        exit 1
    fi
fi

echo ""
echo "Welcome $USER_NAME! Preparing to install with $NUM_ENVS build environment(s)..."
echo ""

# 3. Create the configuration file
CONFIG_DIR="$HOME/.pyroduct"
CONFIG_FILE="$CONFIG_DIR/config.toml"
PYROPATH=$(pwd)

mkdir -p "$CONFIG_DIR"

# Write the configuration using a heredoc
cat <<EOF > "$CONFIG_FILE"
author = "$USER_NAME"
target = "../target"
pyroduct.path = "$PYROPATH/lib/pyroduct"
build_slots = $NUM_ENVS
EOF

echo "✅ Created configuration file at: $CONFIG_FILE"
echo ""

# 4. Navigate to the CLI path and run cargo install
CLI_PATH="lib/cli"

if [ -d "$CLI_PATH" ]; then
    echo "Compiling and installing the pyroduct CLI..."
    cd "$CLI_PATH"
    
    # Run the cargo install command
    cargo install --path .
    
    echo ""
    echo "✅ Successfully installed Pyroduct!"
    echo "You can now run 'pyroduct' from your terminal."
else
    echo "❌ Error: Could not find the '$CLI_PATH' directory."
    echo "Please make sure you are running this script from the root of the repository."
    exit 1
fi