#!/usr/bin/env bash
set -e
echo "Cleaning Penguclip..."
pkill -9 penguclip 2>/dev/null || true
rm -f ~/.local/bin/penguclip
rm -f ~/.local/share/applications/penguclip*.desktop
rm -rf ~/.penguclip
echo "Clean done."
echo ""
echo "Now run:"
echo "  cd ~/Penguclip && pnpm tauri build 2>&1 | tail -5"
echo "  cp src-tauri/target/release/penguclip ~/.local/bin/penguclip"
echo "  WEBKIT_DISABLE_COMPOSITING_MODE=1 GDK_BACKEND=x11 DISPLAY=:0 penguclip"
