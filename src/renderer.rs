use rand::Rng;
use windows_sys::Win32::Foundation::{HWND, HINSTANCE};

const MAX_INSTANCES: usize = 15000;
const NUM_CORES: usize = 24;

const SHADER_SRC: &str = r#"
struct VertexInput {
    @builtin(vertex_index) vertex_idx : u32,
    @location(0) pos : vec3<f32>,
    @location(1) color : vec4<f32>,
    @location(2) size : f32,
    @location(3) p_type : f32,
    @location(4) orbit_u : vec3<f32>,
    @location(5) orbit_v : vec3<f32>,
    @location(6) angle : f32,
}

struct VertexOutput {
    @builtin(position) clip_position : vec4<f32>,
    @location(0) uv : vec2<f32>,
    @location(1) color : vec4<f32>,
    @location(2) @interpolate(flat) p_type : u32,
}

struct ViewportUniform {
    width : f32,
    height : f32,
    time : f32,
    load : f32,
    glow_factor : f32,
    bg_effect_enabled : f32,
    bg_effect_intensity : f32,
    core_x : f32,
    core_y : f32,
    _pad1 : f32,
    _pad2 : f32,
    _pad3 : f32,
}

@group(0) @binding(0) var<uniform> viewport : ViewportUniform;
@group(0) @binding(1) var t_logo : texture_2d<f32>;
@group(0) @binding(2) var s_logo : sampler;
@group(0) @binding(3) var t_wallpaper : texture_2d<f32>;
@group(0) @binding(4) var s_wallpaper : sampler;

@vertex
fn vs_main(input : VertexInput) -> VertexOutput {
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0)
    );
    let uv = uvs[input.vertex_idx];
    
    var world_pos = input.pos;
    if (u32(input.p_type) == 1u) {
        // GPU Orbit Calculation: Compute trig and vector math directly on GPU for perfect smoothness
        let cx = input.pos.x;
        let cy = input.pos.y;
        let r = input.pos.z;
        let cos_a = cos(input.angle);
        let sin_a = sin(input.angle);
        let orbit_pos = r * (cos_a * input.orbit_u + sin_a * input.orbit_v);
        world_pos = vec3<f32>(cx + orbit_pos.x, cy + orbit_pos.y, orbit_pos.z);
    }

    var offset_px = uv;
    if (u32(input.p_type) == 4u) {
        offset_px = uv * vec2<f32>(viewport.width * 0.5, viewport.height * 0.5);
    } else {
        offset_px = uv * input.size;
    }
    let pixel_pos = world_pos.xy + offset_px;

    let ndc_x = (pixel_pos.x / viewport.width) * 2.0 - 1.0;
    let ndc_y = (pixel_pos.y / viewport.height) * -2.0 + 1.0;

    var out : VertexOutput;
    let normalized_z = (world_pos.z + 500.0) / 1000.0;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, normalized_z, 1.0);
    out.uv = uv;
    out.color = input.color;
    out.p_type = u32(input.p_type);
    return out;
}

