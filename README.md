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

## Usage

### Launch from terminal

```bash
penguclip
```

### NVIDIA + Wayland fix

On **NVIDIA GPUs with Wayland**, WebKitGTK produces a blank window or crashes with `Error 71 (Protocol error)`. Run with:

```bash
WEBKIT_DISABLE_COMPOSITING_MODE=1 GDK_BACKEND=x11 DISPLAY=:0 penguclip
```

To make this permanent, create an alias:

```bash
# Add to ~/.bashrc or ~/.config/fish/config.fish:
alias penguclip='WEBKIT_DISABLE_COMPOSITING_MODE=1 GDK_BACKEND=x11 DISPLAY=:0 penguclip'
```

Or copy the desktop entry with the fix applied:

```bash
cp ~/.local/share/applications/penguclip.desktop ~/.local/share/applications/penguclip-wayland.desktop
sed -i 's|^Exec=.*|Exec=env WEBKIT_DISABLE_COMPOSITING_MODE=1 GDK_BACKEND=x11 DISPLAY=:0 penguclip|' ~/.local/share/applications/penguclip-wayland.desktop
update-desktop-database ~/.local/share/applications/
```

### First launch

The setup wizard appears. Configure:
- **Output folder** — where clips are saved (default: `~/.penguclip/clips/`)
- **Clip mode** — Anything / Games Only / Specific Apps
- **FPS, quality, duration, hotkey**

After setup, press **Ctrl+R** (or your configured hotkey) to save clips. Start recording first via the Dashboard tab.

### Wayland hotkey setup (Hyprland, Sway, etc.)

Global hotkeys via XRecord only work on X11. On Wayland, bind a compositor shortcut:

**Hyprland** — add to `~/.config/hypr/hyprland.conf`:
```
bind = $mainMod, R, exec, penguclip --trigger-clip
```

**Sway** — add to `~/.config/sway/config`:
```
bindsym $mod+r exec penguclip --trigger-clip
```

This sends a trigger to the running penguclip instance via a Unix socket at `~/.penguclip/socket`. Make sure penguclip is already running first.

All data lives in `~/.penguclip/`:
```
~/.penguclip/
├── config.json    ← your settings
├── clips/         ← saved MP4 clips
└── buffer/        ← temporary recording segments
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
