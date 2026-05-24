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
    
    var offset_px = uv;
    if (u32(input.p_type) == 4u) {
        // Quad for wallpaper should always fill the viewport
        offset_px = uv * vec2<f32>(viewport.width * 0.5, viewport.height * 0.5);
    } else {
        offset_px = uv * input.size;
    }
    let pixel_pos = input.pos.xy + offset_px;

    let ndc_x = (pixel_pos.x / viewport.width) * 2.0 - 1.0;
    let ndc_y = (pixel_pos.y / viewport.height) * -2.0 + 1.0;

    var out : VertexOutput;
    let normalized_z = (input.pos.z + 500.0) / 1000.0;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, normalized_z, 1.0);
    out.uv = uv;
    out.color = input.color;
    out.p_type = u32(input.p_type);
    return out;
}

// Pseudo-random number generator for pixel-level noise jitter
fn rand(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.uv);
    
    if (in.p_type == 4u) {
        let tex_uv = (in.uv * 0.5 + vec2<f32>(0.5, 0.5)) * in.color.zw + in.color.xy;
        var final_uv = tex_uv;
        
        if (viewport.bg_effect_enabled > 0.5 && viewport.bg_effect_intensity > 0.0) {
            // 1. Shockwave Ripple (radial discrete pulse, fast propagation)
            let pixel_center = vec2<f32>(viewport.core_x, viewport.core_y);
            let to_pixel = in.clip_position.xy - pixel_center;
            let dist_px = length(to_pixel);
            
            // Period of the pulse (e.g. 1.5 seconds)
            let period = 1.5; 
            let pulse_time = fract(viewport.time / period) * period;
            
            // Fast propagation speed (tripled)
            let speed = 2700.0 + 2100.0 * viewport.bg_effect_intensity; 
            let radius = speed * pulse_time;
            
            let dist_to_wavefront = abs(dist_px - radius);
            let thickness = 100.0; // thickness of the wave packet
            
            // Radial direction vector for refraction
            var radial_dir = vec2<f32>(0.0, 0.0);
            if (dist_px > 0.1) {
                radial_dir = to_pixel / dist_px;
            }
            
            // Shallow wave packet
            let wave = sin((dist_px - radius) * 0.10); 
            let envelope = smoothstep(thickness, 0.0, dist_to_wavefront);
            let fade = smoothstep(period, period * 0.7, pulse_time) * smoothstep(0.0, 0.15, pulse_time);
            
            let ripple_displacement = wave * 0.0022 * viewport.bg_effect_intensity * envelope * fade;
            
            // Apply radial ripple displacement
            final_uv = final_uv + radial_dir * ripple_displacement;
            
            // 2. Spiritual Pressure Jitter (pixel-level noise jitter)
            let jitter_amp = 0.0085 * viewport.bg_effect_intensity;
            let noise_seed = in.clip_position.xy + vec2<f32>(viewport.time * 23.45, viewport.time * 56.78);
            let rand_val = rand(noise_seed);
            let noise_jitter = (rand_val * 2.0 - 1.0) * jitter_amp;
            
            // Jitter is applied vertically to represent high pressure noise
            final_uv.y = final_uv.y + noise_jitter;
        }
        
        var tex_color = textureSample(t_wallpaper, s_wallpaper, final_uv);
        
        // Apply color desaturation from Spiritual Pressure
        if (viewport.bg_effect_enabled > 0.5 && viewport.bg_effect_intensity > 0.0) {
            let gray = 0.299 * tex_color.r + 0.587 * tex_color.g + 0.114 * tex_color.b;
            let gray_color = vec3<f32>(gray, gray, gray);
            let desat_factor = clamp(viewport.bg_effect_intensity * 1.5, 0.0, 1.0);
            tex_color = vec4<f32>(mix(tex_color.rgb, gray_color, desat_factor), tex_color.a);
        }
        
        return tex_color;
    }

    if (dist > 1.0) { discard; }

    if (in.p_type == 1u) {
        // Bright core with glowing white-hot center
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
        // Particles and tails: white-hot center and boosted glow
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
}