// Pseudo-random number generator for pixel-level noise jitter
fn rand2d(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.uv);
    
    if (in.p_type == 4u) {
        let tex_uv = (in.uv * 0.5 + vec2<f32>(0.5, 0.5)) * in.color.zw + in.color.xy;
        var final_uv = tex_uv;
        
        let pixel_center = vec2<f32>(viewport.core_x, viewport.core_y);
        let to_pixel = in.clip_position.xy - pixel_center;
        let dist_px = length(to_pixel);

        if (viewport.bg_effect_enabled > 0.5 && viewport.bg_effect_intensity > 0.0) {
            // 1. Dramatic Slow-Motion Shockwave
            let period = 1.5; 
            let pulse_time = fract(viewport.time / period) * period;
            let speed = 1200.0 + 800.0 * viewport.bg_effect_intensity; 
            let radius = speed * pulse_time;
            let dist_to_wavefront = abs(dist_px - radius);
            let thickness = 150.0; 
            
            var radial_dir = vec2<f32>(0.0, 0.0);
            if (dist_px > 0.1) {
                radial_dir = to_pixel / dist_px;
            }
            
            let wave = sin((dist_px - radius) * 0.05); 
            let envelope = smoothstep(thickness, 0.0, dist_to_wavefront);
            let fade = smoothstep(period, period * 0.6, pulse_time) * smoothstep(0.0, 0.2, pulse_time);
            
            let ripple_displacement = wave * 0.0065 * viewport.bg_effect_intensity * envelope * fade;
            final_uv = final_uv + radial_dir * ripple_displacement;
            
            // 2. High-Intensity Spiritual Pressure Jitter
            let jitter_amp = 0.018 * viewport.bg_effect_intensity; 
            let time_seed = floor(viewport.time * 48.0); 
            let noise_seed_x = in.clip_position.xy + vec2<f32>(time_seed * 1.56, time_seed * 9.8);
            let noise_seed_y = in.clip_position.xy + vec2<f32>(time_seed * -0.74, time_seed * 21.2);
            let raw_jx = rand2d(noise_seed_x) * 2.0 - 1.0;
            let raw_jy = rand2d(noise_seed_y) * 2.0 - 1.0;
            let jitter_x = sign(raw_jx) * pow(abs(raw_jx), 0.5) * jitter_amp;
            let jitter_y = sign(raw_jy) * pow(abs(raw_jy), 0.5) * jitter_amp;
            final_uv = final_uv + vec2<f32>(jitter_x, jitter_y);
        }
        
        var tex_color = textureSample(t_wallpaper, s_wallpaper, final_uv);
        
        if (viewport.bg_effect_enabled > 0.5 && viewport.bg_effect_intensity > 0.0) {
            let global_darken = 1.0 - (viewport.bg_effect_intensity * 0.65);
            tex_color = vec4<f32>(tex_color.rgb * global_darken, tex_color.a);
            let gray = 0.299 * tex_color.r + 0.587 * tex_color.g + 0.114 * tex_color.b;
            let gray_color = vec3<f32>(gray, gray, gray);
            let desat_factor = clamp(viewport.bg_effect_intensity * 1.5, 0.0, 1.0);
            tex_color = vec4<f32>(mix(tex_color.rgb, gray_color, desat_factor), tex_color.a);
        }
        
        return tex_color;
    }

    if (dist > 1.0) { discard; }

    if (in.p_type == 1u) {
        let core = smoothstep(0.40, 0.35, dist);
        let white_hot = smoothstep(0.15, 0.07, dist);
        let glow = exp(-dist * dist * 4.5);
        let base_color = mix(in.color.rgb, vec3<f32>(1.0, 1.0, 1.0), white_hot);
        let alpha = max(core, glow * 0.55 * viewport.glow_factor) * in.color.a;
        return vec4<f32>(base_color * (1.0 + glow * 1.8), alpha);
    } else if (in.p_type == 3u) {
        let tex_uv = in.uv * 0.5 + vec2<f32>(0.5, 0.5);
        let tex_color = textureSample(t_logo, s_logo, tex_uv);
        let alpha = tex_color.a * in.color.a;
        let glow = exp(-dist * dist * 3.0) * 0.50 * viewport.glow_factor;
        let color_boosted = in.color.rgb * (1.0 + glow);
        return vec4<f32>(color_boosted, alpha);
    } else {
        let white_hot = smoothstep(0.2, 0.05, dist);
        let base_color = mix(in.color.rgb, vec3<f32>(1.0, 1.0, 1.0), white_hot * 0.85);
        let alpha = smoothstep(1.0, 0.3, dist) * in.color.a;
        return vec4<f32>(base_color * 1.4, alpha);
    }
}
"#;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuInstance {
    pub pos: [f32; 3],      
    pub color: [f32; 4],    
    pub size: f32,
    pub p_type: f32,        
    pub orbit_u: [f32; 3],
    pub orbit_v: [f32; 3],
    pub angle: f32,
}

impl GpuInstance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: 64, // 16 floats * 4 bytes
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x4 },
                wgpu::VertexAttribute { offset: 28, shader_location: 2, format: wgpu::VertexFormat::Float32 },
                wgpu::VertexAttribute { offset: 32, shader_location: 3, format: wgpu::VertexFormat::Float32 },
                wgpu::VertexAttribute { offset: 36, shader_location: 4, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 48, shader_location: 5, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 60, shader_location: 6, format: wgpu::VertexFormat::Float32 },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewportUniform {
    pub width: f32,
    pub height: f32,
    pub time: f32,
    pub load: f32,
    pub glow_factor: f32,
    pub bg_effect_enabled: f32,
    pub bg_effect_intensity: f32,
    pub core_x: f32,
    pub core_y: f32,
    pub _pad: [f32; 3],
}

pub struct ColorPalette {
    pub color_a: [f32; 4], pub color_b: [f32; 4], pub color_c: [f32; 4], pub color_d: [f32; 4],
}

