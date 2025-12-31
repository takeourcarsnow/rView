use anyhow::{anyhow, Result};
use wgpu::util::DeviceExt;
use crate::image_loader::ImageAdjustments;
use image::DynamicImage;

pub struct GpuProcessor {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl GpuProcessor {
    pub fn new() -> Result<Self> {
        // Create instance / adapter / device synchronously
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        // Use the primary adapter (power preference default)
        let adapter = pollster::block_on(async {
            instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
        })
        .ok_or_else(|| anyhow!("No suitable GPU adapter found"))?;

        let (device, queue) = pollster::block_on(async {
            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("image_viewer_device"),
                        required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                        required_limits: wgpu::Limits {
                            max_compute_workgroup_size_x: 256,
                            ..wgpu::Limits::downlevel_defaults()
                        },
                        memory_hints: wgpu::MemoryHints::Performance,
                    },
                    None,
                )
                .await
        })?;

        Ok(Self { device, queue })
    }

    /// Apply a small set of adjustments to the provided RGBA8 image using a compute shader.
    /// Returns an RGBA8 Vec<u8> on success.
    pub fn apply_adjustments(&self, image: &DynamicImage, adj: &ImageAdjustments) -> Result<Vec<u8>> {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        let pixel_count = (width * height) as usize;
        let input_data = rgba.into_raw(); // Vec<u8> (RGBA8)

        // Pack to u32 per pixel for storage buffer (little-endian RGBA8 -> u32)
        let mut packed = Vec::with_capacity(pixel_count);
        for i in 0..pixel_count {
            let base = i * 4;
            let r = input_data[base] as u32;
            let g = input_data[base + 1] as u32;
            let b = input_data[base + 2] as u32;
            let a = input_data[base + 3] as u32;
            let packed_pixel = (a << 24) | (b << 16) | (g << 8) | r;
            packed.push(packed_pixel);
        }

        // Shader WGSL: comprehensive image adjustments
        let shader_source = r#"
struct Params {
    exposure : f32,
    brightness : f32,
    contrast : f32,
    saturation : f32,
    highlights : f32,
    shadows : f32,
    temperature : f32,
    tint : f32,
    blacks : f32,
    whites : f32,
    sharpening : f32,
    width : u32,
    height : u32,
};

@group(0) @binding(0) var<storage, read> input_pixels: array<u32>;
@group(0) @binding(1) var<storage, read_write> output_pixels: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;

fn unpack_u32(px: u32) -> vec4<f32> {
    let r = f32(px & 0xFFu) / 255.0;
    let g = f32((px >> 8) & 0xFFu) / 255.0;
    let b = f32((px >> 16) & 0xFFu) / 255.0;
    let a = f32((px >> 24) & 0xFFu) / 255.0;
    return vec4<f32>(r, g, b, a);
}

fn pack_u32(c: vec4<f32>) -> u32 {
    let r = u32(clamp(c.x * 255.0, 0.0, 255.0));
    let g = u32(clamp(c.y * 255.0, 0.0, 255.0));
    let b = u32(clamp(c.z * 255.0, 0.0, 255.0));
    let a = u32(clamp(c.w * 255.0, 0.0, 255.0));
    return (a << 24) | (b << 16) | (g << 8) | r;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    let idx = GlobalInvocationID.x;
    if (idx >= params.width * params.height) {
        return;
    }

    var c = unpack_u32(input_pixels[idx]); // rgba in [0,1]
    var rgb = c.xyz;

    // Exposure (stops): multiply
    let exposure_mult = pow(2.0, params.exposure);
    rgb = rgb * exposure_mult;

    // Blacks adjustment (lift shadows)
    rgb = rgb + vec3<f32>(params.blacks * 0.1);

    // Whites adjustment (reduce highlights)
    let white_factor = 1.0 - params.whites * 0.1;
    rgb = rgb * white_factor;

    // Shadows adjustment (gamma-like curve for shadows)
    let shadow_lift = params.shadows * 0.2;
    rgb = mix(rgb, pow(rgb, vec3<f32>(1.0 - shadow_lift)), step(0.0, -shadow_lift));

    // Highlights adjustment (compress highlights)
    let highlight_compress = params.highlights * 0.3;
    let luminance = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
    let highlight_mask = smoothstep(0.5, 1.0, luminance);
    rgb = mix(rgb, rgb * (1.0 - highlight_compress * highlight_mask), step(0.0, highlight_compress));

    // Brightness (mapped -100..100 -> -1..1)
    rgb = rgb + vec3<f32>(params.brightness / 100.0);

    // Contrast: params.contrast is multiplier (0..inf)
    rgb = ((rgb - vec3<f32>(0.5)) * params.contrast + vec3<f32>(0.5));

    // Temperature adjustment (blue/yellow shift)
    if (params.temperature > 0.0) {
        // Warmer: reduce blue, increase red/yellow
        rgb.x = rgb.x + params.temperature * 0.1; // more red
        rgb.y = rgb.y + params.temperature * 0.05; // more green
        rgb.z = rgb.z - params.temperature * 0.08; // less blue
    } else {
        // Cooler: increase blue, reduce red/yellow
        rgb.x = rgb.x + params.temperature * 0.08; // less red
        rgb.y = rgb.y + params.temperature * 0.05; // less green
        rgb.z = rgb.z - params.temperature * 0.1; // more blue
    }

    // Tint adjustment (green/magenta shift)
    if (params.tint > 0.0) {
        // More magenta: increase red and blue
        rgb.x = rgb.x + params.tint * 0.05;
        rgb.z = rgb.z + params.tint * 0.05;
    } else {
        // More green: increase green
        rgb.y = rgb.y - params.tint * 0.08;
    }

    // Saturation: convert to luma, mix
    let gray = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
    rgb = mix(vec3<f32>(gray), rgb, params.saturation);

    // Basic sharpening (unsharp mask approximation)
    if (params.sharpening > 0.0) {
        // This is a simplified sharpening - in a real implementation,
        // we'd need access to neighboring pixels
        let sharpened = rgb + (rgb - vec3<f32>(gray)) * params.sharpening * 0.5;
        rgb = mix(rgb, sharpened, params.sharpening);
    }

    // Clamp
    rgb = clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    c = vec4<f32>(rgb, c.w);

    output_pixels[idx] = pack_u32(vec4<f32>(c.xyz, c.w));
}
"#;

        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("adjustments_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create buffers
        let input_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("input_pixels"),
            contents: bytemuck::cast_slice(&packed),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let output_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output_pixels"),
            size: (packed.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Params uniform buffer
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            exposure: f32,
            brightness: f32,
            contrast: f32,
            saturation: f32,
            highlights: f32,
            shadows: f32,
            temperature: f32,
            tint: f32,
            blacks: f32,
            whites: f32,
            sharpening: f32,
            width: u32,
            height: u32,
        }

        let params = Params {
            exposure: adj.exposure,
            brightness: adj.brightness,
            contrast: adj.contrast,
            saturation: adj.saturation,
            highlights: adj.highlights,
            shadows: adj.shadows,
            temperature: adj.temperature,
            tint: adj.tint,
            blacks: adj.blacks,
            whites: adj.whites,
            sharpening: adj.sharpening,
            width,
            height,
        };

        let params_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Bind group layout
        let bind_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind_layout"),
            entries: &[
                // input pixels (read-only storage buffer)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // output pixels
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // params
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind_group"),
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
            ],
        });

        // Pipeline
        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // Command encoder
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("command_encoder"),
        });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("compute_pass"), timestamp_writes: None });
            cpass.set_pipeline(&compute_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            let groups = ((pixel_count as u32) + 255) / 256;
            cpass.dispatch_workgroups(groups, 1, 1);
        }

        // Copy output buffer to a staging buffer for readback
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: (packed.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging, 0, (packed.len() * std::mem::size_of::<u32>()) as u64);

        // Submit
        self.queue.submit(Some(encoder.finish()));

        // Read back
        let buffer_slice = staging.slice(..);
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        // Wait for device to finish
        self.device.poll(wgpu::Maintain::Wait);
        let ok = pollster::block_on(rx.receive()).unwrap_or(Err(wgpu::BufferAsyncError));
        ok.map_err(|_| anyhow!("GPU map failed"))?;

        let data = buffer_slice.get_mapped_range().to_vec();

        // Unmap staging buffer
        staging.unmap();

        // Interpret as u32 pixels and unpack to u8 RGBA
        let mut out = Vec::with_capacity(pixel_count * 4);
        for chunk in data.chunks_exact(4) {
            let v = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let r = (v & 0xFF) as u8;
            let g = ((v >> 8) & 0xFF) as u8;
            let b = ((v >> 16) & 0xFF) as u8;
            let a = ((v >> 24) & 0xFF) as u8;
            out.push(r);
            out.push(g);
            out.push(b);
            out.push(a);
        }

        Ok(out)
    }
}
