#!/usr/bin/env bash
# Clean all build artifacts across the monorepo.
# Usage: bash scripts/clean.sh

set -euo pipefail

echo "🧹 Cleaning build artifacts..."

# Rust build artifacts
if [ -d "target" ]; then
  rm -rf target
  echo "  ✓ Removed target/ (Rust)"
fi

# TypeScript build artifacts
for pkg in packages/shared packages/core packages/cli; do
  if [ -d "$pkg/dist" ]; then
    rm -rf "$pkg/dist"
    echo "  ✓ Removed $pkg/dist"
  fi
done

# NAPI generated files
rm -f packages/binding/*.node
rm -f packages/binding/index.js
rm -f packages/binding/index.d.ts
echo "  ✓ Removed NAPI generated files"

echo ""
echo "✅ Clean complete!"
