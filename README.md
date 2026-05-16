# Penguclip 🐧

High-performance background clipping for Linux — capture your best gaming moments without dropping a single frame.

## Features

- **Zero FPS impact** — Hardware-accelerated encoding via VA-API (Intel/AMD) or NVENC (NVIDIA)
- **Universal Linux support** — Works on X11 and Wayland (GNOME, KDE, Hyprland, Sway)
- **Continuous ring buffer** — Always recording the last N seconds, ready to save
- **Global hotkey** — Press a key combo to instantly save a clip
- **Setup wizard** — Configure output folder, FPS, quality, and hotkey on first launch
- **B&W theme** — Clean, modern interface that stays out of your way

## System Requirements

- **Linux** with PipeWire and XDG Desktop Portal
- **FFmpeg** installed (`sudo pacman -S ffmpeg`)
- For hardware encoding:
  - **Intel/AMD**: `libva` and `mesa` (`sudo pacman -S libva mesa`)
  - **NVIDIA**: NVIDIA drivers with NVENC support

### Install dependencies (Arch / CachyOS)

```bash
sudo pacman -S ffmpeg pipewire wireplumber \
  xdg-desktop-portal xdg-desktop-portal-gtk \
  libva mesa
```

For Hyprland users:
```bash
sudo pacman -S xdg-desktop-portal-hyprland
```

## Development

```bash
# Install frontend dependencies
pnpm install

# Run in development mode (hot reload)
pnpm tauri dev

# Build for production
pnpm tauri build
```

### Linux GPU quirks

If you see a blank window or GBM buffer errors on NVIDIA + Wayland:

```bash
WEBKIT_DISABLE_COMPOSITING_MODE=1 GDK_BACKEND=x11 DISPLAY=:0 ./src-tauri/target/release/penguclip
```

## Project Structure

```
Penguclip/
├── src/                    # React frontend (TypeScript)
│   ├── App.tsx             # Router (setup wizard vs main dashboard)
│   ├── App.css             # Black & White theme
│   ├── types.ts            # Shared TypeScript types
│   ├── tauri-api.ts        # All Tauri invoke() calls
│   └── components/
│       ├── SetupWizard.tsx  # First-launch configuration
│       ├── MainDashboard.tsx # Main app controls
│       ├── StatusBar.tsx    # Recording status indicator
│       └── LogoPlaceholder.tsx # Custom logo slot
├── src-tauri/              # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs          # Entry point
│       ├── lib.rs           # Tauri commands + app setup
│       ├── config.rs        # AppConfig persistence
│       ├── capture.rs       # XDG Desktop Portal ScreenCast
│       ├── encoder.rs       # FFmpeg subprocess management
│       ├── hotkey.rs        # Global hotkey listener
│       └── ring_buffer.rs   # Segment rotation + clip saving
├── LICENSE                  # MIT
└── README.md
```

## How It Works

1. **Screen capture** via XDG Desktop Portal ScreenCast — works on all Linux DEs
2. **FFmpeg** encodes the capture stream using hardware acceleration
3. **Segmented ring buffer** keeps the last ~2 minutes of footage as 2-second MP4 chunks
4. **Global hotkey** (via `rdev` crate on X11) triggers clip extraction
5. **Clip saved** as `Clip_YYYY-MM-DD_HH-MM.mp4` to your configured output folder

## Wayland Hotkey Setup

On Wayland, global hotkeys require compositor cooperation. Configure a system shortcut that sends SIGUSR1 to the penguclip process:

```bash
# Find the PID and send signal
pkill -USR1 penguclip
```

Or bind it in your Hyprland config:
```
bind = $mainMod, R, exec, pkill -USR1 penguclip
```

## License

[GNU GPLv3](LICENSE) — Free software. Copy, modify, distribute — just keep it free.
