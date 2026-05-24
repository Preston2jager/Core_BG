# Gemini Project Instructions: bg_core_v2

This document provides context and instructions for AI agents working on the `bg_core_v2` project.

## Project Overview

`bg_core_v2` is a high-performance live wallpaper application for Windows, written in Rust. It visualizes real-time CPU activity through an interactive particle system rendered using `wgpu`.

### Key Features
- **CPU Visualization (v2.0 "Atomic Starlink"):** 24 glowing spheres orbit in a dense, spherical Starlink-style constellation. Movement speed and colors are synced to overall system load.
- **Dynamic Particle Spray:** Cores emit high-energy particles when CPU load exceeds 50%. Particle frequency and velocity scale with load.
- **Smooth Load Transitions:** Visual changes (color, speed) are interpolated per-frame for maximum smoothness.
- **Desktop Integration:** Parents windows to the Windows desktop background, sitting behind icons.
- **Multi-Monitor Support:** Detects monitor layout and spawns a rendering window for each monitor.
- **High Performance:** Uses High Process Priority to remain fluid even under 100% system load. Optimized 3D math and `wgpu` instancing.
- **Tray Management:** Context menu for Pause, FPS (30/60), Twinkle toggle, and Glow intensity.

## Version History

### v2.2 - "High-Fidelity Wallpaper Rendering" (Current)
- **Visuals**: Restored high-definition desktop wallpaper rendering using a full-screen background quad with 1:1 pixel mapping.
- **Clarity Optimization**: Removed rendering resolution caps (previously 960px); implemented native resolution rendering for ultra-sharp visuals on 2K/4K displays.
- **Lossless Assets**: Wallpaper is loaded at original resolution with zero compression. Applied 16x Anisotropic Filtering for maximum texture sharpness.
- **Smart Mapping**: Implemented dynamic "Cover" aspect ratio correction and multi-monitor coordinate mapping (UV Span) to prevent stretching across different screen layouts.
- **Stability**: Upgraded to a DirectComposition (DComp) visual tree windowing architecture while maintaining robust rendering.

### v2.1 - "Opaque Wallpaper Rendering"
- **Visuals**: Removed background starfield; moved the atomic constellation to the top-right corner of each display.
- **Transparency Solution**: Queries the Windows active wallpaper file via `SPI_GETDESKWALLPAPER`, robustly decodes extensionless JPEG files (`TranscodedWallpaper`), and draws it as a full-screen background quad behind the cores and particles.
- **Memory Optimization**: Automatically downscales high-resolution wallpaper images using thumbnail downscaling.
- **Stability**: Bypassed OS composition bugs in transparent layered child windows by using `wgpu::CompositeAlphaMode::Opaque` on a standard child window layout.

### v2.0 - "Atomic Starlink"
- **Visualization**: Switched from planar orbits to a 3D spherical "Starlink" distribution with 24 cores.
- **Performance**: Implemented High Process Priority logic in `main.rs` to prevent UI stuttering during heavy CPU usage.
- **Depth Fix**: Remapped Z-coordinates in the shader to prevent spheres from being clipped when rotating to the back.
- **Background**: Enhanced starfield with 800 twinkling, rotating stars.
- **Stability**: Fixed wgpu handle ownership and struct alignment for reliable per-core monitoring.

### v1.0 - Initial Release
- Planar disk-like orbits.
- Per-core load visualization.
- Basic background stars.

## Technical Architecture

- **`src/main.rs`**: Orchestrator. Handles Win32 windowing, priority class, and the main loop.
- **`src/renderer.rs`**: `wgpu` pipeline. Implements spherical orbits, particle physics, and 1:1 pixel-perfect wallpaper quad rendering with DComp visual tree persistence.
- **`src/window.rs`**: Monitor detection and window synchronization. Manages the transition to DirectComposition targets.
- **`src/cpu.rs`**: Win32 monitor using `GetSystemTimes` and `NtQuerySystemInformation` (aligned for x64).
- **`src/app.rs`**: App state management and global log management.

## Build and Run

- **Run**: `cargo run --release` (Highly recommended for performance).
- **Build**: `cargo build --release`.

## Logging
Output is written to `wallpaper_new.log` (and legacy `wallpaper.log`).
