use super::types::GpuProcessor;
use anyhow::{anyhow, Result};
use image::DynamicImage;
use wgpu::util::DeviceExt;

impl GpuProcessor {
    /// Demosaic RAW image data using GPU acceleration
    #[allow(dead_code)]
    pub async fn demosaic_raw(
        &self,
        raw_data: &[u16],
        width: u32,
        height: u32,
        bayer_pattern: u32,
    ) -> Result<DynamicImage> {
        if self.raw_demosaic_pipeline.is_none() {
            return Err(anyhow!("RAW demosaicing not supported on this GPU"));
        }

        let pixel_count = (width * height) as usize;

        // Convert u16 raw data to u32 for GPU (pack two u16 values per u32 for efficiency)
        let mut gpu_raw_data = Vec::with_capacity(pixel_count);
        for &value in raw_data {
            gpu_raw_data.push(value as u32);
        }

        // Create input buffer
        let input_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("raw_input_buffer"),
                contents: bytemuck::cast_slice(&gpu_raw_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Create output buffer
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("demosaic_output_buffer"),
            size: (pixel_count * 4) as u64, // RGBA u32 per pixel
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Parameters for the shader
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct DemosaicParams {
            width: u32,
            height: u32,
            bayer_pattern: u32,
            black_level: [f32; 4],
            white_level: [f32; 4],
            color_matrix: [[f32; 3]; 3],
            gamma: f32,
        }

        let params = DemosaicParams {
            width,
            height,
            bayer_pattern,
            black_level: [0.0, 0.0, 0.0, 0.0], // Default black level
            white_level: [65535.0, 65535.0, 65535.0, 65535.0], // 16-bit white level
            color_matrix: [
                [1.0, 0.0, 0.0], // Identity matrix for now
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            gamma: 2.2,
        };

        let param_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("demosaic_params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("demosaic_bind_group"),
            layout: &self.buffer_bind_group_layout,
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
            ],
        });

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("demosaic_encoder"),
            });

        // Dispatch compute shader
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("demosaic_pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(self.raw_demosaic_pipeline.as_ref().unwrap());
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let workgroups_x = width.div_ceil(16);
            let workgroups_y = height.div_ceil(16);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Create staging buffer for reading results
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("demosaic_staging"),
            size: (pixel_count * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Copy output to staging buffer
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (pixel_count * 4) as u64,
        );

        // Submit commands
        self.queue.submit(Some(encoder.finish()));

        // Read back results
        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.poll(wgpu::Maintain::Wait);
        let _ = rx
            .receive()
            .await
            .ok_or_else(|| anyhow!("Failed to receive demosaic result"))?;

        let data = buffer_slice.get_mapped_range().to_vec();
        staging_buffer.unmap();

        // Convert RGBA u32 to u8
        let mut rgba_data = Vec::with_capacity(pixel_count * 4);
        for chunk in data.chunks_exact(4) {
            let v = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let r = (v & 0xFF) as u8;
            let g = ((v >> 8) & 0xFF) as u8;
            let b = ((v >> 16) & 0xFF) as u8;
            let a = ((v >> 24) & 0xFF) as u8;
            rgba_data.push(r);
            rgba_data.push(g);
            rgba_data.push(b);
            rgba_data.push(a);
        }

        // Create DynamicImage from RGBA data
        image::RgbaImage::from_raw(width, height, rgba_data)
            .map(DynamicImage::ImageRgba8)
            .ok_or_else(|| anyhow!("Failed to create image from demosaiced data"))
    }
}
