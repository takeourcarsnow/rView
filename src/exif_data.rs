use std::path::Path;
use std::io::BufReader;
use std::fs::File;
use exif::{In, Tag, Reader};

#[derive(Debug, Clone, Default)]
pub struct ExifInfo {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens: Option<String>,
    pub focal_length: Option<String>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub iso: Option<String>,
    pub date_taken: Option<String>,
    pub dimensions: Option<String>,
    pub file_size: Option<String>,
    pub exposure_compensation: Option<String>,
    pub flash: Option<String>,
    pub white_balance: Option<String>,
    pub metering_mode: Option<String>,
    pub exposure_program: Option<String>,
    pub gps_coordinates: Option<String>,
    pub orientation: Option<u32>,
}

impl ExifInfo {
    pub fn from_file(path: &Path) -> Self {
        let mut info = ExifInfo::default();
        
        // Get file size
        if let Ok(metadata) = std::fs::metadata(path) {
            let size = metadata.len();
            info.file_size = Some(format_file_size(size));
        }
        
        // Try to read EXIF data
        if let Ok(file) = File::open(path) {
            let _reader = BufReader::new(file);
            if let Ok(exif) = Reader::new().read_from_container(&mut BufReader::new(std::fs::File::open(path).unwrap())) {
                // Camera info
                if let Some(field) = exif.get_field(Tag::Make, In::PRIMARY) {
                    info.camera_make = Some(field.display_value().to_string().trim_matches('"').to_string());
                }
                
                if let Some(field) = exif.get_field(Tag::Model, In::PRIMARY) {
                    info.camera_model = Some(field.display_value().to_string().trim_matches('"').to_string());
                }
                
                if let Some(field) = exif.get_field(Tag::LensModel, In::PRIMARY) {
                    info.lens = Some(field.display_value().to_string().trim_matches('"').to_string());
                }
                
                // Exposure info
                if let Some(field) = exif.get_field(Tag::FocalLength, In::PRIMARY) {
                    info.focal_length = Some(field.display_value().to_string());
                }
                
                if let Some(field) = exif.get_field(Tag::FNumber, In::PRIMARY) {
                    info.aperture = Some(field.display_value().to_string());
                }
                
                if let Some(field) = exif.get_field(Tag::ExposureTime, In::PRIMARY) {
                    info.shutter_speed = Some(field.display_value().to_string());
                }
                
                if let Some(field) = exif.get_field(Tag::PhotographicSensitivity, In::PRIMARY) {
                    info.iso = Some(format!("ISO {}", field.display_value()));
                }
                
                if let Some(field) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
                    info.date_taken = Some(field.display_value().to_string().trim_matches('"').to_string());
                }
                
                if let Some(field) = exif.get_field(Tag::ExposureBiasValue, In::PRIMARY) {
                    info.exposure_compensation = Some(field.display_value().to_string());
                }
                
                if let Some(field) = exif.get_field(Tag::Flash, In::PRIMARY) {
                    info.flash = Some(field.display_value().to_string());
                }
                
                if let Some(field) = exif.get_field(Tag::WhiteBalance, In::PRIMARY) {
                    info.white_balance = Some(field.display_value().to_string());
                }
                
                if let Some(field) = exif.get_field(Tag::MeteringMode, In::PRIMARY) {
                    info.metering_mode = Some(field.display_value().to_string());
                }
                
                if let Some(field) = exif.get_field(Tag::ExposureProgram, In::PRIMARY) {
                    info.exposure_program = Some(field.display_value().to_string());
                }
                
                // Image dimensions from EXIF
                let width = exif.get_field(Tag::PixelXDimension, In::PRIMARY)
                    .or_else(|| exif.get_field(Tag::ImageWidth, In::PRIMARY));
                let height = exif.get_field(Tag::PixelYDimension, In::PRIMARY)
                    .or_else(|| exif.get_field(Tag::ImageLength, In::PRIMARY));
                
                if let (Some(w), Some(h)) = (width, height) {
                    info.dimensions = Some(format!("{} × {}", w.display_value(), h.display_value()));
                }
                
                // Orientation
                if let Some(field) = exif.get_field(Tag::Orientation, In::PRIMARY) {
                    if let exif::Value::Short(ref v) = field.value {
                        if !v.is_empty() {
                            info.orientation = Some(v[0] as u32);
                        }
                    }
                }
                
                // GPS coordinates
                let lat = exif.get_field(Tag::GPSLatitude, In::PRIMARY);
                let lon = exif.get_field(Tag::GPSLongitude, In::PRIMARY);
                if lat.is_some() && lon.is_some() {
                    info.gps_coordinates = Some("Available".to_string());
                }
            }
        }
        
        info
    }
    
    pub fn has_data(&self) -> bool {
        self.camera_make.is_some() || 
        self.camera_model.is_some() || 
        self.focal_length.is_some() ||
        self.aperture.is_some() ||
        self.shutter_speed.is_some() ||
        self.iso.is_some()
    }
}

fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
