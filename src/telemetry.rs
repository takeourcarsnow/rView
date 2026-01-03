use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryData {
    pub session_start: u64,
    pub images_viewed: u64,
    pub folders_opened: u64,
    pub adjustments_made: u64,
    pub gpu_enabled: bool,
    pub app_version: String,
    pub os_info: String,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Telemetry {
    data: TelemetryData,
    data_path: PathBuf,
    enabled: bool,
}

#[allow(dead_code)]
impl Telemetry {
    pub fn new(enabled: bool) -> Self {
        let data_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rview")
            .join("telemetry.json");

        let data = Self::load_data(&data_path);

        Self {
            data,
            data_path,
            enabled,
        }
    }

    fn load_data(path: &PathBuf) -> TelemetryData {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(data) = serde_json::from_str(&content) {
                return data;
            }
        }

        // Default data
        TelemetryData {
            session_start: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            images_viewed: 0,
            folders_opened: 0,
            adjustments_made: 0,
            gpu_enabled: false,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            os_info: std::env::consts::OS.to_string(),
        }
    }

    pub fn record_image_viewed(&mut self) {
        if self.enabled {
            self.data.images_viewed += 1;
            self.save_data();
        }
    }

    pub fn record_folder_opened(&mut self) {
        if self.enabled {
            self.data.folders_opened += 1;
            self.save_data();
        }
    }

    pub fn record_adjustment_made(&mut self) {
        if self.enabled {
            self.data.adjustments_made += 1;
            self.save_data();
        }
    }

    pub fn set_gpu_enabled(&mut self, enabled: bool) {
        if self.enabled {
            self.data.gpu_enabled = enabled;
            self.save_data();
        }
    }

    fn save_data(&self) {
        if let Some(parent) = self.data_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(json) = serde_json::to_string_pretty(&self.data) {
            let _ = fs::write(&self.data_path, json);
        }
    }

    pub fn get_stats(&self) -> &TelemetryData {
        &self.data
    }

    /// Send telemetry data to a remote server (opt-in only)
    pub async fn send_telemetry(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.enabled {
            return Ok(());
        }

        // In a real implementation, this would send anonymized data to a server
        // For now, just log it
        log::info!("Telemetry data: {:?}", self.data);
        Ok(())
    }
}
