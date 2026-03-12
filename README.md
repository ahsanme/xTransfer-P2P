# xTransfer-P2P

A peer-to-peer encrypted file transfer desktop app built with Rust, Tauri v2, libp2p, and React.

## Features

- **Automatic peer discovery** via mDNS (no server needed on LAN)
- **End-to-end encryption** — X25519 ECDH key exchange + AES-256-GCM per chunk
- **Drag-and-drop** file sending
- **In-app incoming file prompts** — Accept (saves to Downloads), Save As, or Decline
- **Transfer progress** with speed display (KB/s / MB/s)
- **Cross-platform** — macOS, Windows, Linux
- **Manual peer connection** via connection code (for internet/relay usage)

## Stack

| Layer | Technology |
|-------|-----------|
| GUI shell | Tauri v2 |
| Networking | rust-libp2p 0.55 (mDNS, QUIC, TCP, relay, dcutr, autonat, Kademlia) |
| Encryption | X25519 + HKDF-SHA256 + AES-256-GCM |
| Frontend | React 19 + TypeScript + Vite |

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 18+
- Tauri v2 system dependencies ([guide](https://tauri.app/start/prerequisites/))

### Run in development

```bash
npm install
npm run tauri dev
```

### Build for distribution

```bash
npm run tauri build
```

Installers are output to `target/release/bundle/`.

## Project Structure

```
src/                        # React frontend
  components/
    DropZone/               # Drag-and-drop + file browser
    IncomingFilePrompt/     # Accept/Decline UI for incoming files
    PeerList/               # Sidebar peer list
    TransferList/           # Active/completed transfers panel
    ConnectModal/           # Manual peer connection via code
  hooks/
    usePeers.ts             # Peer discovery & state
    useTransfers.ts         # Transfer events & state
  lib/
    tauri.ts                # Typed IPC wrappers
    types.ts                # Shared TypeScript types

src-tauri/src/              # Rust backend
  p2p/
    behaviour.rs            # libp2p NetworkBehaviour (all protocols)
    swarm.rs                # Main tokio event loop
    codec.rs                # Request/response protocol codec
    encryption.rs           # X25519 + AES-256-GCM per-chunk encryption
    transfer.rs             # Chunked file read/write logic
  commands/
    network.rs              # Tauri IPC: get_peer_id, connect_peer, get_peers
    transfer.rs             # Tauri IPC: send_file, accept_transfer, reject_transfer
  state.rs                  # AppState, SwarmCommand enum, TransferInfo
```

## How It Works

1. On startup each instance generates (or loads from keychain) an Ed25519 identity keypair → unique PeerID
2. mDNS broadcasts the peer on the local network — other instances discover it within ~1 second
3. Sender selects a peer, drops a file → chunked over an encrypted libp2p stream
4. Receiver sees the incoming file in-app, clicks **Accept** (saves to `~/Downloads`) or **Save As**

## Testing Two Instances Locally

```bash
# Terminal 1 — starts Vite + Tauri (instance A)
npm run tauri dev

# Terminal 2 — launches a second instance with a different identity
XTRANSFER_INSTANCE=B ./target/debug/xtransfer-p2p
```

## CI / Releases

GitHub Actions builds macOS (universal), Windows, and Linux installers on every `v*` tag push.

```bash
git tag v1.0.0
git push origin v1.0.0
```

Draft release with installers appears at [Releases](https://github.com/ahsanme/xTransfer-P2P/releases).

> **macOS note:** App is not yet notarized. Right-click → Open the first time to bypass Gatekeeper.

## Roadmap

- [ ] Real app icons
- [ ] Apple notarization / Windows code signing
- [ ] Auto-updater (Tauri built-in)
- [ ] Transfer history persistence
- [ ] Internet relay improvements (dnsaddr bootstrap)
