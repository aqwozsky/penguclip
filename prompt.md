Act as an Expert Rust Systems Engineer and Senior Tauri Developer with deep expertise in Linux audio/video subsystems (PipeWire, Wayland, X11).

I am building a production-ready, high-performance background clipping application for Linux (similar to Medal.tv). The absolute highest priority is zero-to-minimal FPS drop during gaming. I need a comprehensive, end-to-end implementation guide, including detailed architecture explanations and the full code implementation for both the Rust backend and the Tauri frontend.

Here are the strict project requirements:

1. Tech Stack: Rust (Backend), Tauri (Frontend GUI), and FFmpeg (for processing/encoding).
2. Universal Linux Compatibility: The screen capture must work flawlessly across all major Linux distros, desktop environments (KDE, GNOME, Hyprland, etc.), and windowing systems (Wayland and X11). You MUST use PipeWire and XDG Desktop Portal for efficient, native screen capture to avoid compositing overhead.
3. Performance Optimization: Utilize hardware-accelerated encoding via FFmpeg (e.g., VA-API for AMD/Intel, NVENC for NVIDIA) to ensure gameplay FPS is entirely unaffected. Implement an efficient continuous memory or disk-based ring buffer for the background recording.
4. App State & First-Launch Setup Flow:
   - On the very first launch, the app must show a "Setup Wizard" screen where the user configures:
     * Output folder for saved clips.
     * Recording FPS (e.g., 30, 60, 120).
     * Video Quality/Bitrate (Low, Medium, High).
   - This configuration must be saved persistently (e.g., to `~/.config/` or Tauri's app-data directory).
   - On all subsequent launches, the app must read this config, bypass the Setup screen entirely, and open directly to the Main App Dashboard.
5. Core Feature: A reliable global hotkey listener. When the hotkey is pressed, the app must extract the last X seconds from the continuous buffer and save it to the selected folder.
6. File Naming Convention: Clips must be strictly named as `Clip_YYYY-MM-DD_HH-MM.mp4`.
7. UI/UX Design: A modern, sleek Black & White themed interface. Include a clear placeholder/component where I can insert my own custom logo.
8. Licensing: The project will be Open Source. Briefly outline the directory structure and where to place an MIT or GPLv3 license file.

YOUR TASK:
Provide the complete blueprint and implementation. Your response must include:
- A clear architectural explanation of how the Rust + PipeWire + FFmpeg ring buffer pipeline will operate safely and efficiently.
- Complete setup instructions (Cargo.toml dependencies, Tauri configuration, required system packages).
- The Rust backend code (App state management, global hotkeys, FFmpeg bindings/subprocess management, and PipeWire capture integration).
- The Tauri frontend code (Routing logic for the First-Launch Setup vs. Main Dashboard, and the B&W UI implementation).

Do not hold back; generate the full, production-ready solution with clear, explanatory comments throughout the code.
