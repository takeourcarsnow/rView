use super::types::GpuProcessor;
use anyhow::Result;
use image::DynamicImage;
use tokio::sync::oneshot;
use wgpu::util::DeviceExt;

impl GpuProcessor {
    /// Generate focus peaking overlay using GPU
    pub async fn generate_focus_peaking_overlay(
        &self,
        image: &DynamicImage,
        threshold: f32,
    ) -> Result<DynamicImage> {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        // Create input texture
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("focus_peaking_input"),
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

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
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

        // Create output texture
        let output_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("focus_peaking_output"),
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

        // Create parameter buffer
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct OverlayParams {
            mode: u32,           // 0 = focus peaking, 1 = zebra
            threshold: f32,      // focus peaking threshold
            high_threshold: f32, // zebra high threshold
            low_threshold: f32,  // zebra low threshold
            width: u32,
            height: u32,
        }

        let params = OverlayParams {
            mode: 0, // focus peaking
            threshold,
            high_threshold: 0.0,
            low_threshold: 0.0,
            width,
            height,
        };

        let param_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("focus_peaking_params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("focus_peaking_bind_group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &output_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: param_buffer.as_entire_binding(),
                },
            ],
        });

        // Execute
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("focus_peaking_encoder"),
            });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("focus_peaking_pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.overlay_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(width.div_ceil(16), height.div_ceil(16), 1);
        }

        // Download result
        let bytes_per_row = (4 * width).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("focus_peaking_output_buffer"),
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
            let bytes_per_row = (4 * width).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
                * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
            let mut valid_data = Vec::with_capacity((width * height * 4) as usize);
            for row in 0..height {
                let start = (row * bytes_per_row) as usize;
                let end = start + (width * 4) as usize;
                valid_data.extend_from_slice(&data[start..end]);
            }
            image::ImageBuffer::from_raw(width, height, valid_data)
                .ok_or_else(|| anyhow::anyhow!("Failed to create focus peaking overlay"))?
        };

        output_buffer.unmap();

        Ok(DynamicImage::ImageRgba8(result))
    }

    /// Generate zebra overlay using GPU acceleration
    pub async fn generate_zebra_overlay(
        &self,
        image: &DynamicImage,
        high_threshold: f32,
        low_threshold: f32,
    ) -> Result<DynamicImage> {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        // Create input texture
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("zebra_input"),
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

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
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

        // Create output texture
        let output_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("zebra_output"),
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

        // Create parameter buffer
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct OverlayParams {
            mode: u32,           // 0 = focus peaking, 1 = zebra
            threshold: f32,      // focus peaking threshold
            high_threshold: f32, // zebra high threshold
            low_threshold: f32,  // zebra low threshold
            width: u32,
            height: u32,
        }

        let params = OverlayParams {
            mode: 1, // zebra
            threshold: 0.0,
            high_threshold,
            low_threshold,
            width,
            height,
        };

        let param_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("zebra_params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("zebra_bind_group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &output_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: param_buffer.as_entire_binding(),
                },
            ],
        });

        // Execute
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("zebra_encoder"),
            });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("zebra_pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.overlay_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(width.div_ceil(16), height.div_ceil(16), 1);
        }

        // Download result
        let bytes_per_row = (4 * width).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("zebra_output_buffer"),
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
            let bytes_per_row = (4 * width).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
                * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
            let mut valid_data = Vec::with_capacity((width * height * 4) as usize);
            for row in 0..height {
                let start = (row * bytes_per_row) as usize;
                let end = start + (width * 4) as usize;
                valid_data.extend_from_slice(&data[start..end]);
            }
            image::ImageBuffer::from_raw(width, height, valid_data)
                .ok_or_else(|| anyhow::anyhow!("Failed to create zebra overlay"))?
        };

        output_buffer.unmap();

        Ok(DynamicImage::ImageRgba8(result))
    }
}
