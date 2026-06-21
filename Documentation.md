# X-Control: Technical Specification

## Overview

X-Control is a lightweight remote desktop control utility. A small Windows agent (~600 KB) connects to a relay VPS via WebSocket and streams JPEG screen captures to a browser-based viewer. The viewer sends mouse/keyboard input events back through the relay to the agent. PIN-based authentication prevents unauthorized access.

---

## 1. Protocol Specification

### 1.1 WebSocket Endpoint

```
ws://<relay-host>/ws
```

Messages are either **JSON text** (control/input) or **binary** (JPEG frames).

### 1.2 Message Types

#### Agent → Relay

| Type | Direction | Payload |
|---|---|---|
| `register` | Agent → Relay | `{"type":"register","pin":"ABCDE"}` |
| Binary frame | Agent → Relay | Raw JPEG bytes (~10–50 KB per frame) |

#### Viewer → Relay

| Type | Direction | Payload |
|---|---|---|
| `join` | Viewer → Relay | `{"type":"join","pin":"ABCDE"}` |
| `input` | Viewer → Relay | `{"type":"input","event":"mousedown","x":100,"y":200}` |

#### Relay → Agent/Viewer

| Type | Direction | Payload |
|---|---|---|
| `registered` | Relay → Agent | `{"type":"registered","pin":"ABCDE"}` |
| `joined` | Relay → Viewer | `{"type":"joined","pin":"ABCDE"}` |
| `error` | Relay → Either | `{"type":"error","msg":"..."}` |
| Input events | Relay → Agent | Forwarded viewer input JSON |
| Binary frames | Relay → Viewer | Forwarded agent JPEG bytes |

### 1.3 Input Events

| Event | Fields | Description |
|---|---|---|
| `mousedown` | `x`, `y` | Touch start / left button press at absolute coordinates |
| `mouseup` | `x`, `y` | Touch end / left button release |
| `mousemove` | `x`, `y` | Drag / mouse movement |
| `click` | `x`, `y` | Tap (single click) |
| `scroll` | `y` | Two-finger scroll, delta in pixels |
| `keydown` | `key` | Key press (virtual key name) |
| `keyup` | `key` | Key release |

Coordinates are absolute in the remote screen's native resolution (e.g., 1920×1080). The viewer normalizes touch position to the remote screen dimensions.

### 1.4 Supported Virtual Keys

`ENTER`, `BACKSPACE`, `TAB`, `ESCAPE`, `SPACE`, `DELETE`, `SHIFT`, `CONTROL`/`CTRL`, `ALT`, `UP`, `DOWN`, `LEFT`, `RIGHT`, `HOME`, `END`, `PAGEUP`, `PAGEDOWN`, `CAPSLOCK`, `A`–`Z`, `0`–`9`.

---

## 2. Agent (xcontrol)

**Language**: Rust
**Target**: `x86_64-pc-windows-gnu`
**Binary**: `xcontrol.exe` (594 KB release)
**Entry Point**: `xcontrol/src/main.rs`

### 2.1 Architecture

```
main()
  ├── Register window class (WNDCLASSW)
  ├── Create window (320×175, topmost)
  │   ├── "Your PIN:" label
  │   ├── PIN display (Consolas, 32pt, bold)
  │   ├── Status text (Segoe UI, 14pt)
  │   └── Stop button
  ├── Add system tray icon
  ├── Generate 5-letter PIN (A–H)
  ├── Spawn worker thread (Tokio runtime)
  │   ├── Connect via WebSocket
  │   ├── Register PIN
  │   ├── Loop: capture → encode → send (10 FPS)
  │   └── Receive input events → forward to input module
  └── Windows message loop (GetMessageW/DispatchMessageW)
```

### 2.2 GUI

Pure Win32 API (no framework dependency). Window is compact (320×175 px), always-on-top, with:
- **PIN display**: centered, 32pt Consolas bold
- **Status bar**: connection state messages
- **Stop button**: kills the agent
- **System tray**: minimize to tray, right-click for Show/Quit, double-click to restore

Window procedure handles custom messages:
- `WM_PIN`: Update PIN text
- `WM_STATUS`: Update status text
- `WM_STOP`: Graceful shutdown
- `WM_TRAY`: Tray icon events

### 2.3 Screen Capture (`capture.rs`)

```
BitBlt (screen DC → memory DC)
  → GetDIBits (memory DC → RGB32 pixel buffer)
  → image crate (RGBA → RGB8)
  → JpegEncoder (quality 70)
  → Vec<u8>
```