pub fn get_palette(preset: crate::app::ColorPreset) -> ColorPalette {
    match preset {
        crate::app::ColorPreset::AtomicStarlink => ColorPalette {
            color_a: [0.0, 0.900, 1.0, 1.0], color_b: [0.0, 0.600, 1.0, 1.0], color_c: [0.0, 0.400, 0.900, 1.0], color_d: [0.0, 0.200, 0.700, 1.0],
        },
        crate::app::ColorPreset::Cyberpunk => ColorPalette {
            color_a: [0.850, 0.0, 1.0, 1.0], color_b: [1.0, 0.0, 0.800, 1.0], color_c: [0.600, 0.0, 0.900, 1.0], color_d: [0.400, 0.0, 0.700, 1.0],
        },
        crate::app::ColorPreset::AcidGreen => ColorPalette {
            color_a: [0.300, 1.0, 0.0, 1.0], color_b: [0.100, 0.900, 0.0, 1.0], color_c: [0.050, 0.700, 0.0, 1.0], color_d: [0.0, 0.500, 0.0, 1.0],
        },
        crate::app::ColorPreset::SolarFlame => ColorPalette {
            color_a: [1.0, 0.400, 0.0, 1.0], color_b: [1.0, 0.200, 0.0, 1.0], color_c: [0.900, 0.050, 0.0, 1.0], color_d: [0.700, 0.0, 0.0, 1.0],
        },
        crate::app::ColorPreset::DeepOcean => ColorPalette {
            color_a: [0.0, 1.0, 0.600, 1.0], color_b: [0.0, 0.800, 0.500, 1.0], color_c: [0.0, 0.600, 0.400, 1.0], color_d: [0.0, 0.400, 0.300, 1.0],
        },
        crate::app::ColorPreset::EmeraldPulse => ColorPalette {
            color_a: [0.0, 1.0, 0.4, 1.0], color_b: [0.0, 0.8, 0.2, 1.0], color_c: [0.0, 0.6, 0.1, 1.0], color_d: [0.0, 0.4, 0.0, 1.0],
        },
        crate::app::ColorPreset::CrimsonNova => ColorPalette {
            color_a: [1.0, 0.0, 0.2, 1.0], color_b: [0.8, 0.0, 0.1, 1.0], color_c: [0.6, 0.0, 0.05, 1.0], color_d: [0.4, 0.0, 0.0, 1.0],
        },
        crate::app::ColorPreset::VioletNight => ColorPalette {
            color_a: [0.5, 0.0, 1.0, 1.0], color_b: [0.4, 0.0, 0.8, 1.0], color_c: [0.3, 0.0, 0.6, 1.0], color_d: [0.2, 0.0, 0.4, 1.0],
        },
        crate::app::ColorPreset::AmberGhost => ColorPalette {
            color_a: [1.0, 0.8, 0.0, 1.0], color_b: [0.8, 0.6, 0.0, 1.0], color_c: [0.6, 0.4, 0.0, 1.0], color_d: [0.4, 0.2, 0.0, 1.0],
        },
        crate::app::ColorPreset::FrostByte => ColorPalette {
            color_a: [0.7, 0.9, 1.0, 1.0], color_b: [0.5, 0.8, 1.0, 1.0], color_c: [0.3, 0.7, 1.0, 1.0], color_d: [0.1, 0.6, 1.0, 1.0],
        },
    }
}

pub fn get_logo_color(preset: crate::app::ColorPreset, load_f: f32) -> [f32; 3] {
    let base_rgb = match preset {
        crate::app::ColorPreset::AtomicStarlink => [1.0, 0.6, 0.0],
        crate::app::ColorPreset::Cyberpunk => [0.0, 1.0, 0.8],
        crate::app::ColorPreset::AcidGreen => [1.0, 0.2, 0.8],
        crate::app::ColorPreset::SolarFlame => [0.0, 0.6, 1.0],
        crate::app::ColorPreset::DeepOcean => [1.0, 0.4, 0.0],
        crate::app::ColorPreset::EmeraldPulse => [1.0, 0.3, 0.3],
        crate::app::ColorPreset::CrimsonNova => [0.2, 1.0, 1.0],
        crate::app::ColorPreset::VioletNight => [0.8, 1.0, 0.0],
        crate::app::ColorPreset::AmberGhost => [0.0, 0.5, 1.0],
        crate::app::ColorPreset::FrostByte => [1.0, 0.5, 0.0],
    };
    if load_f <= 0.4 { return base_rgb; }
    let t = ((load_f - 0.4) / 0.6).clamp(0.0, 1.0);
    [
        base_rgb[0] + (1.0 - base_rgb[0]) * t * 0.5,
        base_rgb[1] + (1.0 - base_rgb[1]) * t * 0.5,
        base_rgb[2] + (1.0 - base_rgb[2]) * t * 0.5,
    ]
}

fn lerp_color(c1: [f32; 4], c2: [f32; 4], t: f32) -> [f32; 4] {
    [c1[0] + (c2[0] - c1[0]) * t, c1[1] + (c2[1] - c1[1]) * t, c1[2] + (c2[2] - c1[2]) * t, c1[3] + (c2[3] - c1[3]) * t]
}

pub struct Particle {
    pub x: f32, pub y: f32, pub z: f32, pub vx: f32, pub vy: f32, pub vz: f32,
    pub life: f32, pub decay: f32, pub max_life: f32, pub p_type: f32, pub color: [f32; 4],
}

pub struct CoreConfig { pub u: [f32; 3], pub v: [f32; 3], pub orbit_mult: f32 }

