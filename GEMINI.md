# Gemini Project Instructions: bg_core_v2

This document provides context and instructions for AI agents working on the `bg_core_v2` project.

## Project Overview

`bg_core_v2` is a high-performance live wallpaper application for Windows, written in Rust. It visualizes real-time CPU activity through an interactive particle system rendered using `wgpu`.

### Key Features
- **CPU Visualization (v2.0 "Atomic Starlink"):** 24 glowing spheres orbit in a dense, spherical Starlink-style constellation. Movement speed and colors are synced to overall system load.
- **Dynamic Particle Spray:** Cores emit high-energy particles (Sparks) and persistent tails when CPU load increases.
- **Smooth Load Transitions:** Visual changes (color, speed, atmospherics) are interpolated per-frame for maximum smoothness.
- **Desktop Integration:** Parents windows to the Windows desktop background, sitting behind icons.
- **Multi-Monitor Support:** Detects monitor layout and spawns an independent rendering window for each monitor with 1:1 aspect ratio mapping.
- **High Performance:** Uses High Process Priority and optimized `wgpu` instancing to remain fluid under 100% system load.
- **Tray Management:** Context menu for Color Presets, FPS (30/60), and Wallpaper Load Effect toggle.

## Version History

### v2.3 - "Aesthetic & Atmospheric Refinement" (Current)
- **Visuals**: 
  - Central Logo: Doubled in size and implemented high-contrast color logic per preset.
  - Particle Tails: Redesigned to align with core movement direction; increased thickness and added chaotic jitter (Rocket Exhaust effect).
  - Spark Effects: Increased frequency and segment count; switched to contrast colors for maximum visibility.
  - Background FX: Improved 2D chaotic noise to eliminate jitter texture; implemented dramatic slow-motion ripples (1.5s period).
- **Atmosphere**: Implemented global linear screen darkening synchronized with desaturation during high CPU load.
- **Smart Mapping**: Transitioned from UV Span to independent per-monitor wallpaper mapping, ensuring correct aspect ratio on all screens.
- **UI/UX**: Simplified tray menu and settings window by hardcoding visual proportions and focusing on core color/effect controls.

### v2.2 - "High-Fidelity Wallpaper Rendering"
- **Visuals**: Restored high-definition desktop wallpaper rendering using a full-screen background quad with 1:1 pixel mapping.
- **Clarity Optimization**: Removed rendering resolution caps; implemented native resolution rendering for ultra-sharp visuals on 2K/4K displays.
- **Lossless Assets**: Wallpaper is loaded at original resolution with zero compression. Applied 16x Anisotropic Filtering.
- **Stability**: Upgraded to a DirectComposition (DComp) visual tree windowing architecture.

### v2.1 - "Opaque Wallpaper Rendering"
- **Visuals**: Removed background starfield; moved the atomic constellation to the top-right corner (legacy).
- **Transparency Solution**: Queries the Windows active wallpaper file via `SPI_GETDESKWALLPAPER`.
- **Memory Optimization**: Automatically downscales high-resolution wallpaper images using thumbnail downscaling.
- **Stability**: Bypassed OS composition bugs using `wgpu::CompositeAlphaMode::Opaque`.

### v2.0 - "Atomic Starlink"
- **Visualization**: Switched from planar orbits to a 3D spherical "Starlink" distribution with 24 cores.
- **Performance**: Implemented High Process Priority logic in `main.rs`.
- **Depth Fix**: Remapped Z-coordinates in the shader to prevent clipping.

## Technical Architecture

- **`src/main.rs`**: Orchestrator. Handles Win32 windowing, tray management, and the main loop.
- **`src/renderer.rs`**: `wgpu` pipeline. Implements spherical orbits, movement-aligned particle physics, high-contrast logo rendering, and dynamic background atmospherics (noise, ripples, darkening).
- **`src/window.rs`**: Monitor detection and DirectComposition target management.
- **`src/cpu.rs`**: Win32 monitor using `GetSystemTimes` and `NtQuerySystemInformation`.
- **`src/app.rs`**: App state management (Color Presets, FPS, BG Toggles) and global logging.
- **`src/settings_win.rs`**: Compact Win32 settings interface for effect adjustments.
- **`src/tray.rs`**: System tray context menu management.

## Build and Run

- **Run**: `cargo run --release` (Highly recommended for performance).
- **Build**: `cargo build --release`.

## Logging
Output is written to `wallpaper_new.log` (and legacy `wallpaper.log`).
