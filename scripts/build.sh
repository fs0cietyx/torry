#!/usr/bin/env bash
# Full build script: Rust → binding → TypeScript packages
# Usage: bash scripts/build.sh

set -euo pipefail

echo "🔨 Building Torry..."
echo ""

# Step 1: Build the Rust crate and NAPI binding
echo "[1/4] Building Rust core + NAPI binding..."
cd packages/binding
pnpm run build
cd ../..
echo "  ✓ Rust build complete"
echo ""

# Step 2: Build shared types (no dependencies)
echo "[2/4] Building @torry/shared..."
pnpm --filter @torry/shared run build
echo "  ✓ @torry/shared built"

# Step 3: Build core (depends on shared + binding)
echo "[3/4] Building @torry/core..."
pnpm --filter @torry/core run build
echo "  ✓ @torry/core built"

# Step 4: Build CLI (depends on core + shared)
echo "[4/4] Building torry (CLI)..."
pnpm --filter torry run build
echo "  ✓ torry CLI built"
echo ""

echo "✅ Build complete!"