impl GpuInstance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: 36,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x4 },
                wgpu::VertexAttribute { offset: 28, shader_location: 2, format: wgpu::VertexFormat::Float32 },
                wgpu::VertexAttribute { offset: 32, shader_location: 3, format: wgpu::VertexFormat::Float32 },
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
    pub color_a: [f32; 4], // base
    pub color_b: [f32; 4], // low load flicker
    pub color_c: [f32; 4], // med load flicker
    pub color_d: [f32; 4], // high load flicker
}

pub fn get_palette(preset: crate::app::ColorPreset) -> ColorPalette {
    match preset {
        crate::app::ColorPreset::AtomicStarlink => ColorPalette {
            // Pure Cyan/Blue theme
            color_a: [0.0, 0.900, 1.0, 1.0],   
            color_b: [0.0, 0.600, 1.0, 1.0],   
            color_c: [0.0, 0.400, 0.900, 1.0],   
            color_d: [0.0, 0.200, 0.700, 1.0],   
        },
        crate::app::ColorPreset::Cyberpunk => ColorPalette {
            // Neon Violet/Pink theme
            color_a: [0.850, 0.0, 1.0, 1.0],   
            color_b: [1.0, 0.0, 0.800, 1.0],   
            color_c: [0.600, 0.0, 0.900, 1.0],   
            color_d: [0.400, 0.0, 0.700, 1.0],   
        },
        crate::app::ColorPreset::AcidGreen => ColorPalette {
            // Toxic Lime/Green theme
            color_a: [0.300, 1.0, 0.0, 1.0], 
            color_b: [0.100, 0.900, 0.0, 1.0],   
            color_c: [0.050, 0.700, 0.0, 1.0],     
            color_d: [0.0, 0.500, 0.0, 1.0], 
        },
        crate::app::ColorPreset::SolarFlame => ColorPalette {
            // Fire Orange/Red theme
            color_a: [1.0, 0.400, 0.0, 1.0],   
            color_b: [1.0, 0.200, 0.0, 1.0],   
            color_c: [0.900, 0.050, 0.0, 1.0],     
            color_d: [0.700, 0.0, 0.0, 1.0],   
        },
        crate::app::ColorPreset::DeepOcean => ColorPalette {
            // Deep Teal/Turquoise theme
            color_a: [0.0, 1.0, 0.600, 1.0], 
            color_b: [0.0, 0.800, 0.500, 1.0], 
            color_c: [0.0, 0.600, 0.400, 1.0], 
            color_d: [0.0, 0.400, 0.300, 1.0], 
        },
    }
}

pub fn get_logo_color(preset: crate::app::ColorPreset, load_f: f32) -> [f32; 3] {
    if load_f <= 0.4 {
        return [1.0, 1.0, 1.0];
    }
    let t = ((load_f - 0.4) / 0.6).clamp(0.0, 1.0);
    let overheat_color = match preset {
        crate::app::ColorPreset::AtomicStarlink => [0.3, 0.7, 1.0],
        crate::app::ColorPreset::Cyberpunk => [1.0, 0.3, 0.9],
        crate::app::ColorPreset::AcidGreen => [0.6, 1.0, 0.3],
        crate::app::ColorPreset::SolarFlame => [1.0, 0.7, 0.1],
        crate::app::ColorPreset::DeepOcean => [0.2, 1.0, 0.8],
    };
    [
        1.0 + (overheat_color[0] - 1.0) * t,
        1.0 + (overheat_color[1] - 1.0) * t,
        1.0 + (overheat_color[2] - 1.0) * t,
    ]
}

fn lerp_color(c1: [f32; 4], c2: [f32; 4], t: f32) -> [f32; 4] {
    [
        c1[0] + (c2[0] - c1[0]) * t,
        c1[1] + (c2[1] - c1[1]) * t,
        c1[2] + (c2[2] - c1[2]) * t,
        c1[3] + (c2[3] - c1[3]) * t,
    ]
}

pub struct Particle {
    pub x: f32, pub y: f32, pub z: f32,
    pub vx: f32, pub vy: f32, pub vz: f32,
    pub life: f32, pub decay: f32,
    pub max_life: f32,
    pub p_type: f32, // 0: tail, 1: spark
    pub color: [f32; 4],
}

