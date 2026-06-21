# X-Control

Remote desktop screen sharing and control utility. Lightweight Windows agent (594 KB) + web viewer (iPhone/iPad/desktop) relayed through a VPS.

## Architecture

```
┌──────────────┐     WebSocket      ┌──────────────┐     WebSocket     ┌───────────┐
│  Windows     │ ◄─────────────────► │   Relay      │ ◄────────────────► │  Browser  │
│  Agent       │     screen JPEGs    │   (VPS)      │     screen JPEGs   │  Viewer   │
│  (xcontrol)  │ ◄─────────────────► │              │ ◄────────────────► │  (HTML5)  │
│              │     input events    │              │     input events   │           │
└──────────────┘                     └──────────────┘                   └───────────┘
       │                                    │
       │ PIN: A B C D E                      │
       └─────── register ────────────────────┘
                                            │
                               ┌────────────┴────────────┐
                               │        Browser           │
                               │  enters PIN, joins room  │
                               └─────────────────────────┘
```

- **Agent** (Rust): Runs on the Windows PC to be controlled. Shows a PIN, connects via WebSocket to the relay, sends JPEG screen captures, receives mouse/keyboard input.
- **Relay** (Rust/Node.js): Runs on a VPS. Routes WebSocket traffic between agent and viewer using PIN-based rooms. Currently deployed as Node.js on Vultr.
- **Viewer** (HTML/JS): Opens in any modern browser. Enter the agent's PIN, see the remote screen, touch/drag to control mouse.

## Quick Start

### 1. Run the agent on the Windows PC

```
xcontrol.exe
```

A small window appears with a 5-letter PIN (letters A–H only).

### 2. Open the viewer

Browse to `http://[2001:19f0:8000:385b:5400:06ff:fe43:eb83]` on any device (iPhone, iPad, desktop).

Enter the PIN shown on the agent window and tap Connect.

### 3. Control

- **One finger**: Tap to click, drag to move mouse
- **Two fingers**: Drag to scroll
- **Tap near top of screen**: Shows info bar (FPS counter, Disconnect button)

## Building from Source

### Prerequisites

- Rust 1.96+ with `x86_64-pc-windows-gnu` target
- MinGW-w64 (`gcc` linker, e.g. from MSYS2 ucrt64)

### Build

```
cargo build --release
```

Output:
- `target/release/xcontrol.exe` — Windows agent (594 KB)
- `target/release/xcontrol-relay.exe` — Rust relay server (optional, not used on Vultr)

### Cross-compile hints

The agent targets `x86_64-pc-windows-gnu` with MinGW linker. The `.cargo/config.toml` sets:

```toml
[target.x86_64-pc-windows-gnu]
linker = "gcc"
```

## Configuration

| Environment Variable | Default | Description |
|---|---|---|
| `XCONTROL_RELAY` | `ws://[2001:...]/ws` | Override the relay WebSocket URL |

### Deployment (Relay on Vultr)

The relay is a Node.js server behind Caddy reverse proxy:

```
:80 → Caddy → :8080 → Node.js relay
```

**Server**: `node /root/relay/server.js` on port 8080, proxied by Caddy on port 80 (HTTP, no TLS).

**Auto-start**: `/etc/local.d/xcontrol.start` via OpenRC `rc-update add local`.

**Firewall**: UFW open on ports 80 and 443.

### Web Viewer

Embedded in the relay binary via `include_str!`. The source HTML lives at:

```
xcontrol-relay/src/web/index.html
```

Update it, rebuild the relay, or (for Node.js deployment) SCP it to the server:

```
scp xcontrol-relay/src/web/index.html root@[host]:/root/relay/src/web/index.html
```

## Binary Size

| Component | Size |
|---|---|
| `xcontrol.exe` (agent) | 594 KB |
| `xcontrol-relay.exe` (Rust relay) | 858 KB |

Release profile optimizations:

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

## License

MIT
