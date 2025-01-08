#!/usr/bin/env bash

set -euo pipefail

PREFIX="${PWD}/.local"
INSTALL_LOG="${PREFIX}/.install_log"

# Versions
ANCHOR_VERSION="v0.29.0"
SOLANA_VERSION="v1.18.18"

# Utility functions
log() { echo "$1" >> "$INSTALL_LOG"; }
is_installed() { grep -q "^$1$" "$INSTALL_LOG" 2>/dev/null; }

# install_rust() {
    # if ! is_installed "rust"; then
    #     # echo "Installing Rust..."
    #     # export RUSTUP_HOME="${PREFIX}/rustup"
    #     # export CARGO_HOME="${PREFIX}/cargo"
    #     # curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    #     # export PATH="${PREFIX}/cargo/bin:${PATH}"
    #     # rustup component add clippy rustfmt
    #     # log "rust"
    # fi
# }

install_anchor() {
    if ! is_installed "anchor"; then
        echo "Installing Anchor..."
        export PATH="${PREFIX}/cargo/bin:${PATH}"
        cargo install --git https://github.com/coral-xyz/anchor --tag ${ANCHOR_VERSION} avm --locked
        avm install 0.29.0
        avm use 0.29.0
        anchor --version
        log "anchor"
    fi
}

install_solana() {
    if ! is_installed "solana"; then
        echo "Installing Solana ${SOLANA_VERSION}..."
        sh -c "$(curl -sSfL https://release.solana.com/${SOLANA_VERSION}/install)"
        log "solana"
    fi
}

main() {
    mkdir -p "${PREFIX}"
    
    # Install dependencies
    # install_rust
    install_anchor
    install_solana
    
    # Add installations to PATH
    # echo "export PATH=\"${PREFIX}/cargo/bin:${PREFIX}/bin:\$PATH\"" > "${PREFIX}/env"
    # echo "export PATH=\"\$HOME/.local/share/solana/install/active_release/bin:\$PATH\"" >> "${PREFIX}/env"
    
    echo "✨ Rust, Anchor, and Solana have been installed successfully"
    echo "To use the installed tools, run:"
    # echo "source ${PREFIX}/env"
}

main