pub struct CoreConfig { pub u: [f32; 3], pub v: [f32; 3], pub orbit_mult: f32 }

fn load_wallpaper() -> (u32, u32, Vec<u8>) {
    let mut buf = [0u16; 512];
    let path = unsafe {
        let res = windows_sys::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
            windows_sys::Win32::UI::WindowsAndMessaging::SPI_GETDESKWALLPAPER,
            buf.len() as u32,
            buf.as_mut_ptr() as *mut std::ffi::c_void,
            0,
        );
        if res != 0 {
            let len = buf.iter().position(|&x| x == 0).unwrap_or(buf.len());
            let path_str = String::from_utf16_lossy(&buf[..len]);
            if !path_str.trim().is_empty() {
                Some(path_str)
            } else {
                None
            }
        } else {
            None
        }
    };

    if let Some(ref p) = path {
        crate::app::log_msg(&format!("Loading desktop wallpaper: {}", p));
        match std::fs::File::open(p) {
            Ok(file) => {
                let reader = std::io::BufReader::new(file);
                match image::ImageReader::new(reader).with_guessed_format() {
                    Ok(img_reader) => match img_reader.decode() {
                        Ok(img) => {
                            let w_orig = img.width();
                            let h_orig = img.height();
                            crate::app::log_msg(&format!("Loading ORIGINAL wallpaper: {}x{}. (No compression)", w_orig, h_orig));
                            
                            let rgba = img.to_rgba8();
                            let (w, h) = rgba.dimensions();
                            return (w, h, rgba.into_raw());
                        }
                        Err(e) => {
                            crate::app::log_msg(&format!("Failed to decode wallpaper image: {:?}, falling back to black", e));
                        }
                    },
                    Err(e) => {
                        crate::app::log_msg(&format!("Failed to guess wallpaper format: {:?}, falling back to black", e));
                    }
                }
            }
            Err(e) => {
                crate::app::log_msg(&format!("Failed to open wallpaper file: {:?}, falling back to black", e));
            }
        }
    } else {
        crate::app::log_msg("Failed to get wallpaper path from system");
    }

    (1, 1, vec![0, 0, 0, 255])
}

pub struct SharedRenderResources {
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    pub logo_texture: wgpu::Texture,
    pub logo_view: wgpu::TextureView,
    pub logo_sampler: wgpu::Sampler,
    #[allow(dead_code)]
    pub wallpaper_texture: wgpu::Texture,
    pub wallpaper_view: wgpu::TextureView,
    pub wallpaper_sampler: wgpu::Sampler,
}

impl SharedRenderResources {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shared Shader Module"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        // Generate the default Star logo procedurally to prevent logo reset issues
        let mut buf = vec![0u8; 512 * 512 * 4];
        for y in 0..512 {
            for x in 0..512 {
                let dx = (x as f32 - 256.0) / 256.0;
                let dy = (y as f32 - 256.0) / 256.0;
                let d = (dx*dx + dy*dy).sqrt();
                let angle = dy.atan2(dx);
                let star_factor = 0.5 + 0.3 * (angle * 4.0).cos().abs();
                let val = (-((d - star_factor * 0.4) / 0.08).powi(2)).exp();
                let alpha = (val * 255.0).clamp(0.0, 255.0) as u8;
                let idx = (y * 512 + x) * 4;
                buf[idx] = 255;
                buf[idx+1] = 255;
                buf[idx+2] = 255;
                buf[idx+3] = alpha;
            }
        }
        let rgba_data = buf;

        let texture_size = wgpu::Extent3d {
            width: 512,
            height: 512,
            depth_or_array_layers: 1,
        };
        let logo_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("logo_texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &logo_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * 512),
                rows_per_image: Some(512),
            },
            texture_size,
        );

        let logo_view = logo_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let logo_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Load wallpaper
        let (wp_w, wp_h, wp_rgba) = load_wallpaper();

        let wp_size = wgpu::Extent3d {
            width: wp_w,
            height: wp_h,
            depth_or_array_layers: 1,
        };
        let wallpaper_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wallpaper_texture"),
            size: wp_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &wallpaper_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &wp_rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * wp_w),
                rows_per_image: Some(wp_h),
            },
            wp_size,
        );
        let wallpaper_view = wallpaper_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let wallpaper_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 16, 
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shared Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shared Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shared Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[GpuInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            render_pipeline,
            bind_group_layout,
            logo_texture,
            logo_view,
            logo_sampler,
            wallpaper_texture,
            wallpaper_view,
            wallpaper_sampler,
        }
    }
}

