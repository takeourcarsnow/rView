use super::types::{GpuProcessor, AdjustmentParams};
use crate::image_loader::ImageAdjustments;
use anyhow::Result;
use image::DynamicImage;
use tokio::sync::oneshot;
use wgpu::util::DeviceExt;

impl GpuProcessor {
    fn create_adjustment_params(adj: &ImageAdjustments, width: u32, height: u32) -> AdjustmentParams {
        let film = &adj.film;
        AdjustmentParams {
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
            film_enabled: if film.enabled { 1 } else { 0 },
            film_is_bw: if film.is_bw { 1 } else { 0 },
            tone_curve_shadows: film.tone_curve_shadows,
            tone_curve_midtones: film.tone_curve_midtones,
            tone_curve_highlights: film.tone_curve_highlights,
            s_curve_strength: film.s_curve_strength,
            grain_amount: film.grain_amount,
            grain_size: film.grain_size,
            grain_roughness: film.grain_roughness,
            halation_amount: film.halation_amount,
            vignette_amount: film.vignette_amount,
            vignette_softness: film.vignette_softness,
            latitude: film.latitude,
            red_gamma: film.red_gamma,
            green_gamma: film.green_gamma,
            blue_gamma: film.blue_gamma,
            black_point: film.black_point,
            white_point: film.white_point,
        }
    }

    /// Apply adjustments using texture-based processing for better performance
    pub async fn apply_adjustments_texture(&self, image: &DynamicImage, adj: &ImageAdjustments) -> Result<DynamicImage> {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        // Create input texture
        let input_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("input_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Create output texture
        let output_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("output_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Upload input data
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &input_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        // Create parameter buffer
        let params = Self::create_adjustment_params(adj, width, height);
        let param_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("adjustment_params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("adjustment_bind_group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&input_texture.create_view(&wgpu::TextureViewDescriptor::default())),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&output_texture.create_view(&wgpu::TextureViewDescriptor::default())),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: param_buffer.as_entire_binding(),
                },
            ],
        });

        // Execute compute pass
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("adjustment_encoder"),
        });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("adjustment_pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.adjustment_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups((width + 15) / 16, (height + 15) / 16, 1);
        }

        // Download result
        let bytes_per_row = ((4 * width + wgpu::COPY_BYTES_PER_ROW_ALIGNMENT - 1) / wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output_buffer"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        // Read back result
        let buffer_slice = output_buffer.slice(..);
        let (tx, rx) = oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });

        self.device.poll(wgpu::Maintain::Wait);
        rx.await??;

        // Process the data and drop the view before unmapping
        let result = {
            let data = buffer_slice.get_mapped_range();
            let bytes_per_row = ((4 * width + wgpu::COPY_BYTES_PER_ROW_ALIGNMENT - 1) / wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
            let mut valid_data = Vec::with_capacity((width * height * 4) as usize);
            for row in 0..height {
                let start = (row * bytes_per_row) as usize;
                let end = start + (width * 4) as usize;
                valid_data.extend_from_slice(&data[start..end]);
            }
            image::ImageBuffer::from_raw(width, height, valid_data)
                .ok_or_else(|| anyhow::anyhow!("Failed to create result image"))?
        };

        output_buffer.unmap();

        Ok(DynamicImage::ImageRgba8(result))
    }

    // Legacy buffer-based method for backward compatibility
    pub fn apply_adjustments(&self, image: &DynamicImage, adj: &ImageAdjustments) -> Result<Vec<u8>> {
        self.apply_adjustments_legacy(image, adj)
    }

    // Keep the legacy implementation for now
    fn apply_adjustments_legacy(&self, image: &DynamicImage, adj: &ImageAdjustments) -> Result<Vec<u8>> {
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

        // Shader WGSL: comprehensive image adjustments with film emulation
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
    // Film emulation parameters
    film_enabled : u32,
    film_is_bw : u32,
    tone_curve_shadows : f32,
    tone_curve_midtones : f32,
    tone_curve_highlights : f32,
    s_curve_strength : f32,
    grain_amount : f32,
    grain_size : f32,
    grain_roughness : f32,
    halation_amount : f32,
    halation_radius : f32,
    halation_color_r : f32,
    halation_color_g : f32,
    halation_color_b : f32,
    red_in_green : f32,
    red_in_blue : f32,
    green_in_red : f32,
    green_in_blue : f32,
    blue_in_red : f32,
    blue_in_green : f32,
    red_gamma : f32,
    green_gamma : f32,
    blue_gamma : f32,
    black_point : f32,
    white_point : f32,
    shadow_tint_r : f32,
    shadow_tint_g : f32,
    shadow_tint_b : f32,
    highlight_tint_r : f32,
    highlight_tint_g : f32,
    highlight_tint_b : f32,
    vignette_amount : f32,
    vignette_softness : f32,
    latitude : f32,
    _padding : f32,
};

