#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "=== Building workspace ==="
cargo build --workspace

echo ""
echo "=== Running clippy ==="
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo ""
echo "=== Running tests ==="
cargo test --workspace

echo ""
echo "=== All checks passed ==="
