use super::types::{GpuProcessor, GpuPerformanceInfo};

impl GpuProcessor {
    /// Get GPU information
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// Check if RAW demosaicing is supported
    #[allow(dead_code)]
    pub fn supports_raw_demosaic(&self) -> bool {
        self.raw_demosaic_pipeline.is_some()
    }

    /// Get GPU performance information
    pub fn get_performance_info(&self) -> GpuPerformanceInfo {
        GpuPerformanceInfo {
            adapter_name: self.adapter_info.name.clone(),
            backend: self.adapter_info.backend.to_str().to_string(),
            device_type: match self.adapter_info.device_type {
                wgpu::DeviceType::DiscreteGpu => "Discrete GPU".to_string(),
                wgpu::DeviceType::IntegratedGpu => "Integrated GPU".to_string(),
                wgpu::DeviceType::VirtualGpu => "Virtual GPU".to_string(),
                wgpu::DeviceType::Cpu => "CPU".to_string(),
                _ => "Unknown".to_string(),
            },
            supports_texture_operations: true,
            supports_raw_demosaic: self.raw_demosaic_pipeline.is_some(),
        }
    }
}