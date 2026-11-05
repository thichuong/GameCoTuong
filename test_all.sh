#!/bin/bash

# Script to run all tests and builds for GameCoTuong

set -e

echo "🧪 Starting Test Suite..."

echo ""
echo "1. Checking code format (cargo fmt)..."
cargo fmt --check
echo "✅ cargo fmt passed!"

echo ""
echo "2. Running Clippy lints (cargo clippy)..."
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings
echo "✅ cargo clippy passed!"

echo ""
echo "3. Running Rust unit tests (cargo test)..."
cargo test --workspace
echo "✅ Cargo workspace tests passed!"

echo ""
echo "4. Building React frontend..."
npm run build
echo "✅ React frontend build passed!"

echo ""
echo "🎉 All tests and builds passed successfully!"
