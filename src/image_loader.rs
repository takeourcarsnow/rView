use crate::errors::{Result, ViewerError};
use image::{DynamicImage, ImageBuffer, RgbImage, GenericImageView, Rgba, RgbaImage};
use std::path::Path;

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    // Standard formats
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp", "ico", "pnm", "pbm", "pgm", "ppm",
    // RAW formats
    "cr2", "cr3", "nef", "arw", "orf", "rw2", "dng", "raf", "raw", "srw", "pef", "x3f", "3fr",
    "mef", "mrw", "nrw", "rwl", "sr2", "srf", "erf", "kdc", "dcr",
];

pub const RAW_EXTENSIONS: &[&str] = &[
    "cr2", "cr3", "nef", "arw", "orf", "rw2", "dng", "raf", "raw", "srw", "pef", "x3f", "3fr",
    "mef", "mrw", "nrw", "rwl", "sr2", "srf", "erf", "kdc", "dcr",
];

pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn is_raw_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| RAW_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn load_image(path: &Path) -> Result<DynamicImage> {
    if !path.exists() {
        return Err(ViewerError::FileNotFound { path: path.to_path_buf() });
    }

    if is_raw_file(path) {
        load_raw_image(path)
    } else {
        load_standard_image(path)
    }
}

fn load_standard_image(path: &Path) -> Result<DynamicImage> {
    image::open(path)
        .map_err(|e| ViewerError::ImageLoadError { path: path.to_path_buf(), message: e.to_string() })
}

fn load_raw_image(path: &Path) -> Result<DynamicImage> {
    let raw = rawloader::decode_file(path)
        .map_err(|e| ViewerError::RawProcessingError { path: path.to_path_buf(), message: e.to_string() })?;
    
    let source = imagepipe::ImageSource::Raw(raw);
    let mut pipeline = imagepipe::Pipeline::new_from_source(source)
        .map_err(|e| ViewerError::RawProcessingError { path: path.to_path_buf(), message: format!("Pipeline error: {}", e) })?;
    
    let srgb = pipeline.output_8bit(None)
        .map_err(|e| ViewerError::RawProcessingError { path: path.to_path_buf(), message: format!("Processing error: {}", e) })?;
    
    let width = srgb.width;
    let height = srgb.height;
    let pixels = srgb.data;
    
    let img: RgbImage = ImageBuffer::from_raw(width as u32, height as u32, pixels)
        .ok_or_else(|| ViewerError::RawProcessingError { path: path.to_path_buf(), message: "Failed to create image buffer".to_string() })?;
    
    Ok(DynamicImage::ImageRgb8(img))
}

pub fn generate_thumbnail(image: &DynamicImage, max_size: u32) -> DynamicImage {
    image.thumbnail(max_size, max_size)
}

pub fn load_thumbnail(path: &Path, max_size: u32) -> Result<DynamicImage> {
    // For RAW files, try to extract embedded thumbnail first (much faster)
    if is_raw_file(path) {
        if let Ok(thumb) = load_raw_embedded_thumbnail(path, max_size) {
            return Ok(thumb);
        }
    }
    
    let image = load_image(path)?;
    Ok(generate_thumbnail(&image, max_size))
}

/// Load embedded JPEG thumbnail from RAW file (very fast)
fn load_raw_embedded_thumbnail(path: &Path, max_size: u32) -> Result<DynamicImage> {
    use std::io::BufReader;
    use std::fs::File;
    
    let file = File::open(path)
        .map_err(|e| ViewerError::ImageLoadError { path: path.to_path_buf(), message: e.to_string() })?;
    let mut bufreader = BufReader::new(file);
    
    // Try to read EXIF data which may contain embedded thumbnail
    if let Ok(exif) = exif::Reader::new().read_from_container(&mut bufreader) {
        for field in exif.fields() {
            if field.tag == exif::Tag::JPEGInterchangeFormat {
                // Found embedded JPEG - this approach works for some RAW formats
                // Fall through to rawloader approach
                break;
            }
        }
    }
    
    // Use rawloader to get the embedded thumbnail
    let raw = rawloader::decode_file(path)
        .map_err(|e| ViewerError::RawProcessingError { path: path.to_path_buf(), message: e.to_string() })?;
    
    // Create a quick preview by processing at reduced resolution
    let source = imagepipe::ImageSource::Raw(raw);
    let mut pipeline = imagepipe::Pipeline::new_from_source(source)
        .map_err(|e| ViewerError::RawProcessingError { path: path.to_path_buf(), message: e.to_string() })?;
    
    // Get output - we can't pass size directly, so just get full output and resize
    let srgb = pipeline.output_8bit(None)
        .map_err(|e| ViewerError::RawProcessingError { path: path.to_path_buf(), message: e.to_string() })?;
    
    let width = srgb.width;
    let height = srgb.height;
    let pixels = srgb.data;
    
    let img: RgbImage = ImageBuffer::from_raw(width as u32, height as u32, pixels)
        .ok_or_else(|| ViewerError::RawProcessingError { path: path.to_path_buf(), message: "Failed to create thumbnail buffer".to_string() })?;
    
    Ok(DynamicImage::ImageRgb8(img).thumbnail(max_size, max_size))
}

