<div align="center">
  <img src="https://raw.githubusercontent.com/mainakbiswas/torry/main/assets/logo.png" alt="Torry Logo" width="120" />
  <h1>Torry 🧲</h1>
  <p><strong>A sleek, blazingly fast, and modern terminal-based BitTorrent client.</strong></p>
  <p>Torry is a terminal-native BitTorrent client that doesn't compromise. Experience qBittorrent-level performance with a beautiful, out-of-the-box TUI.</p>
</div>

---

## ✨ Features

- **Beautiful Terminal UI**: Built with React and Ink, featuring shimmering progress bars, focus management, and smooth responsive design.
- **Blazing Fast I/O**: Powered by Rust and `memmap2` for zero-copy file mapping directly to disk.
- **Out of the Box**: No confusing configurations. It downloads directly to your OS `Downloads` folder by default.
- **Decentralized**: Full support for DHT (Distributed Hash Table), PEX (Peer Exchange), and UDP/HTTP Trackers.
- **Smart Algorithms**: Employs a hybrid piece-picking strategy (Random-First for quick startup, Rarest-First for network health) and dynamic choke/unchoke algorithms.
- **Pause & Resume**: Seamlessly pause downloads or close the app entirely. Torry stores your session state in a local SQLite database and picks up right where you left off.
- **Cross-Platform**: Built for macOS, Linux, and Windows.

---

## 🚀 Quick Start (Usage)

Torry requires **Node.js >= 22** and **Rust** to build the core engine.

### Installation

```bash
# 1. Clone the repository
git clone https://github.com/your-username/torry.git
cd torry

# 2. Setup and Install Dependencies
pnpm install

# 3. Build the core Rust engine & TypeScript CLI
bash scripts/build.sh

# 4. Run Torry!
pnpm dev
```

### Navigating the UI

Once Torry is open, the TUI is designed to be fully keyboard-driven and intuitive:

- **`↑` / `↓`**: Navigate between active torrents.
- **`s`**: Focus the global search bar to fetch magnet links or search.
- **`Esc`**: Unfocus the search bar or cancel an input.
- **`Space`**: Pause or resume the currently selected torrent.
- **`x`**: Delete a torrent from the session list (useful for cleaning up completed files).
- **`q` or `Ctrl+C`**: Safely exit Torry. Your sessions are automatically saved!

---

## 🏗️ Architecture

Torry splits its responsibilities perfectly between a lightning-fast Rust backend and a beautiful TypeScript frontend:

```text
crates/core       → Pure Rust BitTorrent engine (Zero-copy I/O, DHT, PEX, SQLite)
packages/binding  → Ultra-thin NAPI-RS bridge connecting Rust to Node.js
packages/shared   → Shared TypeScript types and state models
packages/cli      → Terminal UI built with React & Ink
```

By decoupling the engine from the UI, Torry avoids UI thread-blocking while downloading at extreme speeds. The Rust engine spawns a dedicated Tokio runtime and communicates state updates to the Node.js frontend via high-performance channels.

### Performance Under the Hood
Torry leverages a custom `ManagerActor` model in Rust for every active torrent. It maintains real-time telemetry (EMA throughput, latency, and stability) on connected peers to enforce strict token-bucket speed limits and intelligent unchoking.

---

## 🛠️ Development

Want to contribute to Torry? Here are some useful commands:

```bash
# Build everything (Rust + TypeScript)
bash scripts/build.sh

# Type-check all packages
pnpm typecheck
```

## 📜 License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
