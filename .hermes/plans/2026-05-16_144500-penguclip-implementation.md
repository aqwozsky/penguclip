# Penguclip — Complete Implementation Plan

**Goal:** Build a Medal.tv-like background clipping app for Linux (Rust + Tauri + PipeWire + FFmpeg).

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Tauri Frontend (React/TS)             │
│  ┌──────────┐  ┌──────────────────┐  ┌───────────────┐ │
│  │SetupWizard│  │  MainDashboard   │  │   Logo Slot   │ │
│  │(1st run) │  │(recording status) │  │               │ │
│  └──────────┘  └──────────────────┘  └───────────────┘ │
├─────────────────────────────────────────────────────────┤
│              Tauri IPC (invoke commands)                 │
├─────────────────────────────────────────────────────────┤
│                   Rust Backend                           │
│  ┌───────────┐  ┌──────────┐  ┌──────────────────────┐ │
│  │  Config   │  │  Hotkey  │  │  Capture Pipeline     │ │
│  │  Manager  │  │ Listener │  │  (Portal → FFmpeg)   │ │
│  └───────────┘  └──────────┘  └──────────────────────┘ │
│  ┌──────────────────────────────────────────────────┐  │
│  │            Ring Buffer Manager                    │  │
│  │    (FFmpeg segmented encoding → rotating files)   │  │
│  └──────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────┤
│  PipeWire  ←→  XDG Desktop Portal  ←→  FFmpeg (VA-API/NVENC) │
└─────────────────────────────────────────────────────────┘
```

### Pipeline Flow
1. **XDG Desktop Portal ScreenCast** → Gets PipeWire fd for screen capture
2. **GStreamer/PipeWire** → Reads raw video frames from the fd
3. **FFmpeg subprocess** → Receives frames via pipe, encodes with HW acceleration
4. **Segment muxer** → Writes 2-second .mp4 segments to rotating ring buffer (~30 files)
5. **Hotkey trigger** → Concatenates last N segments into `Clip_YYYY-MM-DD_HH-MM.mp4`
6. **Config persistence** → ~/.config/dev.aqwozsky.penguclip/config.json

### Performance Strategy
- **Zero GPU copy**: Portal returns DMA-BUF fd → PipeWire shares with FFmpeg
- **Hardware encoding**: VA-API (Intel/AMD) or NVENC (NVIDIA) — no CPU encoding
- **Segment-based ring buffer**: No massive in-memory buffer; 2-sec segments on disk
- **FFmpeg subprocess**: Separate process, doesn't block Tauri main thread

## Directory Structure (Final)

```
Penguclip/
├── src/                          # React frontend
│   ├── App.tsx                   # Router (setup vs main)
│   ├── App.css                   # B&W theme styles
│   ├── main.tsx                  # React entry
│   ├── types.ts                  # Shared types
│   ├── tauri-api.ts              # All invoke() calls
│   ├── components/
│   │   ├── SetupWizard.tsx       # First-launch config screen
│   │   ├── MainDashboard.tsx     # Main app dashboard
│   │   ├── LogoPlaceholder.tsx   # Custom logo slot
│   │   └── StatusBar.tsx         # Recording/hotkey status
│   └── assets/
│       └── logo-placeholder.svg  # Logo placeholder
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/default.json
│   ├── build.rs
│   ├── icons/
│   └── src/
│       ├── main.rs               # Entry point
│       ├── lib.rs                # Tauri setup + command registration
│       ├── config.rs             # Config struct + persistence
│       ├── capture.rs            # XDG Portal + PipeWire capture
│       ├── encoder.rs            # FFmpeg subprocess management
│       ├── hotkey.rs             # Global hotkey listener
│       └── ring_buffer.rs        # Ring buffer (segment management + clip saving)
├── package.json
├── tsconfig.json
├── vite.config.ts
├── index.html
├── LICENSE                       # MIT or GPLv3
└── README.md
```

## Implementation Steps

### Phase 1: Rust Backend Foundation
- [ ] 1.1 Update Cargo.toml with all dependencies
- [ ] 1.2 Implement config.rs — AppConfig struct + JSON persistence
- [ ] 1.3 Implement hotkey.rs — Global hotkey listener (rdev crate, X11 primary)
- [ ] 1.4 Update lib.rs — State management, command registration

### Phase 2: Capture Pipeline
- [ ] 2.1 Implement capture.rs — XDG Desktop Portal ScreenCast via ashpd
- [ ] 2.2 Implement encoder.rs — FFmpeg subprocess with HW encoding detection
- [ ] 2.3 Implement ring_buffer.rs — Segment rotation + clip concatenation

### Phase 3: Tauri Frontend
- [ ] 3.1 Install react-router-dom for routing
- [ ] 3.2 Build SetupWizard component (output folder, FPS, quality)
- [ ] 3.3 Build MainDashboard component (recording status, recent clips)
- [ ] 3.4 Build LogoPlaceholder component
- [ ] 3.5 Implement B&W theme in App.css

### Phase 4: Integration & Polish
- [ ] 4.1 Wire all Tauri commands to frontend
- [ ] 4.2 Add LICENSE file
- [ ] 4.3 Update README.md with setup instructions
- [ ] 4.4 Test compilation

## Dependencies

### System Packages (pacman/CachyOS)
```
pipewire pipewire-pulse wireplumber
xdg-desktop-portal xdg-desktop-portal-gtk xdg-desktop-portal-hyprland
ffmpeg
```

### Rust Crates (Cargo.toml)
```toml
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ashpd = { version = "0.10", features = ["tokio", "pipewire"] }
tokio = { version = "1", features = ["full"] }
rdev = "0.5"
dirs = "6"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
log = "0.4"
env_logger = "0.11"
uuid = { version = "1", features = ["v4"] }
```

### Frontend (package.json)
```json
"react-router-dom": "^7",
"@tauri-apps/plugin-dialog": "^2",
"@tauri-apps/plugin-fs": "^2"
```

## Key Design Decisions

1. **Capture approach**: XDG Desktop Portal ScreenCast (NOT raw PipeWire API). This works on GNOME, KDE, Hyprland, Sway, and handles permission prompts.
2. **Ring buffer**: FFmpeg segment muxer with segment_wrap for rotating file buffer. No in-memory frame buffer needed.
3. **Hotkey on Wayland**: rdev (XRecord) for X11. For Wayland, provide fallback: CLI arg + DBus activation.
4. **Encoding detection**: Auto-detect VA-API, NVENC, or fall back to software x264.
5. **Config path**: `dirs::config_dir()/dev.aqwozsky.penguclip/config.json`

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Portal not available (minimal DE) | Graceful fallback with error message + instructions |
| HW encoding not supported | Fall back to libx264 with warning |
| Wayland hotkey limitations | Provide system shortcut workaround + DBus listener |
| PipeWire version incompatibility | Check pw version at startup, document min requirements |
| Performance regression with high FPS | Segment size tuning; ensure FFmpeg uses zero-copy |