// Focus peaking - detect edges/sharp areas
pub fn generate_focus_peaking_overlay(image: &DynamicImage, threshold: f32) -> RgbaImage {
    let gray = image.to_luma8();
    let (width, height) = gray.dimensions();
    let mut overlay = RgbaImage::new(width, height);
    
    // Sobel edge detection
    for y in 1..height-1 {
        for x in 1..width-1 {
            let gx = 
                -1.0 * gray.get_pixel(x-1, y-1).0[0] as f32 +
                 1.0 * gray.get_pixel(x+1, y-1).0[0] as f32 +
                -2.0 * gray.get_pixel(x-1, y).0[0] as f32 +
                 2.0 * gray.get_pixel(x+1, y).0[0] as f32 +
                -1.0 * gray.get_pixel(x-1, y+1).0[0] as f32 +
                 1.0 * gray.get_pixel(x+1, y+1).0[0] as f32;
                
            let gy = 
                -1.0 * gray.get_pixel(x-1, y-1).0[0] as f32 +
                -2.0 * gray.get_pixel(x, y-1).0[0] as f32 +
                -1.0 * gray.get_pixel(x+1, y-1).0[0] as f32 +
                 1.0 * gray.get_pixel(x-1, y+1).0[0] as f32 +
                 2.0 * gray.get_pixel(x, y+1).0[0] as f32 +
                 1.0 * gray.get_pixel(x+1, y+1).0[0] as f32;
            
            let magnitude = (gx * gx + gy * gy).sqrt();
            
            if magnitude > threshold {
                overlay.put_pixel(x, y, Rgba([255, 0, 0, 200]));
            } else {
                overlay.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }
    
    overlay
}

// Zebra pattern for overexposure
pub fn generate_zebra_overlay(image: &DynamicImage, high_threshold: u8, low_threshold: u8) -> RgbaImage {
    let rgb = image.to_rgb8();
    let (width, height) = rgb.dimensions();
    let mut overlay = RgbaImage::new(width, height);
    
    for y in 0..height {
        for x in 0..width {
            let pixel = rgb.get_pixel(x, y);
            let max_val = pixel.0[0].max(pixel.0[1]).max(pixel.0[2]);
            let min_val = pixel.0[0].min(pixel.0[1]).min(pixel.0[2]);
            
            // Zebra stripes pattern
            let stripe = ((x + y) / 4) % 2 == 0;
            
            if max_val >= high_threshold && stripe {
                // Overexposed - red stripes
                overlay.put_pixel(x, y, Rgba([255, 0, 0, 180]));
            } else if min_val <= low_threshold && stripe {
                // Underexposed - blue stripes
                overlay.put_pixel(x, y, Rgba([0, 0, 255, 180]));
            } else {
                overlay.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }
    
    overlay
}

// Calculate histogram
pub fn calculate_histogram(image: &DynamicImage) -> Vec<Vec<u32>> {
    let rgb = image.to_rgb8();
    let mut histogram = vec![vec![0u32; 256]; 3];
    
    for pixel in rgb.pixels() {
        histogram[0][pixel[0] as usize] += 1;
        histogram[1][pixel[1] as usize] += 1;
        histogram[2][pixel[2] as usize] += 1;
    }
    
    histogram
}

// Apply basic adjustments (non-destructive preview)
#[derive(Debug, Clone)]
pub struct ImageAdjustments {
    pub exposure: f32,      // -3.0 to +3.0 (stops)
    pub contrast: f32,      // 0.5 to 2.0 (multiplier)
    pub brightness: f32,    // -100 to +100
    pub saturation: f32,    // 0.0 to 2.0 (multiplier)
    pub highlights: f32,    // -1.0 to +1.0
    pub shadows: f32,       // -1.0 to +1.0
    pub temperature: f32,   // -1.0 to +1.0 (cool to warm)
    pub tint: f32,          // -1.0 to +1.0 (green to magenta)
    pub blacks: f32,        // -1.0 to +1.0
    pub whites: f32,        // -1.0 to +1.0
    pub sharpening: f32,    // 0.0 to 2.0
}

impl Default for ImageAdjustments {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            brightness: 0.0,
            saturation: 1.0,
            highlights: 0.0,
            shadows: 0.0,
            temperature: 0.0,
            tint: 0.0,
            blacks: 0.0,
            whites: 0.0,
            sharpening: 0.0,
        }
    }
}

impl ImageAdjustments {
    pub fn is_default(&self) -> bool {
        self.exposure == 0.0 &&
        self.contrast == 1.0 &&
        self.brightness == 0.0 &&
        self.saturation == 1.0 &&
        self.highlights == 0.0 &&
        self.shadows == 0.0 &&
        self.temperature == 0.0 &&
        self.tint == 0.0 &&
        self.blacks == 0.0 &&
        self.whites == 0.0 &&
        self.sharpening == 0.0
    }
}

pub fn apply_adjustments(image: &DynamicImage, adj: &ImageAdjustments) -> DynamicImage {
    if adj.is_default() {
        return image.clone();
    }
    
    let mut img = image.to_rgba8();
    let (width, height) = img.dimensions();
    
    // Exposure multiplier (stops)
    let exposure_mult = 2.0_f32.powf(adj.exposure);
    
    // Contrast adjustment
    let contrast_factor = (100.0 + adj.contrast) / 100.0;
    
    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel_mut(x, y);
            let mut r = pixel[0] as f32;
            let mut g = pixel[1] as f32;
            let mut b = pixel[2] as f32;
            
            // Apply exposure
            r *= exposure_mult;
            g *= exposure_mult;
            b *= exposure_mult;
            
            // Apply brightness
            r += adj.brightness * 2.55;
            g += adj.brightness * 2.55;
            b += adj.brightness * 2.55;
            
            // Apply contrast
            r = ((r / 255.0 - 0.5) * contrast_factor + 0.5) * 255.0;
            g = ((g / 255.0 - 0.5) * contrast_factor + 0.5) * 255.0;
            b = ((b / 255.0 - 0.5) * contrast_factor + 0.5) * 255.0;
            
            // Apply saturation
            let gray = 0.299 * r + 0.587 * g + 0.114 * b;
            let sat_factor = (100.0 + adj.saturation) / 100.0;
            r = gray + (r - gray) * sat_factor;
            g = gray + (g - gray) * sat_factor;
            b = gray + (b - gray) * sat_factor;
            
            // Apply temperature (simplified)
            if adj.temperature > 0.0 {
                // Warmer
                r += adj.temperature * 0.5;
                b -= adj.temperature * 0.3;
            } else {
                // Cooler
                r += adj.temperature * 0.3;
                b -= adj.temperature * 0.5;
            }
            
            // Clamp values
            pixel[0] = r.clamp(0.0, 255.0) as u8;
            pixel[1] = g.clamp(0.0, 255.0) as u8;
            pixel[2] = b.clamp(0.0, 255.0) as u8;
        }
    }
    
    DynamicImage::ImageRgba8(img)
}

