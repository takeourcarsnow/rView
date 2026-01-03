#!/bin/bash

# Pre-commit hook for rView
# This script runs various checks before allowing a commit

set -e

echo "Running pre-commit checks..."

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "Cargo not found. Please install Rust."
    exit 1
fi

# Run tests
echo "Running tests..."
cargo test

# Run clippy for linting
echo "Running clippy..."
cargo clippy -- -D warnings

# Check formatting
echo "Checking code formatting..."
cargo fmt --check

# Run security audit
echo "Running security audit..."
if command -v cargo-audit &> /dev/null; then
    cargo audit
else
    echo "cargo-audit not installed, skipping security audit"
fi

echo "All checks passed!"