#!/bin/bash
set -e  # Exit on error

echo "==================================="
echo "Building Proto Module and Harness"
echo "==================================="

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

cargo build --release -p proto_reporter

# 1. Build the module
echo -e "\n${BLUE}Building capability/module...${NC}"
cd proto/module
cargo build --target wasm32-unknown-unknown --release
cd ../..

# 2. Run the harness
echo -e "\n${BLUE}Running capability/harness...${NC}"
cd crates/harness
# Pass the path to the compiled wasm file
cargo run --release -- ../../target/wasm32-unknown-unknown/release/proto_module.wasm ../../target/wasm32-unknown-unknown/release/proto_module.wasm

echo -e "\n${GREEN}Done!${NC}"