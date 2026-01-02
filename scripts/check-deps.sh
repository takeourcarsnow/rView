#!/bin/bash

# Dependency vulnerability check script for rView

set -e

echo "Checking for dependency vulnerabilities..."

# Install cargo-audit if not present
if ! command -v cargo-audit &> /dev/null; then
    echo "Installing cargo-audit..."
    cargo install cargo-audit
fi

# Run audit
cargo audit

# Check for outdated dependencies
echo "Checking for outdated dependencies..."
cargo outdated || echo "cargo-outdated not installed, run 'cargo install cargo-outdated' to check for outdated deps"

echo "Dependency check complete."