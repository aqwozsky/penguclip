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

- **Any Linux distribution** with PipeWire + XDG Desktop Portal
- **FFmpeg** (for encoding and clip processing)
- **Tauri v2 system deps** (WebKitGTK, etc. — see [Tauri docs](https://v2.tauri.app/start/prerequisites/#linux))
- Hardware encoding (optional but recommended):
  - **Intel/AMD**: VA-API (`libva`, `mesa`)
  - **NVIDIA**: proprietary drivers with NVENC
- **xdotool** or **wmctrl** (optional, for app/window listing)

### Install dependencies

| Distro | Command |
|--------|---------|
| **Arch / CachyOS** | `sudo pacman -S ffmpeg pipewire wireplumber xdg-desktop-portal xdg-desktop-portal-gtk libva mesa webkit2gtk-4.1` |
| **Fedora** | `sudo dnf install ffmpeg-free pipewire wireplumber xdg-desktop-portal xdg-desktop-portal-gtk libva mesa-libVA webkit2gtk4.1-devel` |
| **Ubuntu / Debian** | `sudo apt install ffmpeg pipewire wireplumber xdg-desktop-portal xdg-desktop-portal-gtk libva2 mesa-va-drivers libwebkit2gtk-4.1-dev` |
| **openSUSE** | `sudo zypper install ffmpeg pipewire wireplumber xdg-desktop-portal xdg-desktop-portal-gtk libva2 Mesa-libva libwebkit2gtk-4_1-0` |
| **NixOS** | Add `ffmpeg pipewire wireplumber xdg-desktop-portal libva` to `environment.systemPackages` |
| **Void** | `sudo xbps-install ffmpeg pipewire wireplumber xdg-desktop-portal xdg-desktop-portal-gtk libva mesa-vaapi webkit2gtk-devel` |

For **Hyprland / Sway / wlroots**: also install `xdg-desktop-portal-hyprland` or `xdg-desktop-portal-wlr`.

### What the code depends on (distro-agnostic)

Penguclip uses only standard Linux interfaces — nothing distro-specific:

| Component | Linux API | Works on |
|-----------|-----------|----------|
| Screen capture | XDG Desktop Portal ScreenCast | All DEs (GNOME, KDE, Hyprland, Sway, XFCE, ...) |
| Audio/video streams | PipeWire | Standard since 2021 |
| Hardware encoding | VA-API / NVENC via FFmpeg | All GPUs, all distros |
| Global hotkeys | XRecord extension (X11) | Every X11 desktop |
| File dialogs | XDG Desktop Portal / Tauri | All DEs |
| GPU detection | `nvidia-smi` + `vainfo` | Standard tools |

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
