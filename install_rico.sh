#!/bin/bash

# Exit on any error
set -e

# Constants
RICO_REPO="https://github.com/ranajahanzaib/rico.git"
INSTALL_DIR="/usr/local/bin"
RUST_VERSION="1.80" # Adjust to the required Rust version

# Function to check and install Rust if not already installed
install_rust() {
    if ! command -v rustup &>/dev/null; then
        echo "Rust is not installed. Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    else
        echo "Rust is already installed."
    fi

    echo "Ensuring required Rust version is installed..."
    rustup install $RUST_VERSION
    rustup default $RUST_VERSION
}

# Clone the rico repository
clone_rico_repo() {
    if [ -d "rico" ]; then
        echo "Removing existing rico directory..."
        rm -rf rico
    fi

    echo "Cloning rico repository..."
    git clone $RICO_REPO
}

# Build rico with cargo
build_rico() {
    echo "Building rico..."
    cd rico
    cargo build --release
    cd ..
}

# Move the rico binary to the system PATH
install_rico_binary() {
    echo "Installing rico binary to $INSTALL_DIR..."
    sudo mv rico/target/release/rico "$INSTALL_DIR"
    echo "Rico installed successfully and is accessible as 'rico'."
}

# Main script execution
main() {
    install_rust
    clone_rico_repo
    build_rico
    install_rico_binary

    # Cleanup
    echo "Cleaning up..."
    rm -rf rico
}

main
