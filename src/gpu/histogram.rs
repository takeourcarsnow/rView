use super::types::GpuProcessor;
use anyhow::Result;
use image::DynamicImage;
use tokio::sync::oneshot;

impl GpuProcessor {
    /// Compute histogram using GPU acceleration
    pub async fn compute_histogram(&self, image: &DynamicImage) -> Result<Vec<Vec<u32>>> {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        // Create input texture
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("histogram_input"),
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

        // Create histogram buffer (4 channels × 256 bins)
        let histogram_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("histogram_buffer"),
            size: (4 * 256 * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Clear histogram buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.clear_buffer(&histogram_buffer, 0, None);
        self.queue.submit(Some(encoder.finish()));

        // Create bind group for histogram computation
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("histogram_bind_group"),
            layout: &self.histogram_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: histogram_buffer.as_entire_binding(),
                },
            ],
        });

        // Execute histogram computation
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("histogram_encoder"),
            });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("histogram_pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.histogram_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(width.div_ceil(16), height.div_ceil(16), 1);
        }

        // Download histogram
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("histogram_staging"),
            size: (4 * 256 * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(
            &histogram_buffer,
            0,
            &staging_buffer,
            0,
            4 * 256 * std::mem::size_of::<u32>() as u64,
        );
        self.queue.submit(Some(encoder.finish()));

        // Read back result
        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });

        self.device.poll(wgpu::Maintain::Wait);
        rx.await??;

        // Process the data and drop the view before unmapping
        let result = {
            let data = buffer_slice.get_mapped_range();
            let histogram_data: &[u32] = bytemuck::cast_slice(&data);

            let mut result = vec![vec![0u32; 256]; 4];
            for (i, &value) in histogram_data.iter().enumerate() {
                let channel = i / 256;
                let bin = i % 256;
                result[channel][bin] = value;
            }
            result
        };

        staging_buffer.unmap();

        Ok(result)
    }
}
