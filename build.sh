#!/bin/bash

set -e

# Ensure we're in the right directory
cd "$(dirname "$0")"

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "cargo is not installed. Please install Rust and try again."
    exit 1
fi

# Check if cross is installed
if ! command -v cross &> /dev/null; then
    echo "cross is not installed. Installing now..."
    cargo install cross
fi

# Create bin directory if it doesn't exist
mkdir -p bin

# Build for Windows
# echo "Building for Windows..."
# cross build --target x86_64-pc-windows-msvc --release
# cp target/x86_64-pc-windows-msvc/release/releaser.exe bin/releaser-win.exe

# Build for macOS Intel
echo "Building for macOS Intel..."
cargo build --target x86_64-apple-darwin --release
cp ./target/release/releaser bin/releaser-macos-x64

# Build for macOS ARM
echo "Building for macOS ARM..."
cargo build --target aarch64-apple-darwin --release
cp target/aarch64-apple-darwin/release/releaser bin/releaser-macos-arm64

# Build for Linux
echo "Building for Linux..."
cross build --target x86_64-unknown-linux-gnu --release
cp target/x86_64-unknown-linux-gnu/release/releaser bin/releaser-linux

echo "Build complete. Executables are in the 'bin' directory."