Steps:
1. `GetDC(NULL)` for the full-screen device context
2. `CreateCompatibleDC` + `CreateCompatibleBitmap` for the memory DC
3. `BitBlt` with `SRCCOPY` to capture the screen
4. `GetDIBits` to read 32-bit BGRA pixel data
5. Convert to RGB8 via the `image` crate
6. JPEG encode at quality 70 using `image::codecs::jpeg::JpegEncoder`

The capture runs at ~10 FPS (100 ms interval between captures).

### 2.4 Input Injection (`input.rs`)

Uses `SendInput` from the Windows API:

- **Mouse move**: `MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE` with absolute coordinates
- **Mouse click**: `MOUSEEVENTF_LEFTDOWN` / `MOUSEEVENTF_LEFTUP`
- **Mouse scroll**: `MOUSEEVENTF_WHEEL`
- **Key press**: `INPUT_KEYBOARD` with virtual key codes (`KEYEVENTF_KEYUP` for release)

### 2.5 PIN Scheme

```
gen_pin():
  rand::thread_rng()
  → 5 characters from range 'A'..='H' (only A–H, 8 possible values)
  → 8^5 = 32,768 possible combinations
```

The restricted alphabet (A–H only, no digits or letters I–Z) simplifies the viewer input (no ambiguity between e.g. `1`/`I`/`l`/`O`/`0`). The viewer input field filters to `[A-H]` only.

### 2.6 Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `tokio` | 1.x | Async runtime (WebSocket) |
| `tokio-tungstenite` | 0.29 | WebSocket client |
| `futures-util` | 0.3 | Async stream combinators |
| `serde` / `serde_json` | 1.x | JSON serialization |
| `image` | 0.25 | JPEG encoding (only "jpeg" feature) |
| `windows-sys` | 0.61 | Win32 API bindings |
| `rand` | 0.8 | PIN generation |
| `lazy_static` | 1.x | Global state |

---

## 3. Relay

### 3.1 Rust Relay (`xcontrol-relay`)

**Language**: Rust
**Target**: Any platform (tested on Windows)
**Binary**: `xcontrol-relay.exe` (858 KB release)

A simple WebSocket relay using `axum`:

- `GET /` — Serves the embedded web viewer HTML
- `GET /ws` — WebSocket upgrade for agent/viewer connections

**Room management**: In-memory `HashMap<String, Room>` protected by `Arc<Mutex<>>`.

**Room structure**:
```rust
struct Room {
    agent_tx: Option<UnboundedSender<Message>>,   // Agent's send channel
    viewers: Vec<UnboundedSender<Message>>,        // All viewers' send channels
}
```

- Agent registers → creates room with `agent_tx`
- Viewer joins → added to `viewers` list
- Agent sends binary frames → relay fans out to all viewers
- Viewer sends input → relay forwards to `agent_tx`
- Agent disconnects → room is removed

### 3.2 Node.js Relay (Deployed on Vultr)

The server at `/root/relay/server.js` provides the same functionality implemented in Node.js. It was chosen over the Rust relay because the free-tier Vultr VPS ran out of memory when compiling the Rust binary.

**Tech stack**: Node.js (no frameworks), `ws` WebSocket library, static file serving for the web viewer.

**Process management**: Runs via `nohup` on boot, started from `/etc/local.d/xcontrol.start`.

---

## 4. Web Viewer

**File**: `xcontrol-relay/src/web/index.html`
**Single-file**: HTML + CSS + JS embedded, no external dependencies

### 4.1 PIN Screen

- Aesthetic gradient background
- Single input field (5 chars, A–H only, auto-uppercased)
- "Connect" button (enabled only when 5 chars entered)
- Status messages: error (red), connected (green), default (gray)

### 4.2 Remote Screen

- `<img>` element displaying the JPEG stream
- `object-fit: contain` for proper aspect ratio
- FPS counter overlay (auto-hides after 5 seconds)
- Tap near top of screen to reveal info bar

### 4.3 Touch Input Mapping

| Touch gesture | Mapped action |
|---|---|
| Single finger down → up (no move) | Click at position |
| Single finger down → drag | Mouse move (absolute) |
| Single finger down → up (after drag) | Mouse up |
| Two-finger drag | Scroll (delta accumulation, 5px threshold) |

All touch coordinates are normalized:
```
remote_x = (touch_x - rect.left) / rect.width  * remote_width
remote_y = (touch_y - rect.top)  / rect.height * remote_height
```

### 4.4 Mouse Input