fn load_wallpaper() -> (u32, u32, Vec<u8>, bool) {
    if let Ok(app_data) = std::env::var("APPDATA") {
        let transcoded_path = std::path::Path::new(&app_data).join("Microsoft").join("Windows").join("Themes").join("TranscodedWallpaper");
        if transcoded_path.exists() {
            if let Ok(file) = std::fs::File::open(&transcoded_path) {
                let reader = std::io::BufReader::new(file);
                if let Ok(img_reader) = image::ImageReader::new(reader).with_guessed_format() {
                    if let Ok(img) = img_reader.decode() {
                        let rgba = img.to_rgba8(); let (w, h) = rgba.dimensions();
                        return (w, h, rgba.into_raw(), true);
                    }
                }
            }
        }
    }
    let mut buf = [0u16; 512];
    let path = unsafe {
        let res = windows_sys::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(windows_sys::Win32::UI::WindowsAndMessaging::SPI_GETDESKWALLPAPER, buf.len() as u32, buf.as_mut_ptr() as *mut std::ffi::c_void, 0);
        if res != 0 { let len = buf.iter().position(|&x| x == 0).unwrap_or(buf.len()); let path_str = String::from_utf16_lossy(&buf[..len]); if !path_str.trim().is_empty() { Some(path_str) } else { None } } else { None }
    };
    if let Some(ref p) = path {
        if let Ok(file) = std::fs::File::open(p) {
            let reader = std::io::BufReader::new(file);
            if let Ok(img_reader) = image::ImageReader::new(reader).with_guessed_format() {
                if let Ok(img) = img_reader.decode() {
                    let rgba = img.to_rgba8(); let (w, h) = rgba.dimensions();
                    return (w, h, rgba.into_raw(), false);
                }
            }
        }
    }
    (1, 1, vec![0, 0, 0, 255], false)
}

pub fn generate_star_logo() -> Vec<u8> {
    let mut buf = vec![0u8; 512 * 512 * 4];
    for y in 0..512 {
        let y_f = (y as f32 - 256.0) / 256.0;
        for x in 0..512 {
            let x_f = (x as f32 - 256.0) / 256.0;
            let d = (x_f*x_f + y_f*y_f).sqrt();
            let angle = y_f.atan2(x_f);
            let star_factor = 0.5 + 0.3 * (angle * 4.0).cos().abs();
            let val = (-((d - star_factor * 0.4) / 0.08).powi(2)).exp();
            let idx = (y * 512 + x) * 4;
            buf[idx] = 255; buf[idx+1] = 255; buf[idx+2] = 255;
            buf[idx+3] = (val * 255.0).clamp(0.0, 255.0) as u8;
        }
    }
    buf
}

pub struct SharedRenderResources {
    pub render_pipeline: wgpu::RenderPipeline, pub bind_group_layout: wgpu::BindGroupLayout,
    pub logo_texture: wgpu::Texture, pub logo_view: wgpu::TextureView, pub logo_sampler: wgpu::Sampler,
    pub wallpaper_texture: wgpu::Texture, pub wallpaper_view: wgpu::TextureView, pub wallpaper_sampler: wgpu::Sampler,
    pub is_transcoded: bool,
}

impl SharedRenderResources {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()) });
        let logo_rgba = generate_star_logo();
        let texture_size = wgpu::Extent3d { width: 512, height: 512, depth_or_array_layers: 1 };
        let logo_texture = device.create_texture(&wgpu::TextureDescriptor { label: None, size: texture_size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        queue.write_texture(wgpu::ImageCopyTexture { texture: &logo_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, &logo_rgba, wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * 512), rows_per_image: Some(512) }, texture_size);
        let logo_view = logo_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let logo_sampler = device.create_sampler(&wgpu::SamplerDescriptor { address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, address_mode_w: wgpu::AddressMode::ClampToEdge, mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        let (wp_w, wp_h, wp_rgba, is_transcoded) = load_wallpaper();
        let wp_size = wgpu::Extent3d { width: wp_w, height: wp_h, depth_or_array_layers: 1 };
        let wallpaper_texture = device.create_texture(&wgpu::TextureDescriptor { label: None, size: wp_size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        queue.write_texture(wgpu::ImageCopyTexture { texture: &wallpaper_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, &wp_rgba, wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * wp_w), rows_per_image: Some(wp_h) }, wp_size);
        let wallpaper_view = wallpaper_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let wallpaper_sampler = device.create_sampler(&wgpu::SamplerDescriptor { address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::FilterMode::Linear, anisotropy_clamp: 16, ..Default::default() });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ] });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&bind_group_layout], push_constant_ranges: &[] });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor { label: None, layout: Some(&pipeline_layout), vertex: wgpu::VertexState { module: &shader, entry_point: "vs_main", compilation_options: Default::default(), buffers: &[GpuInstance::desc()] }, fragment: Some(wgpu::FragmentState { module: &shader, entry_point: "fs_main", compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }), primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview: None, cache: None });
        Self { render_pipeline, bind_group_layout, logo_texture, logo_view, logo_sampler, wallpaper_texture, wallpaper_view, wallpaper_sampler, is_transcoded }
    }
}

