/// GPU performance and capability information
#[derive(Debug, Clone)]
pub struct GpuPerformanceInfo {
    pub adapter_name: String,
    pub backend: String,
    pub device_type: String,
    pub supports_texture_operations: bool,
    pub supports_raw_demosaic: bool,
}

/// GPU-accelerated image processing with advanced features
#[derive(Debug)]
#[allow(unused)]
pub struct GpuProcessor {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub buffer_bind_group_layout: wgpu::BindGroupLayout,
    pub adjustment_pipeline: wgpu::ComputePipeline,
    pub histogram_bind_group_layout: wgpu::BindGroupLayout,
    pub histogram_pipeline: wgpu::ComputePipeline,
    pub overlay_pipeline: wgpu::ComputePipeline,
    pub raw_demosaic_pipeline: Option<wgpu::ComputePipeline>,
    pub adapter_info: wgpu::AdapterInfo,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AdjustmentParams {
    pub exposure: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub temperature: f32,
    pub tint: f32,
    pub blacks: f32,
    pub whites: f32,
    pub sharpening: f32,
    pub width: u32,
    pub height: u32,
    pub film_enabled: u32,
    pub film_is_bw: u32,
    pub tone_curve_shadows: f32,
    pub tone_curve_midtones: f32,
    pub tone_curve_highlights: f32,
    pub s_curve_strength: f32,
    pub grain_amount: f32,
    pub grain_size: f32,
    pub grain_roughness: f32,
    pub halation_amount: f32,
    pub vignette_amount: f32,
    pub vignette_softness: f32,
    pub latitude: f32,
    pub red_gamma: f32,
    pub green_gamma: f32,
    pub blue_gamma: f32,
    pub black_point: f32,
    pub white_point: f32,
}