Regular mouse clicks on desktop also send `click` events with absolute coordinates, enabling use on non-touch devices.

---

## 5. Security Model

### 5.1 Authentication

- PIN is generated randomly on the agent (A–H only, 5 chars = 32,768 combinations)
- PIN is registered with the relay before viewer can join
- Viewer must provide the exact PIN to join the room
- No session tokens, no persistent state

### 5.2 Limitations

- **No encryption**: All traffic is plain WebSocket (ws://, not wss://). The relay uses HTTP, not HTTPS. Screen contents and input events are transmitted in cleartext.
- **No re-authentication**: Once a viewer joins a room, they stay connected until disconnect. The PIN is only verified at join time.
- **Single-agent rooms**: Only one agent can register a given PIN at a time. The relay rejects duplicate registrations.

### 5.3 Future Improvements

- Add WSS/TLS via Caddy (requires a domain for Let's Encrypt)
- Add viewer authentication tokens
- Add session timeouts and PIN expiry
- Replace PIN scheme with cryptographic handshake

---

## 6. Binary Size Analysis

| Optimization | Approx. saving |
|---|---|
| `opt-level = "z"` (size optimization) | ~30% vs `opt-level = 3` |
| `lto = true` (link-time optimization) | ~15% |
| `codegen-units = 1` | ~5% |
| `strip = true` (remove symbols) | ~30% |
| `panic = "abort"` (remove unwind tables) | ~5% |
| JPEG-only `image` crate (no PNG/GIF/etc) | ~150 KB vs full `image` |

The `image` crate accounts for ~200 KB of the binary; the Win32 glue via `windows-sys` is minimal (~50 KB for the GDI/UI/input subset).

---

## 7. Performance

### 7.1 Frame Rate

- Agent captures and sends at **10 FPS** (100 ms interval)
- JPEG quality **70** — balances size vs quality
- Typical JPEG frame: ~10–50 KB (depends on screen content and resolution)

### 7.2 Bandwidth

- At 10 FPS, ~100–500 KB/s upstream from the agent
- Viewer receives identical bandwidth downstream
- Relay performs zero-copy forwarding (no transcoding)

### 7.3 Latency

- Capture → JPEG encode: ~5–30 ms (CPU-bound on Windows)
- Network round-trip: varies by VPS location
- Viewer decode: browser-native JPEG decode (< 1 ms)

### 7.4 Memory

- Agent: ~5–15 MB (includes Tokio runtime, JPEG buffer, Win32 resources)
- Node.js relay: ~30 MB
- Rust relay: ~5–10 MB
- Browser viewer: negligible

---

## 8. Development

### 8.1 Workspace Structure

```
x-control/
├── Cargo.toml                  # Workspace root
├── .cargo/config.toml          # Linker config (gcc)
├── xcontrol/                   # Windows agent
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # Entry point, GUI, WebSocket worker
│       ├── capture.rs          # Screen capture → JPEG
│       └── input.rs            # Mouse/keyboard input injection
├── xcontrol-relay/             # Rust relay
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # Axum WebSocket relay
│       └── web/index.html      # Web viewer (embedded)
└── dist/                       # Distribution artifacts
    ├── xcontrol.exe
    ├── start.bat
    └── README.txt
```

### 8.2 Build Commands

```powershell
# Debug build
cargo build

# Release build (optimized, stripped)
cargo build --release

# Build only the agent
cargo build -p xcontrol --release

# Build only the relay
cargo build -p xcontrol-relay --release
```

### 8.3 Release Profile

```toml
[profile.release]
opt-level = "z"    # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Single codegen unit for better LTO
strip = true        # Strip debug symbols
panic = "abort"     # No unwind tables
```

---

## 9. Deployment (Relay on Vultr)

```
IPv6: 2001:19f0:8000:385b:5400:06ff:fe43:eb83
Relay URL: ws://[2001:19f0:8000:385b:5400:06ff:fe43:eb83]/ws
Web Viewer URL: http://[2001:19f0:8000:385b:5400:06ff:fe43:eb83]/
```

### Components

| Component | Path | Port |
|---|---|---|
| Node.js relay | `/root/relay/server.js` | 8080 |
| Caddy reverse proxy | `/etc/caddy/Caddyfile` | 80 |
| Boot script | `/etc/local.d/xcontrol.start` | — |

### Restart

```bash
pkill -f 'node /root/relay/server.js'
cd /root/relay && nohup node server.js > /tmp/relay.log 2>&1 &
rc-service caddy restart
```

### Firewall

```bash
ufw allow 80
ufw allow 443
```
