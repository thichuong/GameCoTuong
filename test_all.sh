#!/bin/bash

# Script to run all tests for GameCoTuong

echo "🧪 Starting Test Suite..."

echo ""
echo "running cargo test --workspace..."
if cargo test --workspace; then
    echo "✅ Workspace tests passed!"
else
    echo "❌ Workspace tests failed!"
    exit 1
fi

echo ""
echo "running cargo test -p cotuong_core logic::generator..."
if cargo test -p cotuong_core logic::generator; then
     echo "✅ Move Generator tests passed!"
else
     echo "❌ Move Generator tests failed!"
     exit 1
fi

echo ""
echo "🎉 All tests passed successfully!"
