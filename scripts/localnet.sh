#!/usr/bin/env bash

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
LOCAL_BIN="${PWD}/.local/bin"
LOCAL_CARGO="${PWD}/.local/cargo/bin"
VALIDATOR_LOG="test-validator.log"

# Add local paths to PATH if not already there
export PATH="${LOCAL_BIN}:${LOCAL_CARGO}:${PATH}"

log_info() {
    echo -e "${GREEN}INFO:${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}WARN:${NC} $1"
}

log_error() {
    echo -e "${RED}ERROR:${NC} $1"
}

cleanup() {
    log_info "Cleaning up..."
    # Kill the test validator if it's running
    pkill -f "solana-test-validator" || true
}

# Set up trap to cleanup on script exit
trap cleanup EXIT

start_test_validator() {
    log_info "Starting local test validator..."
    # Kill any existing validator
    pkill -f "solana-test-validator" || true
    
    # Start new validator in background
    solana-test-validator --reset --quiet > "$VALIDATOR_LOG" 2>&1 &
    
    # Wait for validator to start
    log_info "Waiting for validator to start..."
    sleep 5
    
    # Configure solana to use local validator
    solana config set --url http://127.0.0.1:8899
    
    # Wait a bit more to ensure validator is ready
    sleep 5
}

build_program() {
    log_info "Building program..."
    solana-install init 1.18.22
    anchor build
    log_info "Build completed successfully"
}

deploy_program() {
    log_info "Deploying program to local validator..."
    anchor deploy --provider.cluster localnet
    log_info "Deployment completed successfully"
}

main() {
    log_info "Starting build and deploy process..."
    
    # Start local test validator
    start_test_validator
    
    # Build the program
    build_program
    
    # Deploy locally
    deploy_program
    
    log_info "Build and deploy process completed successfully!"
    log_info "Your program is now deployed to the local test validator"
    log_info "Test validator logs are available in: $VALIDATOR_LOG"
}

main