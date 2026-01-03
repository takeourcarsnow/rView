use crate::gpu::types::GpuProcessor;
use crate::image_loader::ImageAdjustments;
use anyhow::Result;
use image::DynamicImage;
use tokio::sync::oneshot;
use wgpu::util::DeviceExt;

impl GpuProcessor {
    /// Apply adjustments using buffer-based processing for compatibility
    pub async fn apply_adjustments_texture(
        &self,
        image: &DynamicImage,
        adj: &ImageAdjustments,
    ) -> Result<DynamicImage> {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        let pixel_count = (width * height) as usize;

        // Convert RGBA to packed u32 array for GPU processing
        let mut input_pixels = Vec::with_capacity(pixel_count);
        for pixel in rgba.pixels() {
            let r = pixel[0] as u32;
            let g = pixel[1] as u32;
            let b = pixel[2] as u32;
            let a = pixel[3] as u32;
            let packed = (a << 24) | (b << 16) | (g << 8) | r;
            input_pixels.push(packed);
        }

        // Create input and output buffers
        let input_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("adjustment_input_buffer"),
                contents: bytemuck::cast_slice(&input_pixels),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adjustment_output_buffer"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create parameter buffer
        let params = Self::create_adjustment_params(adj, width, height);
        let param_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("adjustment_params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create offset buffer (for chunked processing if needed)
        let offset = 0u32;
        let offset_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("adjustment_offset"),
                contents: bytemuck::bytes_of(&offset),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group with the correct layout
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("adjustment_bind_group"),
            layout: &self.adjustment_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: param_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: offset_buffer.as_entire_binding(),
                },
            ],
        });

        // Execute compute pass
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("adjustment_encoder"),
            });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("adjustment_pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.adjustment_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(((pixel_count + 255) / 256) as u32, 1, 1);
        }

        // Download result
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adjustment_staging_buffer"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (pixel_count * std::mem::size_of::<u32>()) as u64,
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

        // Process the data and create result image
        let result = {
            let data = buffer_slice.get_mapped_range();
            let output_pixels: &[u32] = bytemuck::cast_slice(&data);

            let mut result_pixels = Vec::with_capacity(pixel_count * 4);
            for &packed in output_pixels {
                let r = (packed & 0xFF) as u8;
                let g = ((packed >> 8) & 0xFF) as u8;
                let b = ((packed >> 16) & 0xFF) as u8;
                let a = ((packed >> 24) & 0xFF) as u8;
                result_pixels.extend_from_slice(&[r, g, b, a]);
            }

            image::ImageBuffer::from_raw(width, height, result_pixels)
                .ok_or_else(|| anyhow::anyhow!("Failed to create result image"))?
        };

        staging_buffer.unmap();

        Ok(DynamicImage::ImageRgba8(result))
    }
}