@group(0) @binding(0) var<storage, read> input_pixels: array<u32>;
@group(0) @binding(1) var<storage, read_write> output_pixels: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var<uniform> offset: u32;

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

// Hash function for pseudo-random noise
fn hash(x: u32, y: u32, seed: u32) -> f32 {
    var h = seed;
    h = h ^ x;
    h = h * 0x517cc1b7u;
    h = h ^ y;
    h = h * 0x517cc1b7u;
    h = h ^ (h >> 16u);
    return f32(h) / f32(0xFFFFFFFFu) * 2.0 - 1.0;
}

// S-curve for film characteristic curve
fn apply_s_curve(x: f32, strength: f32) -> f32 {
    let xc = clamp(x, 0.0, 1.0);
    let midpoint = 0.5;
    let steepness = 1.0 + strength * 3.0;
    
    let sigmoid = 1.0 / (1.0 + exp(-steepness * (xc - midpoint)));
    let min_sig = 1.0 / (1.0 + exp(steepness * midpoint));
    let max_sig = 1.0 / (1.0 + exp(-steepness * (1.0 - midpoint)));
    
    let normalized = (sigmoid - min_sig) / (max_sig - min_sig);
    return xc * (1.0 - strength) + normalized * strength;
}

// Tone curve for shadows/midtones/highlights
fn apply_tone_curve(x: f32, shadows: f32, midtones: f32, highlights: f32) -> f32 {
    let xc = clamp(x, 0.0, 1.0);
    
    let shadow_weight = clamp(1.0 - xc * 3.0, 0.0, 1.0);
    let highlight_weight = clamp((xc - 0.66) * 3.0, 0.0, 1.0);
    let midtone_weight = 1.0 - shadow_weight - highlight_weight;
    
    let adjustment = shadows * shadow_weight * 0.15 
                   + midtones * midtone_weight * 0.1
                   + highlights * highlight_weight * 0.15;
    
    return clamp(xc + adjustment, 0.0, 1.0);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    let idx = GlobalInvocationID.x + offset;
    if (idx >= params.width * params.height) {
        return;
    }

    let px = idx % params.width;
    let py = idx / params.width;
    
    var c = unpack_u32(input_pixels[idx]); // rgba in [0,1]
    var rgb = c.xyz;
    
    let film_enabled = params.film_enabled != 0u;
    let film_is_bw = params.film_is_bw != 0u;

    // ============ FILM EMULATION ============
    if (film_enabled) {
        // B&W conversion for monochrome films
        if (film_is_bw) {
            let luminance = 0.30 * rgb.x + 0.59 * rgb.y + 0.11 * rgb.z;
            rgb = vec3<f32>(luminance);
        }
        
        // Color channel crossover/crosstalk
        if (!film_is_bw) {
            let orig = rgb;
            rgb.x = orig.x + orig.y * params.green_in_red + orig.z * params.blue_in_red;
            rgb.y = orig.y + orig.x * params.red_in_green + orig.z * params.blue_in_green;
            rgb.z = orig.z + orig.x * params.red_in_blue + orig.y * params.green_in_blue;
        }
        
        // Per-channel gamma
        rgb.x = pow(max(rgb.x, 0.0), params.red_gamma);
        rgb.y = pow(max(rgb.y, 0.0), params.green_gamma);
        rgb.z = pow(max(rgb.z, 0.0), params.blue_gamma);
        
        // Film latitude (dynamic range compression)
        if (params.latitude > 0.0) {
            let lat = params.latitude * 0.5;
            rgb = rgb / (vec3<f32>(1.0) + rgb * lat);
            let comp = 1.0 + lat * 0.5;
            rgb = rgb * comp;
        }
        
        // S-curve
        if (params.s_curve_strength > 0.0) {
            rgb.x = apply_s_curve(rgb.x, params.s_curve_strength);
            rgb.y = apply_s_curve(rgb.y, params.s_curve_strength);
            rgb.z = apply_s_curve(rgb.z, params.s_curve_strength);
        }
        
        // Tone curve
        rgb.x = apply_tone_curve(rgb.x, params.tone_curve_shadows, params.tone_curve_midtones, params.tone_curve_highlights);
        rgb.y = apply_tone_curve(rgb.y, params.tone_curve_shadows, params.tone_curve_midtones, params.tone_curve_highlights);
        rgb.z = apply_tone_curve(rgb.z, params.tone_curve_shadows, params.tone_curve_midtones, params.tone_curve_highlights);
        
        // Black/white point
        let bp = params.black_point;
        let wp = params.white_point;
        let range = wp - bp;
        if (range > 0.01) {
            rgb = vec3<f32>(bp) + rgb * range;
        }
        
        // Shadow/highlight tinting
        let luminance = 0.299 * rgb.x + 0.587 * rgb.y + 0.114 * rgb.z;
        let shadow_amount = clamp(1.0 - luminance * 2.0, 0.0, 1.0);
        let highlight_amount = clamp((luminance - 0.5) * 2.0, 0.0, 1.0);
        
        rgb.x = rgb.x + params.shadow_tint_r * shadow_amount + params.highlight_tint_r * highlight_amount;
        rgb.y = rgb.y + params.shadow_tint_g * shadow_amount + params.highlight_tint_g * highlight_amount;
        rgb.z = rgb.z + params.shadow_tint_b * shadow_amount + params.highlight_tint_b * highlight_amount;
    }

    // Convert to 0-255 range for standard adjustments
    rgb = rgb * 255.0;
    
    // ============ STANDARD ADJUSTMENTS ============
    
    // Apply exposure
    let exposure_mult = pow(2.0, params.exposure);
    rgb = rgb * exposure_mult;
    
    // Blacks adjustment (lift shadows)
    rgb = rgb + vec3<f32>(params.blacks * 25.5);
    
    // Whites adjustment (reduce highlights)
    rgb = rgb * (1.0 - params.whites * 0.1);
    
    // Shadows adjustment (gamma-like curve for shadows)
    if (params.shadows < 0.0) {
        let gamma = 1.0 - params.shadows;
        rgb = pow(max(rgb / 255.0, vec3<f32>(0.0)), vec3<f32>(gamma)) * 255.0;
    }
    
    // Highlights adjustment (compress highlights)
    if (params.highlights > 0.0) {
        let luminance = 0.299 * rgb.x + 0.587 * rgb.y + 0.114 * rgb.z;
        let highlight_mask = clamp((luminance - 127.5) / 127.5, 0.0, 1.0);
        let compress = 1.0 - params.highlights * highlight_mask;
        rgb = rgb * compress;
    }
    
    // Brightness
    rgb = rgb + vec3<f32>(params.brightness * 2.55);
    
    // Contrast
    rgb = ((rgb / 255.0 - vec3<f32>(0.5)) * params.contrast + vec3<f32>(0.5)) * 255.0;
    
    // Temperature
    rgb.x = rgb.x + params.temperature * 25.5;
    rgb.z = rgb.z - params.temperature * 15.3;
    
    // Tint
    rgb.x = rgb.x + params.tint * 12.75;
    rgb.y = rgb.y - params.tint * 20.4;
    rgb.z = rgb.z + params.tint * 12.75;
    
    // Saturation (skip for B&W film)
    if (!film_enabled || !film_is_bw) {
        let gray = 0.299 * rgb.x + 0.587 * rgb.y + 0.114 * rgb.z;
        rgb = vec3<f32>(gray) + (rgb - vec3<f32>(gray)) * params.saturation;
    }
    
    // Basic sharpening
    if (params.sharpening > 0.0) {
        let gray = 0.299 * rgb.x + 0.587 * rgb.y + 0.114 * rgb.z;
        let sharpened = rgb + (rgb - vec3<f32>(gray)) * params.sharpening;
        rgb = rgb + (sharpened - rgb) * params.sharpening;
    }
    
    // Convert back to 0-1 range
    rgb = rgb / 255.0;
    
    // ============ FILM POST-PROCESSING ============
    if (film_enabled) {
        // Vignette
        if (params.vignette_amount > 0.0) {
            let center = vec2<f32>(f32(params.width) / 2.0, f32(params.height) / 2.0);
            let max_dist = length(center);
            let pos = vec2<f32>(f32(px), f32(py));
            let dist = length(pos - center) / max_dist;
            let vignette = 1.0 - params.vignette_amount * pow(dist / params.vignette_softness, 2.0);
            rgb = rgb * clamp(vignette, 0.0, 1.0);
        }
        
        // Film grain
        if (params.grain_amount > 0.0) {
            let scale = 1.0 / params.grain_size;
            let sx = u32(f32(px) * scale);
            let sy = u32(f32(py) * scale);
            
            var grain = hash(sx, sy, 12345u);
            if (params.grain_roughness > 0.0) {
                let grain2 = hash(sx + 1u, sy + 1u, 54321u);
                grain = grain * (1.0 - params.grain_roughness * 0.5) + grain2 * params.grain_roughness * 0.5;
            }
            
            let lum = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
            let grain_mask = 4.0 * lum * (1.0 - lum);
            let grain_strength = params.grain_amount * 255.0 * 0.15 * grain_mask;
            
            rgb.x = rgb.x + grain * grain_strength / 255.0;
            rgb.y = rgb.y + grain * grain_strength / 255.0;
            rgb.z = rgb.z + grain * grain_strength / 255.0;
        }
        
        // Halation
        if (params.halation_amount > 0.0) {
            let luminance = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
            let halation_mask = clamp((luminance - 0.7) / 0.3, 0.0, 1.0);
            let halation_strength = params.halation_amount * halation_mask * 30.0 / 255.0;
            rgb.x = rgb.x + params.halation_color_r * halation_strength;
            rgb.y = rgb.y + params.halation_color_g * halation_strength;
            rgb.z = rgb.z + params.halation_color_b * halation_strength;
        }
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

        // Params uniform buffer with film emulation
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
            // Film emulation parameters
            film_enabled: u32,
            film_is_bw: u32,
            tone_curve_shadows: f32,
            tone_curve_midtones: f32,
            tone_curve_highlights: f32,
            s_curve_strength: f32,
            grain_amount: f32,
            grain_size: f32,
            grain_roughness: f32,
            halation_amount: f32,
            halation_radius: f32,
            halation_color_r: f32,
            halation_color_g: f32,
            halation_color_b: f32,
            red_in_green: f32,
            red_in_blue: f32,
            green_in_red: f32,
            green_in_blue: f32,
            blue_in_red: f32,
            blue_in_green: f32,
            red_gamma: f32,
            green_gamma: f32,
            blue_gamma: f32,
            black_point: f32,
            white_point: f32,
            shadow_tint_r: f32,
            shadow_tint_g: f32,
            shadow_tint_b: f32,
            highlight_tint_r: f32,
            highlight_tint_g: f32,
            highlight_tint_b: f32,
            vignette_amount: f32,
            vignette_softness: f32,
            latitude: f32,
            _padding: f32,
        }

        let film = &adj.film;
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
            // Film emulation
            film_enabled: if film.enabled { 1 } else { 0 },
            film_is_bw: if film.is_bw { 1 } else { 0 },
            tone_curve_shadows: film.tone_curve_shadows,
            tone_curve_midtones: film.tone_curve_midtones,
            tone_curve_highlights: film.tone_curve_highlights,
            s_curve_strength: film.s_curve_strength,
            grain_amount: film.grain_amount,
            grain_size: film.grain_size,
            grain_roughness: film.grain_roughness,
            halation_amount: film.halation_amount,
            halation_radius: film.halation_radius,
            halation_color_r: film.halation_color[0],
            halation_color_g: film.halation_color[1],
            halation_color_b: film.halation_color[2],
            red_in_green: film.red_in_green,
            red_in_blue: film.red_in_blue,
            green_in_red: film.green_in_red,
            green_in_blue: film.green_in_blue,
            blue_in_red: film.blue_in_red,
            blue_in_green: film.blue_in_green,
            red_gamma: film.red_gamma,
            green_gamma: film.green_gamma,
            blue_gamma: film.blue_gamma,
            black_point: film.black_point,
            white_point: film.white_point,
            shadow_tint_r: film.shadow_tint[0],
            shadow_tint_g: film.shadow_tint[1],
            shadow_tint_b: film.shadow_tint[2],
            highlight_tint_r: film.highlight_tint[0],
            highlight_tint_g: film.highlight_tint[1],
            highlight_tint_b: film.highlight_tint[2],
            vignette_amount: film.vignette_amount,
            vignette_softness: film.vignette_softness,
            latitude: film.latitude,
            _padding: 0.0,
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
                // offset
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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

        const MAX_WORKGROUPS: u32 = 65535;
        let workgroup_size = 256;
        let mut offset = 0u32;
        while offset < pixel_count as u32 {
            let remaining_pixels = pixel_count as u32 - offset;
            let max_pixels_this_dispatch = MAX_WORKGROUPS * workgroup_size;
            let pixels_this_dispatch = remaining_pixels.min(max_pixels_this_dispatch);
            let groups = (pixels_this_dispatch + workgroup_size - 1) / workgroup_size;

            let offset_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("offset"),
                contents: bytemuck::bytes_of(&offset),
                usage: wgpu::BufferUsages::UNIFORM,
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
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: offset_buf.as_entire_binding(),
                    },
                ],
            });

            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("compute_pass"), timestamp_writes: None });
                cpass.set_pipeline(&compute_pipeline);
                cpass.set_bind_group(0, &bind_group, &[]);
                cpass.dispatch_workgroups(groups, 1, 1);
            }

            offset += pixels_this_dispatch;
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
        ok.map_err(|_| anyhow::anyhow!("GPU map failed"))?;

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