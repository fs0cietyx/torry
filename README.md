# Torry 🧲

A modern, terminal-native decentralized download manager.

- **Fast** — Rust-powered download engine with parallel chunked transfers
- **Secure** — Verification-first design with cryptographic hash checking
- **Extensible** — Provider/plugin architecture for any protocol
- **Cross-platform** — Works on macOS, Linux, and Windows

## Architecture

```
crates/core       → Pure Rust download engine (no Node.js dependency)
packages/binding  → Ultra-thin NAPI-RS bridge (Rust → Node.js)
packages/shared   → Shared TypeScript types and utilities
packages/core     → TypeScript API layer
packages/cli      → Terminal CLI (what users install)
providers/        → Protocol provider plugins
```

### Dependency Graph

```
torry (CLI)
  └── @torry/core (TS API)
        ├── @torry/binding (NAPI bridge)
        │     └── torry-core (Rust crate)
        └── @torry/shared (types/utils)
```

## Prerequisites

- Node.js >= 22.0.0
- Rust (latest stable, via [rustup](https://rustup.rs))
- pnpm >= 10.x

## Quick Start

```bash
# Clone and setup
git clone https://github.com/your-username/torry.git
cd torry
bash scripts/dev-setup.sh

# Or manual setup
pnpm install
bash scripts/build.sh

# Run the CLI
pnpm dev
```

## Development

```bash
# Build everything (Rust + TypeScript)
bash scripts/build.sh

# Build only Rust
pnpm build:rust

# Build only TypeScript
pnpm build

# Type-check all packages
pnpm typecheck

# Run the CLI in development
pnpm dev

# Clean all build artifacts
pnpm clean
```

## Project Structure

| Directory | Language | Purpose |
|---|---|---|
| `crates/core` | Rust | Download engine, hash verification, protocols |
| `packages/binding` | Rust + npm | NAPI-RS bridge between Rust and Node.js |
| `packages/shared` | TypeScript | Shared types, errors, constants |
| `packages/core` | TypeScript | Public API layer |
| `packages/cli` | TypeScript | CLI entry point |
| `providers/` | TypeScript | Protocol provider plugins |
| `scripts/` | Bash | Build and development scripts |

## License

MIT
