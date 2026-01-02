use super::types::GpuProcessor;
use anyhow::{anyhow, Result};

impl GpuProcessor {
    pub async fn new() -> Result<Self> {
        // Request high-performance adapter with compute capabilities
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow!("No suitable GPU adapter found"))?;

        let adapter_info = adapter.get_info();

        // Request device with advanced features
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("image_viewer_gpu_device"),
                    required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                        | wgpu::Features::BUFFER_BINDING_ARRAY
                        | wgpu::Features::UNIFORM_BUFFER_AND_STORAGE_TEXTURE_ARRAY_NON_UNIFORM_INDEXING,
                    required_limits: wgpu::Limits {
                        max_compute_workgroup_size_x: 1024,
                        max_compute_workgroup_size_y: 1024,
                        max_compute_workgroup_size_z: 64,
                        max_storage_buffers_per_shader_stage: 8,
                        max_storage_textures_per_shader_stage: 8,
                        max_uniform_buffers_per_shader_stage: 4,
                        max_texture_dimension_2d: 16384,
                        ..wgpu::Limits::downlevel_defaults()
                    },
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?;

        log::info!("GPU initialized: {} ({})", adapter_info.name, adapter_info.backend.to_str());

        // Create bind group layouts
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bind_group_layout"),
            entries: &[
                // Input texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Output texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Parameters
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

        let buffer_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("buffer_bind_group_layout"),
            entries: &[
                // Input buffer
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
                // Output buffer
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
                // Parameters
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

        // Create pipelines
        let adjustment_pipeline = Self::create_adjustment_pipeline(&device, &texture_bind_group_layout);
        let (histogram_bind_group_layout, histogram_pipeline) = Self::create_histogram_pipeline(&device);
        let overlay_pipeline = Self::create_overlay_pipeline(&device, &texture_bind_group_layout);
        let raw_demosaic_pipeline = Self::create_raw_demosaic_pipeline(&device, &buffer_bind_group_layout);

        Ok(Self {
            device,
            queue,
            texture_bind_group_layout,
            buffer_bind_group_layout,
            adjustment_pipeline,
            histogram_bind_group_layout,
            histogram_pipeline,
            overlay_pipeline,
            raw_demosaic_pipeline,
            adapter_info,
        })
    }
}