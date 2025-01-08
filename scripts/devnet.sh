#!/usr/bin/env bash

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
NETWORK="devnet"
FEATURES="devnet"
LOCAL_BIN="${PWD}/.local/bin"
LOCAL_CARGO="${PWD}/.local/cargo/bin"

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

check_dependencies() {
    # Check for anchor in local installation
    if [ ! -x "${LOCAL_BIN}/anchor" ]; then
        log_error "anchor is not installed in ${LOCAL_BIN}"
        log_info "Please run the installation script first"
        exit 1
    fi

    # Check for solana in local installation
    if [ ! -x "${LOCAL_BIN}/solana" ]; then
        log_error "solana-cli is not installed in ${LOCAL_BIN}"
        log_info "Please run the installation script first"
        exit 1
    fi
}

check_solana_config() {
    solana config set --url https://api.devnet.solana.com
}

build_program() {
    log_info "Building program with devnet features..."
    solana-install init 1.18.22
    anchor build -- --features "$FEATURES"
    log_info "Build completed successfully"
}

# check_wallet_balance() {
#     # solana airdrop 2
# }

deploy_program() {
    log_info "Deploying program to devnet..."
    anchor deploy --provider.cluster "$NETWORK"
    log_info "Deployment completed successfully"
}

main() {
    log_info "Starting build and deploy process..."
    
    # Source the environment if it exists
    # if [ -f "${PWD}/.local/env" ]; then
    #     source "${PWD}/.local/env"
    # else
    #     log_warn "Environment file not found at ${PWD}/.local/env"
    #     log_info "Make sure you've run the installation script"
    # fi
    
    # Check if required tools are installed
    # check_dependencies
    
    # Ensure we're on devnet
    check_solana_config
    
    # Check wallet balance before deploying
    # check_wallet_balance
    
    # Build the program
    build_program
    
    # Deploy to devnet
    deploy_program
    
    log_info "Build and deploy process completed successfully!"
    log_info "You can verify your program on Solana Explorer:"
    log_info "https://explorer.solana.com/?cluster=devnet"
}

main