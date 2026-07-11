#!/bin/bash
set -euo pipefail

echo "Packaging Pixie..."
cargo build --release --locked
mkdir -p build/bin
cp target/release/pixie-app build/bin/pixie
echo "Application compiled in build/bin/pixie"
