#!/usr/bin/env bash
# First-time development environment setup.
# Usage: bash scripts/dev-setup.sh

set -euo pipefail

echo "🚀 Setting up Torry development environment..."
echo ""

# Check prerequisites
echo "Checking prerequisites..."

if ! command -v node &> /dev/null; then
  echo "❌ Node.js is required. Install from https://nodejs.org"
  exit 1
fi

NODE_VERSION=$(node -v | sed 's/v//' | cut -d. -f1)
if [ "$NODE_VERSION" -lt 22 ]; then
  echo "❌ Node.js >= 22 is required. Current: $(node -v)"
  exit 1
fi
echo "  ✓ Node.js $(node -v)"

if ! command -v pnpm &> /dev/null; then
  echo "❌ pnpm is required. Install with: npm install -g pnpm"
  exit 1
fi
echo "  ✓ pnpm $(pnpm -v)"

if ! command -v rustc &> /dev/null; then
  echo "❌ Rust is required. Install from https://rustup.rs"
  exit 1
fi
echo "  ✓ Rust $(rustc --version | cut -d' ' -f2)"

if ! command -v cargo &> /dev/null; then
  echo "❌ Cargo is required. It should come with Rust."
  exit 1
fi
echo "  ✓ Cargo $(cargo --version | cut -d' ' -f2)"

echo ""
echo "Installing dependencies..."
pnpm install

echo ""
echo "Building Rust core + binding..."
bash scripts/build.sh

echo ""
echo "✅ Development environment ready!"
echo ""
echo "Try running:"
echo "  pnpm dev"