pub struct Renderer {
    pub width: usize, pub height: usize,
    pub smoothed_load: f32, pub time: f32,
    pub config_glow: u8,
    pub particles: Vec<Particle>,
    pub core_angles: [f32; NUM_CORES],
    pub core_configs: Vec<CoreConfig>,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub instance_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub shared_resources: std::sync::Arc<SharedRenderResources>,
    // Add DComp objects to maintain lifetime
    #[allow(dead_code)]
    pub dcomp_device: windows::Win32::Graphics::DirectComposition::IDCompositionDevice,
    #[allow(dead_code)]
    pub dcomp_target: windows::Win32::Graphics::DirectComposition::IDCompositionTarget,
    #[allow(dead_code)]
    pub dcomp_visual: windows::Win32::Graphics::DirectComposition::IDCompositionVisual,
    // Multi-monitor mapping
    pub vs_x: f32,
    pub vs_y: f32,
    pub vs_w: f32,
    pub vs_h: f32,
    pub win_w: f32,
    pub win_h: f32,

    // Per-core twinkling state
    pub core_flicker_timers: [f32; NUM_CORES],
    pub core_flicker_durations: [f32; NUM_CORES],
    pub core_flicker_targets: [[f32; 4]; NUM_CORES],
    pub core_colors: [[f32; 4]; NUM_CORES],
}

