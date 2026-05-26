# Gemini Project Instructions: StarCore

This document provides context and instructions for AI agents working on the **StarCore** project.

## Project Overview

**StarCore** is a high-performance live wallpaper application for Windows, written in Rust. It visualizes real-time CPU activity through an interactive "Atomic Starlink" particle system rendered using `wgpu`.

### Key Features
- **Atomic Starlink Visualization (v0.1):** 24 glowing spheres orbit in a dense, spherical constellation. Movement speed and colors are dynamically synced to overall system load.
- **Dynamic Particle Physics:** Cores emit high-energy sparks and persistent tails. Tail alignment is calculated based on movement vectors for a "rocket exhaust" effect.
- **Automatic Wallpaper Integration:** 
  - Automatically detects and reads the system's wallpaper fit style (Stretch, Fill, Center) from the Windows Registry.
  - Parents windows directly to the Windows desktop background.
- **Multi-Monitor Support:** Detects monitor layout and spawns an independent rendering window for each screen with pixel-perfect aspect ratio mapping.
- **High-Performance Rendering:** Uses `wgpu` with optimized instancing and High Process Priority to ensure fluid animation even under 100% CPU load.
- **StarCore Tray Management:** 
  - **Quick Color Presets:** Switch between 10 high-contrast themes directly from the tray menu.
  - **Wallpaper Load Effect:** Toggleable real-time background ripples and desaturation effects based on CPU stress.

## Version History

### v0.1 - "The First Star" (Current)
- **Rebranding**: Officially named **StarCore**. Consolidated versioning to v0.1.
- **Automatic Fit**: Implemented Registry-based wallpaper style detection (Stretch/Fill/Center).
- **Tray UI Refinement**: Moved Color Presets to the primary context menu for easier access.
- **Code Optimization**: Cleaned up unused WGPU/DComp variables and resolved compiler warnings.
- **Visuals**: 
  - High-contrast Central Logo logic.
  - Movement-aligned particle tails with chaotic jitter.
  - Atmospheric background effects (Ripples, Desaturation, Darkening).

---
*Legacy Note: This project evolved from the `bg_core_v2` research prototype.*

## Technical Architecture

- **`src/main.rs`**: Orchestrator. Handles Win32 windowing, tray management, and the high-precision event loop.
- **`src/renderer.rs`**: Core `wgpu` engine. Implements the WGSL shader, particle physics, Registry-based wallpaper mapping, and atmospheric effects.
- **`src/window.rs`**: Win32 windowing helpers, DPI awareness, and monitor layout detection.
- **`src/cpu.rs`**: High-frequency CPU monitor using low-level Win32 APIs.
- **`src/app.rs`**: Application state, color preset definitions, and settings persistence.
- **`src/tray.rs`**: StarCore tray icon and context menu management.
- **`src/settings_win.rs`**: Win32 settings dialog for advanced configurations.

## Build and Run

- **Run**: `cargo run --release` (Mandatory for smooth performance).
- **Build**: `cargo build --release`.

## Logging
Primary output is written to `wallpaper_new.log`.