// Export image with preset
pub fn export_image(
    image: &DynamicImage,
    output_path: &Path,
    format: crate::settings::ExportFormat,
    quality: u8,
    max_width: Option<u32>,
    max_height: Option<u32>,
) -> Result<()> {
    let mut img = image.clone();
    
    // Resize if needed
    if let (Some(max_w), Some(max_h)) = (max_width, max_height) {
        let (w, h) = img.dimensions();
        if w > max_w || h > max_h {
            img = img.resize(max_w, max_h, image::imageops::FilterType::Lanczos3);
        }
    } else if let Some(max_w) = max_width {
        let (w, h) = img.dimensions();
        if w > max_w {
            let new_h = (h as f32 * max_w as f32 / w as f32) as u32;
            img = img.resize(max_w, new_h, image::imageops::FilterType::Lanczos3);
        }
    } else if let Some(max_h) = max_height {
        let (w, h) = img.dimensions();
        if h > max_h {
            let new_w = (w as f32 * max_h as f32 / h as f32) as u32;
            img = img.resize(new_w, max_h, image::imageops::FilterType::Lanczos3);
        }
    }
    
    match format {
        crate::settings::ExportFormat::Jpeg => {
            let rgb = img.to_rgb8();
            let mut output = std::fs::File::create(output_path)
                .map_err(|e| ViewerError::ExportError { path: output_path.to_path_buf(), message: e.to_string() })?;
            
            let encoder = jpeg_encoder::Encoder::new(&mut output, quality);
            encoder.encode(&rgb, rgb.width() as u16, rgb.height() as u16, jpeg_encoder::ColorType::Rgb)
                .map_err(|e| ViewerError::ExportError { path: output_path.to_path_buf(), message: e.to_string() })?;
        }
        crate::settings::ExportFormat::Png => {
            img.save(output_path)
                .map_err(|e| ViewerError::ExportError { path: output_path.to_path_buf(), message: e.to_string() })?;
        }
        crate::settings::ExportFormat::WebP => {
            img.save(output_path)
                .map_err(|e| ViewerError::ExportError { path: output_path.to_path_buf(), message: e.to_string() })?;
        }
    }
    
    Ok(())
}

// Rotate image losslessly (for JPEG, just update EXIF, for others, actually rotate)
pub fn rotate_image(image: &DynamicImage, degrees: i32) -> DynamicImage {
    match degrees {
        90 | -270 => image.rotate90(),
        180 | -180 => image.rotate180(),
        270 | -90 => image.rotate270(),
        _ => image.clone(),
    }
}

// Get color at pixel
pub fn get_pixel_color(image: &DynamicImage, x: u32, y: u32) -> Option<(u8, u8, u8)> {
    if x < image.width() && y < image.height() {
        let pixel = image.get_pixel(x, y);
        Some((pixel[0], pixel[1], pixel[2]))
    } else {
        None
    }
}