pub struct Renderer {
    pub width: usize, pub height: usize, pub smoothed_load: f32, pub time: f32, pub config_glow: u8,
    pub particles: Vec<Particle>, pub core_angles: [f32; NUM_CORES], pub core_configs: Vec<CoreConfig>,
    pub surface: wgpu::Surface<'static>, pub surface_config: wgpu::SurfaceConfiguration,
    pub instance_buffer: wgpu::Buffer, pub uniform_buffer: wgpu::Buffer, pub uniform_bind_group: wgpu::BindGroup,
    pub shared_resources: std::sync::Arc<SharedRenderResources>,
    pub dcomp_device: windows::Win32::Graphics::DirectComposition::IDCompositionDevice,
    pub dcomp_target: windows::Win32::Graphics::DirectComposition::IDCompositionTarget,
    pub dcomp_visual: windows::Win32::Graphics::DirectComposition::IDCompositionVisual,
    pub win_w: f32, pub win_h: f32, pub monitor_rect: windows_sys::Win32::Foundation::RECT, pub monitor_total_rect: windows_sys::Win32::Foundation::RECT,
    pub wp_offset_scale: [f32; 4], pub core_flicker_timers: [f32; NUM_CORES], pub core_flicker_durations: [f32; NUM_CORES], pub core_flicker_targets: [[f32; 4]; NUM_CORES], pub core_colors: [[f32; 4]; NUM_CORES],
}

