#!/usr/bin/env bash
# Penguclip installer — builds and installs system-wide.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="$HOME/.local/bin"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"

echo "=== Penguclip Installer ==="
echo ""

# ─── Build ─────────────────────────────────────────────────────
echo "[1/4] Building release binary..."
cd "$SCRIPT_DIR"

if command -v pnpm &>/dev/null; then
    pnpm install --silent 2>/dev/null || true
    pnpm tauri build 2>&1 | grep -E "Finished|error|Bundle" || true
else
    echo "pnpm not found. Install Node.js + pnpm first."
    exit 1
fi

# Find the built binary
BINARY=$(find src-tauri/target/release -maxdepth 1 -name penguclip -type f 2>/dev/null | head -1)
if [ -z "$BINARY" ]; then
    # Try debug binary as fallback
    BINARY=$(find src-tauri/target/debug -maxdepth 1 -name penguclip -type f 2>/dev/null | head -1)
fi

if [ -z "$BINARY" ]; then
    echo "Binary not found. Build may have failed. Check output above."
    exit 1
fi

echo "  Binary: $BINARY"

# ─── Install binary ────────────────────────────────────────────
echo "[2/4] Installing binary to $BIN_DIR/penguclip..."
mkdir -p "$BIN_DIR"
cp "$BINARY" "$BIN_DIR/penguclip"
chmod +x "$BIN_DIR/penguclip"

# Check if ~/.local/bin is in PATH
if ! echo "$PATH" | grep -q "$BIN_DIR"; then
    echo "  ⚠ $BIN_DIR is not in your PATH."
    echo "  Add this to your ~/.bashrc or ~/.config/fish/config.fish:"
    echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# ─── Desktop entry ─────────────────────────────────────────────
echo "[3/4] Creating desktop entry..."
mkdir -p "$DESKTOP_DIR"
mkdir -p "$ICON_DIR"

# Copy icon if available
if [ -f "$SCRIPT_DIR/src-tauri/icons/128x128.png" ]; then
    cp "$SCRIPT_DIR/src-tauri/icons/128x128.png" "$ICON_DIR/penguclip.png"
elif [ -f "$SCRIPT_DIR/src-tauri/icons/icon.png" ]; then
    cp "$SCRIPT_DIR/src-tauri/icons/icon.png" "$ICON_DIR/penguclip.png"
fi

cat > "$DESKTOP_DIR/penguclip.desktop" << 'DESKTOP'
[Desktop Entry]
Name=Penguclip
Comment=High-performance background clipping for Linux
Exec=penguclip
Icon=penguclip
Terminal=false
Type=Application
Categories=Utility;Video;AudioVideo;
Keywords=clip;recording;gaming;screen;capture;
StartupWMClass=penguclip
DESKTOP

echo "  Desktop entry: $DESKTOP_DIR/penguclip.desktop"

# ─── App directory ─────────────────────────────────────────────
echo "[4/4] Creating ~/.penguclip/..."
mkdir -p "$HOME/.penguclip/clips"
mkdir -p "$HOME/.penguclip/buffer"

echo ""
echo "=== Done! ==="
echo ""
echo "  Run from terminal:  penguclip"
echo "  Or find 'Penguclip' in your app launcher"
echo "  Clips saved to:      ~/.penguclip/clips/"
echo "  Config at:           ~/.penguclip/config.json"
echo ""
echo "  Hotkey: Ctrl+R (configurable in settings)"
echo ""
