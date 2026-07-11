#!/bin/bash
set -euo pipefail

echo "=========================================="
echo "      Pixie Bootstrap Process             "
echo "=========================================="

# 1. Check system dependencies
command -v pkg-config >/dev/null 2>&1 || { echo "Error: pkg-config is not installed." >&2; exit 1; }
command -v pre-commit >/dev/null 2>&1 || { echo "Error: pre-commit is not installed." >&2; exit 1; }

# 2. Setup Cargo tools
export PATH="$HOME/.cargo/bin:$PATH"

if ! command -v cargo-audit &> /dev/null; then
    echo "Installing cargo-audit..."
    cargo install cargo-audit
else
    echo "cargo-audit is already installed."
fi

if ! command -v cargo-deny &> /dev/null; then
    echo "Installing cargo-deny..."
    cargo install --locked cargo-deny
else
    echo "cargo-deny is already installed."
fi

if ! command -v cargo-llvm-cov &> /dev/null; then
    echo "Installing cargo-llvm-cov..."
    cargo install cargo-llvm-cov
else
    echo "cargo-llvm-cov is already installed."
fi

# 3. Setup git hooks
echo "Installing pre-commit hooks..."
pre-commit install
pre-commit install --hook-type pre-push

echo "Bootstrap complete!"