impl Renderer {
    pub fn new(instance: &wgpu::Instance, adapter: &wgpu::Adapter, device: &wgpu::Device, hwnd: HWND, hinstance: HINSTANCE, shared_resources: std::sync::Arc<SharedRenderResources>, render_w: usize, render_h: usize, win_w: f32, win_h: f32, monitor_rect: windows_sys::Win32::Foundation::RECT, monitor_total_rect: windows_sys::Win32::Foundation::RECT) -> Self {
        let mut rng = rand::thread_rng(); let mut core_configs = Vec::with_capacity(NUM_CORES);
        for _ in 0..NUM_CORES {
            let z: f32 = rng.gen_range(-1.0..1.0); let phi: f32 = rng.gen_range(0.0..2.0 * std::f32::consts::PI);
            let n = [(1.0 - z*z).sqrt() * phi.cos(), (1.0 - z*z).sqrt() * phi.sin(), z];
            let u_ref = if n[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
            let v = [n[1]*u_ref[2] - n[2]*u_ref[1], n[2]*u_ref[0] - n[0]*u_ref[2], n[0]*u_ref[1] - n[1]*u_ref[0]];
            let v_len = (v[0]*v[0] + v[1]*v[1] + v[2]*v[2]).sqrt();
            let v = [v[0]/v_len, v[1]/v_len, v[2]/v_len];
            let u = [v[1]*n[2] - v[2]*n[1], v[2]*n[0] - v[0]*n[2], v[0]*n[1] - v[1]*n[0]];
            core_configs.push(CoreConfig { u, v, orbit_mult: rng.gen_range(0.95..1.2) });
        }
        let mut core_angles = [0.0; NUM_CORES]; for i in 0..NUM_CORES { core_angles[i] = rng.gen_range(0.0..2.0 * std::f32::consts::PI); }
        let surface = unsafe {
            let hwnd_val = std::num::NonZeroIsize::new(hwnd as isize).unwrap();
            let mut win_handle = raw_window_handle::Win32WindowHandle::new(hwnd_val); win_handle.hinstance = std::num::NonZeroIsize::new(hinstance as isize);
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle { raw_display_handle: raw_window_handle::RawDisplayHandle::Windows(raw_window_handle::WindowsDisplayHandle::new()), raw_window_handle: raw_window_handle::RawWindowHandle::Win32(win_handle) })
        }.expect("Surface fail");
        let (dcomp_device, dcomp_target, dcomp_visual) = unsafe {
            use windows::Win32::Graphics::DirectComposition::*; use windows::Win32::Foundation::HWND;
            let dcomp_device: IDCompositionDevice = DCompositionCreateDevice(None).expect("DComp device fail");
            let dcomp_target = dcomp_device.CreateTargetForHwnd(HWND(hwnd as isize), true).expect("DComp target fail");
            let dcomp_visual = dcomp_device.CreateVisual().expect("DComp visual fail");
            (dcomp_device, dcomp_target, dcomp_visual)
        };
        let caps = surface.get_capabilities(adapter); let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
        let alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::PreMultiplied) { wgpu::CompositeAlphaMode::PreMultiplied } else if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) { wgpu::CompositeAlphaMode::Opaque } else { caps.alpha_modes[0] };
        let surface_config = wgpu::SurfaceConfiguration { usage: wgpu::TextureUsages::RENDER_ATTACHMENT, format, width: render_w as u32, height: render_h as u32, present_mode: wgpu::PresentMode::Fifo, alpha_mode, view_formats: vec![], desired_maximum_frame_latency: 2 };
        surface.configure(device, &surface_config);
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (MAX_INSTANCES * 64) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: 64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &shared_resources.bind_group_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&shared_resources.logo_view) }, wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&shared_resources.logo_sampler) }, wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&shared_resources.wallpaper_view) }, wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&shared_resources.wallpaper_sampler) }] });
        let mut renderer = Self { width: render_w, height: render_h, smoothed_load: 0.0, time: 0.0, config_glow: 1, particles: Vec::with_capacity(4000), core_angles, core_configs, surface, surface_config, instance_buffer, uniform_buffer, uniform_bind_group, shared_resources, dcomp_device, dcomp_target, dcomp_visual, win_w, win_h, monitor_rect, monitor_total_rect, wp_offset_scale: [0.0; 4], core_flicker_timers: [0.0; NUM_CORES], core_flicker_durations: [1.0; NUM_CORES], core_flicker_targets: [[0.0; 4]; NUM_CORES], core_colors: [[0.0; 4]; NUM_CORES] };
        renderer.update_wp_mapping(); renderer
    }

    fn update_wp_mapping(&mut self) {
        let (tex_w, tex_h) = (self.shared_resources.wallpaper_texture.width() as f64, self.shared_resources.wallpaper_texture.height() as f64);
        let win_w = self.win_w as f64; let win_h = self.win_h as f64;
        let screen_aspect = win_w / win_h; let tex_aspect = tex_w / tex_h;
        let v_bias = 0.44; 
        let (scale_x, scale_y, off_x, off_y) = if self.shared_resources.is_transcoded {
            if (tex_w - win_w).abs() < 1.0 && (tex_h - win_h).abs() < 1.0 { (1.0, 1.0, 0.0, 0.0) } else { if tex_aspect > screen_aspect { let s_x = (tex_h * screen_aspect) / tex_w; (s_x, 1.0, (1.0 - s_x) * 0.5, 0.0) } else { let s_y = (tex_w / screen_aspect) / tex_h; (1.0, s_y, 0.0, (1.0 - s_y) * v_bias) } }
        } else {
            if tex_aspect > screen_aspect { let s_x = (tex_h * screen_aspect) / tex_w; (s_x, 1.0, (1.0 - s_x) * 0.5, 0.0) } else { let s_y = (tex_w / screen_aspect) / tex_h; (1.0, s_y, 0.0, (1.0 - s_y) * v_bias) }
        };
        self.wp_offset_scale = [off_x as f32, off_y as f32, scale_x as f32, scale_y as f32];
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: usize, height: usize, monitor_rect: windows_sys::Win32::Foundation::RECT, monitor_total_rect: windows_sys::Win32::Foundation::RECT) {
        if width == 0 || height == 0 { return; }
        if self.width == width && self.height == height
            && self.monitor_rect.left == monitor_rect.left
            && self.monitor_rect.top == monitor_rect.top
            && self.monitor_rect.right == monitor_rect.right
            && self.monitor_rect.bottom == monitor_rect.bottom
            && self.monitor_total_rect.left == monitor_total_rect.left
            && self.monitor_total_rect.top == monitor_total_rect.top
            && self.monitor_total_rect.right == monitor_total_rect.right
            && self.monitor_total_rect.bottom == monitor_total_rect.bottom
        {
            return;
        }
        self.width = width; self.height = height; self.win_w = width as f32; self.win_h = height as f32;
        self.monitor_rect = monitor_rect; self.monitor_total_rect = monitor_total_rect;
        self.surface_config.width = width as u32; self.surface_config.height = height as u32;
        self.surface.configure(device, &self.surface_config); self.update_wp_mapping();
    }

    pub fn recreate_bind_group(&mut self, device: &wgpu::Device) {
        self.uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &self.shared_resources.bind_group_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: self.uniform_buffer.as_entire_binding() }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.shared_resources.logo_view) }, wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.shared_resources.logo_sampler) }, wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&self.shared_resources.wallpaper_view) }, wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&self.shared_resources.wallpaper_sampler) }] });
    }

    pub fn update(&mut self, delta_time: f32, overall_cpu: f32, color_preset: crate::app::ColorPreset) {
        self.time += delta_time; let mut rng = rand::thread_rng();
        self.smoothed_load += (overall_cpu - self.smoothed_load) * (delta_time * 5.0).min(1.0);
        let load_f = self.smoothed_load * 0.01;
        let speed = 0.3 + load_f.powf(1.2) * 12.0;
        for angle in &mut self.core_angles { *angle += speed * delta_time; }
        let scale = (self.height as f32 / 1080.0).max(0.2); let base_r = (60.0 + 90.0 * load_f) * scale;
        let palette = get_palette(color_preset);
        for i in 0..NUM_CORES {
            if self.core_flicker_timers[i] > 0.0 { self.core_flicker_timers[i] = (self.core_flicker_timers[i] - delta_time).max(0.0); }
            if self.core_flicker_timers[i] == 0.0 {
                let flicker_freq = 0.05 + 4.95 * load_f;
                if rng.gen::<f32>() < flicker_freq * delta_time {
                    let target = if load_f < 0.5 { palette.color_b } else if load_f < 0.8 { let t = (load_f - 0.5) / 0.3; if rng.gen::<f32>() < 0.4 * t { palette.color_c } else { palette.color_b } } else { let t = (load_f - 0.8) / 0.2; let p_b = 0.6 - 0.267 * t; let p_c = 0.4 - 0.067 * t; let r = rng.gen::<f32>(); if r < p_b { palette.color_b } else if r < p_b + p_c { palette.color_c } else { palette.color_d } };
                    let duration = rng.gen_range(0.25..0.45); self.core_flicker_timers[i] = duration; self.core_flicker_durations[i] = duration; self.core_flicker_targets[i] = target;
                }
            }
            self.core_colors[i] = if self.core_flicker_timers[i] > 0.0 { let t = self.core_flicker_timers[i] / self.core_flicker_durations[i]; let factor = 1.0 - (2.0 * t - 1.0).abs(); lerp_color(palette.color_a, self.core_flicker_targets[i], factor) } else { palette.color_a };
        }
        let logo_rgb = get_logo_color(color_preset, load_f); let spark_color = [logo_rgb[0], logo_rgb[1], logo_rgb[2], 1.0];
        for i in 0..NUM_CORES {
            let angle = self.core_angles[i]; let cfg = &self.core_configs[i]; let r_orbit = base_r * cfg.orbit_mult;
            let (sin_a, cos_a) = angle.sin_cos();
            let tangent = [-sin_a * cfg.u[0] + cos_a * cfg.v[0], -sin_a * cfg.u[1] + cos_a * cfg.v[1], -sin_a * cfg.u[2] + cos_a * cfg.v[2]];
            let mv = [tangent[0] * speed * r_orbit, tangent[1] * speed * r_orbit, tangent[2] * speed * r_orbit];
            let prev_angle = angle - speed * delta_time;
            let num_steps = if load_f > 0.5 { 4 } else if load_f > 0.2 { 2 } else { 1 };
            for step in 0..num_steps {
                let t_step = step as f32 / num_steps as f32; let ia = prev_angle + (angle - prev_angle) * t_step; let (s_ia, c_ia) = ia.sin_cos();
                let px = r_orbit * (c_ia * cfg.u[0] + s_ia * cfg.v[0]); let py = r_orbit * (c_ia * cfg.u[1] + s_ia * cfg.v[1]); let pz = r_orbit * (c_ia * cfg.u[2] + s_ia * cfg.v[2]);
                let life = 0.25 + load_f * 0.25; let jitter = 50.0 * scale * (1.0 + load_f);
                self.particles.push(Particle { x: px, y: py, z: pz, vx: -mv[0] * 0.3 + rng.gen_range(-jitter..jitter), vy: -mv[1] * 0.3 + rng.gen_range(-jitter..jitter), vz: -mv[2] * 0.3 + rng.gen_range(-jitter..jitter), life, decay: 1.0, max_life: life, p_type: 0.0, color: self.core_colors[i] });
            }
            if load_f > 0.5 && rng.gen::<f32>() < (load_f - 0.5) * 3.0 {
                let (s_a, c_a) = angle.sin_cos();
                let px = r_orbit * (c_a * cfg.u[0] + s_a * cfg.v[0]); let py = r_orbit * (c_a * cfg.u[1] + s_a * cfg.v[1]); let pz = r_orbit * (c_a * cfg.u[2] + s_a * cfg.v[2]);
                let theta: f32 = rng.gen_range(0.0..2.0 * std::f32::consts::PI); let z_val: f32 = rng.gen_range(-1.0..1.0); let r_xy = (1.0f32 - z_val * z_val).sqrt(); let (s_t, c_t) = theta.sin_cos(); let dir = [r_xy * c_t, r_xy * s_t, z_val];
                let (mut cx, mut cy, mut cz) = (px, py, pz); let step_len = 10.0 * scale; let jit = 4.0 * scale; let life = rng.gen_range(0.05..0.15); 
                for _ in 0..5 { cx += dir[0] * step_len + rng.gen_range(-jit..jit); cy += dir[1] * step_len + rng.gen_range(-jit..jit); cz += dir[2] * step_len + rng.gen_range(-jit..jit); self.particles.push(Particle { x: cx, y: cy, z: cz, vx: 0.0, vy: 0.0, vz: 0.0, life, decay: 1.0, max_life: life, p_type: 1.0, color: spark_color }); }
            }
        }
        let drag = 0.5 * delta_time; let chaos = 15.0 * scale * (1.0 + load_f); let p_decay_v = (1.0 - delta_time).max(0.0);
        for p in &mut self.particles {
            if p.p_type == 0.0 { p.vx = (p.vx + rng.gen_range(-chaos..chaos) * delta_time) * (1.0 - drag); p.vy = (p.vy + rng.gen_range(-chaos..chaos) * delta_time) * (1.0 - drag); p.vz = (p.vz + rng.gen_range(-chaos..chaos) * delta_time) * (1.0 - drag); } else { p.vx *= p_decay_v; p.vy *= p_decay_v; p.vz *= p_decay_v; }
            p.x += p.vx * delta_time; p.y += p.vy * delta_time; p.z += p.vz * delta_time; p.life -= p.decay * delta_time;
        }
        self.particles.retain(|p| p.life > 0.0); if self.particles.len() > 4000 { self.particles.truncate(4000); }
    }

    pub fn draw(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue, color_preset: crate::app::ColorPreset, bg_effect_enabled: bool) {
        let scale = (self.height as f32 / 1080.0).max(0.2); let (cx, cy) = (self.width as f32 * 0.5, self.height as f32 * 0.5);
        let load_f = self.smoothed_load * 0.01; let base_r = (60.0 + 90.0 * load_f) * scale;
        let mut instances = Vec::with_capacity((NUM_CORES + self.particles.len() + 2).min(MAX_INSTANCES)); let mut rng = rand::thread_rng();
        let bg_effect_intensity = if self.smoothed_load > 60.0 { ((self.smoothed_load - 60.0) * 0.025).min(1.0) } else { 0.0 };
        instances.push(GpuInstance { pos: [cx, cy, -499.0], color: self.wp_offset_scale, size: 1.0, p_type: 4.0, orbit_u: [0.0; 3], orbit_v: [0.0; 3], angle: 0.0 });
        for p in &self.particles {
            let ratio = (p.life / p.max_life).clamp(0.0, 1.0); let size = if p.p_type == 0.0 { 7.5 * scale * ratio } else { 1.8 * scale * ratio }; let alpha = if p.p_type == 0.0 { ratio * 0.7 } else { ratio * 0.9 };
            let (mut jx, mut jy, mut jz) = (0.0, 0.0, 0.0); if p.p_type == 1.0 { let jit = scale; jx = rng.gen_range(-jit..jit); jy = rng.gen_range(-jit..jit); jz = rng.gen_range(-jit..jit); }
            instances.push(GpuInstance { pos: [cx + p.x + jx, cy + p.y + jy, p.z + jz], color: [p.color[0], p.color[1], p.color[2], alpha], size, p_type: 2.0, orbit_u: [0.0; 3], orbit_v: [0.0; 3], angle: 0.0 });
        }
        if load_f > 0.4 {
            let center_f = ((load_f - 0.4) / 0.6).clamp(0.0, 1.0); let logo_rgb = get_logo_color(color_preset, load_f); let center_size = base_r * 1.20 * (0.2 + 0.8 * center_f.sqrt()) + rng.gen_range(-2.6..2.6) * center_f * scale; let jit = 2.73 * center_f * scale;
            instances.push(GpuInstance { pos: [cx + rng.gen_range(-jit..jit), cy + rng.gen_range(-jit..jit), 0.0], color: [logo_rgb[0], logo_rgb[1], logo_rgb[2], center_f.sqrt() * 0.98], size: center_size, p_type: 3.0, orbit_u: [0.0; 3], orbit_v: [0.0; 3], angle: 0.0 });
        }
        for i in 0..NUM_CORES {
            let angle = self.core_angles[i]; let cfg = &self.core_configs[i]; let r = base_r * cfg.orbit_mult;
            instances.push(GpuInstance { 
                pos: [cx, cy, r], // Pass center and radius
                color: self.core_colors[i], size: 9.0 * scale, p_type: 1.0, 
                orbit_u: cfg.u, orbit_v: cfg.v, angle 
            });
        }
        instances.sort_unstable_by(|a, b| a.pos[2].partial_cmp(&b.pos[2]).unwrap_or(std::cmp::Ordering::Equal));
        let count = instances.len().min(MAX_INSTANCES); if count == 0 { return; }
        let glow_factor = match self.config_glow { 0 => 0.15, 1 => 1.0, 2 => 2.5, _ => 1.0 };
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances[..count]));
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&ViewportUniform { width: self.width as f32, height: self.height as f32, time: self.time, load: self.smoothed_load, glow_factor, bg_effect_enabled: if bg_effect_enabled { 1.0 } else { 0.0 }, bg_effect_intensity, core_x: cx, core_y: cy, _pad: [0.0; 3] }));
        let frame = match self.surface.get_current_texture() { Ok(f) => f, Err(_) => return };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = _device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { label: None, color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })], depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None });
            rp.set_pipeline(&self.shared_resources.render_pipeline); rp.set_bind_group(0, &self.uniform_bind_group, &[]); rp.set_vertex_buffer(0, self.instance_buffer.slice(..)); rp.draw(0..6, 0..count as u32);
        }
        queue.submit(std::iter::once(encoder.finish())); frame.present();
    }
}