impl Renderer {
    pub fn new(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        hwnd: HWND,
        hinstance: HINSTANCE,
        shared_resources: std::sync::Arc<SharedRenderResources>,
        render_w: usize,
        render_h: usize,
        vs_x: f32,
        vs_y: f32,
        vs_w: f32,
        vs_h: f32,
        win_w: f32,
        win_h: f32,
    ) -> Self {
        let mut rng = rand::thread_rng();
        let mut core_configs = Vec::with_capacity(NUM_CORES);
        for _ in 0..NUM_CORES {
            let z: f32 = rng.gen_range(-1.0..1.0);
            let phi: f32 = rng.gen_range(0.0..2.0 * std::f32::consts::PI);
            let n = [(1.0 - z*z).sqrt() * phi.cos(), (1.0 - z*z).sqrt() * phi.sin(), z];
            let u_ref = if n[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
            let v = [n[1]*u_ref[2] - n[2]*u_ref[1], n[2]*u_ref[0] - n[0]*u_ref[2], n[0]*u_ref[1] - n[1]*u_ref[0]];
            let v_len = (v[0]*v[0] + v[1]*v[1] + v[2]*v[2]).sqrt();
            let v = [v[0]/v_len, v[1]/v_len, v[2]/v_len];
            let u = [v[1]*n[2] - v[2]*n[1], v[2]*n[0] - v[0]*n[2], v[0]*n[1] - v[1]*n[0]];
            core_configs.push(CoreConfig { u, v, orbit_mult: rng.gen_range(0.95..1.2) });
        }

        let mut core_angles = [0.0; NUM_CORES];
        for i in 0..NUM_CORES {
            core_angles[i] = rng.gen_range(0.0..2.0 * std::f32::consts::PI);
        }

        let surface = unsafe {
            let hwnd_val = std::num::NonZeroIsize::new(hwnd as isize).unwrap();
            let mut win_handle = raw_window_handle::Win32WindowHandle::new(hwnd_val);
            win_handle.hinstance = std::num::NonZeroIsize::new(hinstance as isize);
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle { 
                raw_display_handle: raw_window_handle::RawDisplayHandle::Windows(raw_window_handle::WindowsDisplayHandle::new()),
                raw_window_handle: raw_window_handle::RawWindowHandle::Win32(win_handle),
            })
        }.expect("Surface fail");

        // DComp objects
        let (dcomp_device, dcomp_target, dcomp_visual) = unsafe {
            use windows::Win32::Graphics::DirectComposition::*;
            use windows::Win32::Foundation::HWND;
            let dcomp_device: IDCompositionDevice = DCompositionCreateDevice(None).expect("DComp device fail");
            let dcomp_target = dcomp_device.CreateTargetForHwnd(HWND(hwnd as isize), true).expect("DComp target fail");
            let dcomp_visual = dcomp_device.CreateVisual().expect("DComp visual fail");
            (dcomp_device, dcomp_target, dcomp_visual)
        };

        let caps = surface.get_capabilities(adapter);
        let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
        
        let alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::PreMultiplied) {
            wgpu::CompositeAlphaMode::PreMultiplied
        } else if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            caps.alpha_modes[0]
        };
        
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, format, width: render_w as u32, height: render_h as u32,
            present_mode: wgpu::PresentMode::Fifo, alpha_mode,
            view_formats: vec![], desired_maximum_frame_latency: 2,
        };
        surface.configure(device, &surface_config);

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (MAX_INSTANCES * 36) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: 64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &shared_resources.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shared_resources.logo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shared_resources.logo_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&shared_resources.wallpaper_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&shared_resources.wallpaper_sampler),
                },
            ],
        });

        let core_flicker_timers = [0.0; NUM_CORES];
        let core_flicker_durations = [1.0; NUM_CORES];
        let core_flicker_targets = [[0.0; 4]; NUM_CORES];
        let core_colors = [[0.0; 4]; NUM_CORES];

        Self {
            width: render_w, height: render_h, smoothed_load: 0.0, time: 0.0, config_glow: 1, 
            particles: Vec::new(), core_angles, core_configs,
            surface, surface_config, instance_buffer, uniform_buffer, uniform_bind_group,
            shared_resources,
            dcomp_device,
            dcomp_target,
            dcomp_visual,
            vs_x,
            vs_y,
            vs_w,
            vs_h,
            win_w,
            win_h,
            core_flicker_timers,
            core_flicker_durations,
            core_flicker_targets,
            core_colors,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: usize, height: usize) {
        if width == 0 || height == 0 { return; }
        if self.width == width && self.height == height { return; }
        self.width = width; self.height = height;
        self.surface_config.width = width as u32;
        self.surface_config.height = height as u32;
        self.surface.configure(device, &self.surface_config);
    }

    pub fn recreate_bind_group(&mut self, device: &wgpu::Device) {
        self.uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.shared_resources.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.shared_resources.logo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.shared_resources.logo_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.shared_resources.wallpaper_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.shared_resources.wallpaper_sampler),
                },
            ],
        });
    }

    pub fn update(&mut self, delta_time: f32, overall_cpu: f32, _core_usages: &[f32], color_preset: crate::app::ColorPreset) {
        self.time += delta_time;
        let mut rng = rand::thread_rng();
        let lerp = (delta_time * 2.0).min(1.0);
        self.smoothed_load = self.smoothed_load + (overall_cpu - self.smoothed_load) * lerp;
        let load_f = self.smoothed_load / 100.0;
 
        let speed = 0.3 + load_f.powf(1.5) * 8.0;
        for angle in &mut self.core_angles { *angle += speed * delta_time; }
 
        let scale = (self.height as f32 / 1080.0).max(0.2);
        let orbit_mult = {
            let state = crate::app::STATE.lock().unwrap();
            state.core_orbit_r
        };
        let base_r = (60.0 + (150.0 - 60.0) * load_f) * scale * orbit_mult;
        
        let palette = get_palette(color_preset);

        // Update core colors and twinkling states
        for i in 0..NUM_CORES {
            if self.core_flicker_timers[i] > 0.0 {
                self.core_flicker_timers[i] -= delta_time;
                if self.core_flicker_timers[i] < 0.0 {
                    self.core_flicker_timers[i] = 0.0;
                }
            }

            if self.core_flicker_timers[i] == 0.0 {
                let flicker_freq = 0.05 + 4.95 * load_f;
                if rng.gen::<f32>() < flicker_freq * delta_time {
                    let target = if load_f < 0.5 {
                        palette.color_b
                    } else if load_f < 0.8 {
                        let t = (load_f - 0.5) / 0.3;
                        let p_c = 0.4 * t;
                        if rng.gen::<f32>() < p_c {
                            palette.color_c
                        } else {
                            palette.color_b
                        }
                    } else {
                        let t = (load_f - 0.8) / 0.2;
                        let p_b = 0.6 - 0.267 * t;
                        let p_c = 0.4 - 0.067 * t;
                        let r = rng.gen::<f32>();
                        if r < p_b {
                            palette.color_b
                        } else if r < p_b + p_c {
                            palette.color_c
                        } else {
                            palette.color_d
                        }
                    };
                    let duration = rng.gen_range(0.25..0.45);
                    self.core_flicker_timers[i] = duration;
                    self.core_flicker_durations[i] = duration;
                    self.core_flicker_targets[i] = target;
                }
            }

            self.core_colors[i] = if self.core_flicker_timers[i] > 0.0 {
                let t = self.core_flicker_timers[i] / self.core_flicker_durations[i];
                let factor = 1.0 - (2.0 * t - 1.0).abs();
                lerp_color(palette.color_a, self.core_flicker_targets[i], factor)
            } else {
                palette.color_a
            };
        }
        
        // Spawn persistent tails and load-conditioned spontaneous sparks
        for i in 0..NUM_CORES {
            let angle = self.core_angles[i];
            let prev_angle = angle - speed * delta_time;
            let cfg = &self.core_configs[i];
            let r_orbit = base_r * cfg.orbit_mult;
            let current_core_color = self.core_colors[i];
            
            // 1. TAIL: Interpolated flame trail matching core's twinkling color
            let num_steps = if load_f > 0.5 { 4 } else if load_f > 0.2 { 2 } else { 1 };
            for step in 0..num_steps {
                let t_step = step as f32 / num_steps as f32;
                let interpolated_angle = prev_angle + (angle - prev_angle) * t_step;
                let (sin_ia, cos_ia) = interpolated_angle.sin_cos();
                let px = r_orbit * (cos_ia * cfg.u[0] + sin_ia * cfg.v[0]);
                let py = r_orbit * (cos_ia * cfg.u[1] + sin_ia * cfg.v[1]);
                let pz = r_orbit * (cos_ia * cfg.u[2] + sin_ia * cfg.v[2]);

                let initial_life = 0.12 + load_f * 0.18;
                self.particles.push(Particle {
                    x: px, y: py, z: pz,
                    vx: 0.0, vy: 0.0, vz: 0.0,
                    life: initial_life,
                    decay: 1.0,
                    max_life: initial_life,
                    p_type: 0.0,
                    color: current_core_color,
                });
            }

            // 2. SPARK: Spontaneous static electric discharges matching core's twinkling color
            if load_f > 0.5 {
                let spark_chance = (load_f - 0.5) * 1.5;
                if rng.gen::<f32>() < spark_chance {
                    let (sin_a, cos_a) = angle.sin_cos();
                    let px = r_orbit * (cos_a * cfg.u[0] + sin_a * cfg.v[0]);
                    let py = r_orbit * (cos_a * cfg.u[1] + sin_a * cfg.v[1]);
                    let pz = r_orbit * (cos_a * cfg.u[2] + sin_a * cfg.v[2]);

                    let theta: f32 = rng.gen_range(0.0..2.0 * std::f32::consts::PI);
                    let z_val: f32 = rng.gen_range(-1.0..1.0);
                    let r_xy = (1.0f32 - z_val * z_val).sqrt();
                    let (sin_theta, cos_theta) = theta.sin_cos();
                    let dir = [r_xy * cos_theta, r_xy * sin_theta, z_val];
                    
                    let mut curr_x = px;
                    let mut curr_y = py;
                    let mut curr_z = pz;
                    let step_len = 10.0 * scale; 
                    let jit = 4.0 * scale; 
                    let life = rng.gen_range(0.05..0.15); 
                    
                    for _ in 0..3 {
                        curr_x += dir[0] * step_len + rng.gen_range(-jit..jit);
                        curr_y += dir[1] * step_len + rng.gen_range(-jit..jit);
                        curr_z += dir[2] * step_len + rng.gen_range(-jit..jit);
                        
                        self.particles.push(Particle {
                            x: curr_x, y: curr_y, z: curr_z,
                            vx: 0.0, vy: 0.0, vz: 0.0, 
                            life,
                            decay: 1.0,
                            max_life: life,
                            p_type: 1.0,
                            color: current_core_color,
                        });
                    }
                }
            }
        }

        // Particle updates
        for p in &mut self.particles {
            if p.p_type == 0.0 {
                // TAIL: Swaying flame physics (periodic horizontal wave rising upward)
                let age = p.max_life - p.life;
                let sway_speed = 14.0;
                let sway_amount = 160.0 * scale * (p.life / p.max_life); 
                let phase = self.time * sway_speed - age * 25.0;
                
                p.vx = phase.sin() * sway_amount + rng.gen_range(-15.0..15.0) * scale;
                p.vy = -200.0 * scale * (0.4 + 0.6 * (p.life / p.max_life)); 
                p.vz = phase.cos() * (sway_amount * 0.3); 
            } else {
                p.vx *= (1.0 - 1.0 * delta_time).max(0.0);
                p.vy *= (1.0 - 1.0 * delta_time).max(0.0);
                p.vz *= (1.0 - 1.0 * delta_time).max(0.0);
            }
            p.x += p.vx * delta_time;
            p.y += p.vy * delta_time;
            p.z += p.vz * delta_time;
            p.life -= p.decay * delta_time;
        }
        self.particles.retain(|p| p.life > 0.0);
        if self.particles.len() > 4000 { self.particles.truncate(4000); }
    }

    pub fn draw(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue, color_preset: crate::app::ColorPreset, bg_effect_enabled: bool) {
        let scale = (self.height as f32 / 1080.0).max(0.2);
        
        let (core_size_mult, core_orbit_mult, sat_size_mult, core_position) = {
            let state = crate::app::STATE.lock().unwrap();
            (state.core_size, state.core_orbit_r, state.satellite_size, state.core_position)
        };
        
        let (cx, cy) = match core_position {
            crate::app::CorePosition::TopRight => {
                let padding = 200.0 * scale;
                (self.width as f32 - padding, padding)
            }
            crate::app::CorePosition::Center => {
                (self.width as f32 * 0.5, self.height as f32 * 0.5)
            }
        };
        let load_f = self.smoothed_load / 100.0;
        let base_r = (60.0 + (150.0 - 60.0) * load_f) * scale * core_orbit_mult;
        
        let mut instances = Vec::with_capacity(NUM_CORES + self.particles.len() + 2);
        let mut rng = rand::thread_rng();

        // Background effect intensity: scales linearly from 0.0 (at 60% CPU) to 1.0 (at 100% CPU)
        let bg_effect_intensity = if self.smoothed_load > 60.0 {
            ((self.smoothed_load - 60.0) / 40.0).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Full-screen wallpaper background quad mapping
        let (tex_w, tex_h) = {
            let res = &self.shared_resources;
            (res.wallpaper_texture.width() as f32, res.wallpaper_texture.height() as f32)
        };
        
        let screen_aspect = self.vs_w / self.vs_h;
        let tex_aspect = tex_w / tex_h;
        
        let (cover_scale_x, cover_scale_y, cover_offset_x, cover_offset_y) = if tex_aspect > screen_aspect {
            let visible_w = tex_h * screen_aspect;
            let scale_x = visible_w / tex_w;
            (scale_x, 1.0, (1.0 - scale_x) * 0.5, 0.0)
        } else {
            let visible_h = tex_w / screen_aspect;
            let scale_y = visible_h / tex_h;
            (1.0, scale_y, 0.0, (1.0 - scale_y) * 0.5)
        };

        let monitor_uv_scale_x = (self.win_w / self.vs_w) * cover_scale_x;
        let monitor_uv_scale_y = (self.win_h / self.vs_h) * cover_scale_y;
        let monitor_uv_offset_x = cover_offset_x + (self.vs_x / self.vs_w) * cover_scale_x;
        let monitor_uv_offset_y = cover_offset_y + (self.vs_y / self.vs_h) * cover_scale_y;

        instances.push(GpuInstance {
            pos: [self.width as f32 * 0.5, self.height as f32 * 0.5, -499.0],
            color: [monitor_uv_offset_x, monitor_uv_offset_y, monitor_uv_scale_x, monitor_uv_scale_y],
            size: 1.0,
            p_type: 4.0,
        });

        // Add particles
        for p in &self.particles {
            let ratio = (p.life / p.max_life).clamp(0.0, 1.0);
            let size = if p.p_type == 0.0 {
                3.5 * scale * ratio
            } else {
                1.8 * scale * ratio
            };
            let alpha = if p.p_type == 0.0 {
                p.life * 0.6
            } else {
                ratio * 0.9
            };
            let (jit_x, jit_y, jit_z) = if p.p_type == 1.0 {
                let jit = 1.0 * scale;
                (rng.gen_range(-jit..jit), rng.gen_range(-jit..jit), rng.gen_range(-jit..jit))
            } else {
                (0.0, 0.0, 0.0)
            };
            instances.push(GpuInstance { 
                pos: [cx + p.x + jit_x, cy + p.y + jit_y, p.z + jit_z], 
                color: [p.color[0], p.color[1], p.color[2], alpha], 
                size, 
                p_type: 2.0 
            });
        }
        
        // 3. Central glowing star logo (active under load, starting at 40% CPU load, with overheating color transition)
        if load_f > 0.4 {
            let center_f = ((load_f - 0.4) / 0.6).clamp(0.0, 1.0);
            let center_alpha = center_f.sqrt() * 0.98;
            let logo_rgb = get_logo_color(color_preset, load_f);
            
            let size_vibe = rng.gen_range(-2.6..2.6) * center_f * scale;
            let center_size = base_r * (core_size_mult * 0.60) * (0.2 + 0.8 * center_f.sqrt()) + size_vibe;
            
            let pos_jit = 2.73 * center_f * scale;
            let jit_x = rng.gen_range(-pos_jit..pos_jit);
            let jit_y = rng.gen_range(-pos_jit..pos_jit);
            
            instances.push(GpuInstance {
                pos: [cx + jit_x, cy + jit_y, 0.0],
                color: [logo_rgb[0], logo_rgb[1], logo_rgb[2], center_alpha],
                size: center_size,
                p_type: 3.0,
            });
        }
 
        // 4. Orbiting core satellites (with individual twinkling colors)
        for i in 0..NUM_CORES {
            let angle = self.core_angles[i]; let cfg = &self.core_configs[i];
            let r = base_r * cfg.orbit_mult;
            let (sin_a, cos_a) = angle.sin_cos();
            instances.push(GpuInstance {
                pos: [
                    cx + r * (cos_a * cfg.u[0] + sin_a * cfg.v[0]), 
                    cy + r * (cos_a * cfg.u[1] + sin_a * cfg.v[1]), 
                    r * (cos_a * cfg.u[2] + sin_a * cfg.v[2])
                ],
                color: self.core_colors[i],
                size: 9.0 * scale * sat_size_mult,
                p_type: 1.0,
            });
        }

        instances.sort_by(|a, b| a.pos[2].partial_cmp(&b.pos[2]).unwrap_or(std::cmp::Ordering::Equal));
        let count = instances.len().min(MAX_INSTANCES);
        if count == 0 { return; }

        let glow_factor = match self.config_glow {
            0 => 0.15,
            1 => 1.0,
            2 => 2.5,
            _ => 1.0,
        };
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances[..count]));
        
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&ViewportUniform {
            width: self.width as f32,
            height: self.height as f32,
            time: self.time,
            load: self.smoothed_load,
            glow_factor,
            bg_effect_enabled: if bg_effect_enabled { 1.0 } else { 0.0 },
            bg_effect_intensity,
            core_x: cx,
            core_y: cy,
            _pad: [0.0; 3],
        }));

        let frame = match self.surface.get_current_texture() { Ok(f) => f, Err(_) => return };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = _device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None, color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None,
            });
            rp.set_pipeline(&self.shared_resources.render_pipeline);
            rp.set_bind_group(0, &self.uniform_bind_group, &[]);
            rp.set_vertex_buffer(0, self.instance_buffer.slice(..));
            rp.draw(0..6, 0..count as u32);
        }
        queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}
