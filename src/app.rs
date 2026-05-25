use crate::cpu::CpuMonitor;
use crate::renderer::Renderer;
use crate::window::{self, DesktopInfo};
use windows_sys::Win32::Foundation::*;
use std::sync::Mutex;
use std::io::{Write, Read};
use std::fs::{OpenOptions, File};

pub fn log_msg(msg: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("wallpaper_new.log") {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ColorPreset {
    AtomicStarlink,
    Cyberpunk,
    AcidGreen,
    SolarFlame,
    DeepOcean,
    EmeraldPulse,
    CrimsonNova,
    VioletNight,
    AmberGhost,
    FrostByte,
}

#[derive(Copy, Clone)]
pub struct AppState {
    pub is_paused: bool,
    pub fps: u32,
    pub glow: u8,
    pub should_exit: bool,
    
    pub color_preset: ColorPreset,
    pub bg_effect_enabled: bool,

    // Thread communication flags
    pub pending_refresh: bool,
    pub pending_logo_update: bool,
    pub pending_settings_show: bool,
}

pub static STATE: Mutex<AppState> = Mutex::new(AppState {
    is_paused: false,
    fps: 60,
    glow: 1,
    should_exit: false,
    
    color_preset: ColorPreset::AtomicStarlink,
    bg_effect_enabled: true,

    pending_refresh: false,
    pending_logo_update: false,
    pending_settings_show: false,
});

pub fn save_settings(state: &AppState) {
    let content = format!(
        "color_preset={:?}\n\
         fps={}\n\
         glow={}\n\
         bg_effect_enabled={}\n",
        state.color_preset,
        state.fps,
        state.glow,
        state.bg_effect_enabled,
    );
    if let Ok(mut file) = File::create("settings.txt") {
        let _ = file.write_all(content.as_bytes());
    }
}

pub fn load_settings() -> AppState {
    let mut state = AppState {
        is_paused: false,
        fps: 60,
        glow: 1,
        should_exit: false,
        color_preset: ColorPreset::AtomicStarlink,
        bg_effect_enabled: true,
        pending_refresh: false,
        pending_logo_update: false,
        pending_settings_show: false,
    };
    
    if let Ok(mut file) = File::open("settings.txt") {
        let mut buf = String::new();
        if file.read_to_string(&mut buf).is_ok() {
            for line in buf.lines() {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let val = parts[1].trim();
                    match key {
                        "fps" => if let Ok(v) = val.parse::<u32>() { state.fps = v; },
                        "glow" => if let Ok(v) = val.parse::<u8>() { state.glow = v; },
                        "bg_effect_enabled" => if let Ok(v) = val.parse::<bool>() { state.bg_effect_enabled = v; },
                        "color_preset" => {
                            state.color_preset = match val {
                                "Cyberpunk" => ColorPreset::Cyberpunk,
                                "AcidGreen" => ColorPreset::AcidGreen,
                                "SolarFlame" => ColorPreset::SolarFlame,
                                "DeepOcean" => ColorPreset::DeepOcean,
                                "EmeraldPulse" => ColorPreset::EmeraldPulse,
                                "CrimsonNova" => ColorPreset::CrimsonNova,
                                "VioletNight" => ColorPreset::VioletNight,
                                "AmberGhost" => ColorPreset::AmberGhost,
                                "FrostByte" => ColorPreset::FrostByte,
                                _ => ColorPreset::AtomicStarlink,
                            };
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    
    state
}

pub struct MonitorState {
    pub hwnd: HWND,
    pub rect: RECT,
    pub width: i32,
    pub height: i32,
    pub renderer: Renderer,
}

pub struct WallpaperApp {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub hinstance: HINSTANCE,
    pub monitor_states: Vec<MonitorState>,
    pub desktop_info: DesktopInfo,
    pub cpu_monitor: CpuMonitor,
    pub shared_resources: Option<std::sync::Arc<crate::renderer::SharedRenderResources>>,
}

impl WallpaperApp {
    pub fn new(hinstance: HINSTANCE) -> Self {
        log_msg("Initializing WallpaperApp...");
        
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })).expect("Failed to find a suitable GPU adapter");
        let adapter_info = adapter.get_info();
        log_msg(&format!("Selected GPU: {}, backend: {:?}", adapter_info.name, adapter_info.backend));

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Shared Wallpaper Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::MemoryUsage, // Optimize for memory!
            },
            None,
        )).expect("Failed to create wgpu device");

        let desktop_info = unsafe { window::get_desktop_info() };
        let cpu_monitor = CpuMonitor::new();

        Self {
            instance,
            adapter,
            device,
            queue,
            hinstance,
            monitor_states: Vec::new(),
            desktop_info,
            cpu_monitor,
            shared_resources: None,
        }
    }

    pub fn sync_monitors(&mut self) {
        // 1. Clean up old window states and surfaces/renderers FIRST.
        // We pop and drop the state (drops the renderer and surface), then destroy the window handle.
        while let Some(state) = self.monitor_states.pop() {
            let hwnd = state.hwnd;
            drop(state);
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
            }
        }

        // 2. Drop the old shared resources to release GPU memory before requesting the new device
        self.shared_resources = None;

        // 3. Recreate the WGPU context to handle display connection/disconnection safely
        log_msg("Recreating WGPU context during monitor sync...");
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })).expect("Failed to find a suitable GPU adapter");
        let adapter_info = adapter.get_info();
        log_msg(&format!("Recreated GPU: {}, backend: {:?}", adapter_info.name, adapter_info.backend));

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Shared Wallpaper Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::MemoryUsage, // Optimize for memory!
            },
            None,
        )).expect("Failed to create wgpu device");

        self.instance = instance;
        self.adapter = adapter;
        self.device = device;
        self.queue = queue;

        // 4. Call sync_monitor_windows
        unsafe {
            window::sync_monitor_windows(
                &self.instance,
                &self.adapter,
                &self.device,
                &self.queue,
                self.hinstance,
                &mut self.monitor_states,
                &mut self.shared_resources,
                &self.desktop_info,
            );
        }
    }

    pub fn update_and_draw(&mut self, delta_time: f32, overall_cpu: f32, glow: u8) {
        let (color_preset, bg_effect_enabled) = {
            let s = STATE.lock().unwrap();
            (s.color_preset, s.bg_effect_enabled)
        };

        // Calculate virtual screen in physical pixels from known monitors
        let mut v_left = 0;
        let mut v_top = 0;
        let mut v_right = 0;
        let mut v_bottom = 0;
        if !self.monitor_states.is_empty() {
            v_left = self.monitor_states[0].rect.left;
            v_top = self.monitor_states[0].rect.top;
            v_right = self.monitor_states[0].rect.right;
            v_bottom = self.monitor_states[0].rect.bottom;
            for state in &self.monitor_states[1..] {
                v_left = v_left.min(state.rect.left);
                v_top = v_top.min(state.rect.top);
                v_right = v_right.max(state.rect.right);
                v_bottom = v_bottom.max(state.rect.bottom);
            }
        }
        let v_rect = RECT { left: v_left, top: v_top, right: v_right, bottom: v_bottom };

        for state in &mut self.monitor_states {
            state.renderer.config_glow = glow;

            let (render_w, render_h) = window::compute_renderer_size(state.width, state.height);
            state.renderer.resize(&self.device, render_w, render_h, state.rect, v_rect);

            state.renderer.update(delta_time, overall_cpu, color_preset);
            state.renderer.draw(&self.device, &self.queue, color_preset, bg_effect_enabled);
        }
    }

    pub fn reload_wallpaper(&mut self) {
        if self.monitor_states.is_empty() { return; }
        log_msg("Reloading desktop wallpaper...");
        
        let format = self.monitor_states[0].renderer.surface_config.format;
        
        // Re-create shared resources, which reloads and downscales the system wallpaper
        let shared = std::sync::Arc::new(crate::renderer::SharedRenderResources::new(
            &self.device,
            &self.queue,
            format,
        ));
        
        self.shared_resources = Some(shared.clone());
        
        // Recreate the bind groups for each renderer
        for state in &mut self.monitor_states {
            state.renderer.shared_resources = shared.clone();
            state.renderer.recreate_bind_group(&self.device);
        }
        self.update_logo();
        log_msg("Desktop wallpaper successfully reloaded!");
    }

    pub fn update_logo(&self) {
        log_msg("Updating logo (Star preset only for V3)");

        let rgba = crate::renderer::generate_star_logo();

        if let Some(ref shared) = self.shared_resources {
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &shared.logo_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &rgba,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * 512),
                    rows_per_image: Some(512),
                },
                wgpu::Extent3d {
                    width: 512,
                    height: 512,
                    depth_or_array_layers: 1,
                },
            );
            log_msg("Logo texture successfully updated!");
        }
    }
}
