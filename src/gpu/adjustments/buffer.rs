use crate::gpu::types::GpuProcessor;
use crate::image_loader::ImageAdjustments;
use anyhow::Result;
use wgpu::util::DeviceExt;

impl GpuProcessor {
    // Legacy buffer-based method for backward compatibility
    pub fn apply_adjustments(
        &self,
        image: &image::DynamicImage,
        adj: &ImageAdjustments,
    ) -> Result<Vec<u8>> {
        self.apply_adjustments_legacy(image, adj)
    }

    // Keep the legacy implementation for now
    fn apply_adjustments_legacy(
        &self,
        image: &image::DynamicImage,
        adj: &ImageAdjustments,
    ) -> Result<Vec<u8>> {
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

        // Load shader from file instead of embedded source
        let shader_source = std::fs::read_to_string("shaders/adjustments.wgsl")
            .expect("Failed to read adjustments shader");

        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("adjustments_shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        // Create buffers
        let input_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
            saturation: f32,
            temperature: f32,
            width: u32,
            height: u32,
            // Film emulation
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
            vignette_amount: f32,
            vignette_softness: f32,
            latitude: f32,
            red_gamma: f32,
            green_gamma: f32,
            blue_gamma: f32,
            black_point: f32,
            white_point: f32,
            // Color crossover matrix
            red_in_green: f32,
            red_in_blue: f32,
            green_in_red: f32,
            green_in_blue: f32,
            blue_in_red: f32,
            blue_in_green: f32,
            // Shadow/highlight tints
            shadow_tint_r: f32,
            shadow_tint_g: f32,
            shadow_tint_b: f32,
            highlight_tint_r: f32,
            highlight_tint_g: f32,
            highlight_tint_b: f32,
            // Halation color
            halation_color_r: f32,
            halation_color_g: f32,
            halation_color_b: f32,
            halation_radius: f32,
        }

        let film = &adj.film;
        let params = Params {
            exposure: adj.exposure,
            saturation: adj.saturation,
            temperature: adj.temperature,
            width,
            height,
            // Film emulation
            film_enabled: if film.enabled { 1 } else { 0 },
            film_is_bw: if film.is_bw { 1 } else { 0 },
            tone_curve_shadows: film.tone.shadows,
            tone_curve_midtones: film.tone.midtones,
            tone_curve_highlights: film.tone.highlights,
            s_curve_strength: film.tone.s_curve_strength,
            grain_amount: film.grain.amount,
            grain_size: film.grain.size,
            grain_roughness: film.grain.roughness,
            halation_amount: film.halation.amount,
            vignette_amount: film.vignette.amount,
            vignette_softness: film.vignette.softness,
            latitude: film.latitude,
            red_gamma: film.color_gamma.red,
            green_gamma: film.color_gamma.green,
            blue_gamma: film.color_gamma.blue,
            black_point: film.black_point,
            white_point: film.white_point,
            // Color crossover matrix
            red_in_green: film.color_crossover.red_in_green,
            red_in_blue: film.color_crossover.red_in_blue,
            green_in_red: film.color_crossover.green_in_red,
            green_in_blue: film.color_crossover.green_in_blue,
            blue_in_red: film.color_crossover.blue_in_red,
            blue_in_green: film.color_crossover.blue_in_green,
            // Shadow/highlight tints
            shadow_tint_r: film.shadow_tint[0],
            shadow_tint_g: film.shadow_tint[1],
            shadow_tint_b: film.shadow_tint[2],
            highlight_tint_r: film.highlight_tint[0],
            highlight_tint_g: film.highlight_tint[1],
            highlight_tint_b: film.highlight_tint[2],
            // Halation color
            halation_color_r: film.halation.color[0],
            halation_color_g: film.halation.color[1],
            halation_color_b: film.halation.color[2],
            halation_radius: film.halation.radius,
        };

        let params_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Bind group layout
        let bind_layout = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("pipeline_layout"),
                bind_group_layouts: &[&bind_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("compute_pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some("main_v2"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                });

        // Command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("command_encoder"),
            });

        const MAX_WORKGROUPS: u32 = 65535;
        let workgroup_size = 256;
        let mut offset = 0u32;
        while offset < pixel_count as u32 {
            let remaining_pixels = pixel_count as u32 - offset;
            let max_pixels_this_dispatch = MAX_WORKGROUPS * workgroup_size;
            let pixels_this_dispatch = remaining_pixels.min(max_pixels_this_dispatch);
            let groups = pixels_this_dispatch.div_ceil(workgroup_size);

            let offset_buf = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("compute_pass"),
                    timestamp_writes: None,
                });
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

        encoder.copy_buffer_to_buffer(
            &output_buf,
            0,
            &staging,
            0,
            (packed.len() * std::mem::size_of::<u32>()) as u64,
        );

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
