use crate::errors::{Result, ViewerError};
use image::{DynamicImage, ImageBuffer, RgbImage};
use std::path::Path;

/// Supported image extensions
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    // Standard formats
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp", "ico", "pnm", "pbm", "pgm", "ppm",
    // RAW formats
    "cr2", "cr3", "nef", "arw", "orf", "rw2", "dng", "raf", "raw", "srw", "pef", "x3f", "3fr",
    "mef", "mrw", "nrw", "rwl", "sr2", "srf", "erf", "kdc", "dcr",
];

/// RAW file extensions
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
        return Err(ViewerError::FileNotFound(path.display().to_string()));
    }

    if is_raw_file(path) {
        load_raw_image(path)
    } else {
        load_standard_image(path)
    }
}

fn load_standard_image(path: &Path) -> Result<DynamicImage> {
    image::open(path)
        .map_err(|e| ViewerError::ImageLoadError(format!("{}: {}", path.display(), e)))
}

fn load_raw_image(path: &Path) -> Result<DynamicImage> {
    // Try using rawloader for RAW files
    let raw = rawloader::decode_file(path)
        .map_err(|e| ViewerError::RawProcessingError(format!("{}: {}", path.display(), e)))?;
    
    // Process the raw data using imagepipe
    let source = imagepipe::ImageSource::Raw(raw);
    let mut pipeline = imagepipe::Pipeline::new_from_source(source)
        .map_err(|e| ViewerError::RawProcessingError(format!("Pipeline error: {}", e)))?;
    
    let srgb = pipeline.output_8bit(None)
        .map_err(|e| ViewerError::RawProcessingError(format!("Processing error: {}", e)))?;
    
    // Convert to DynamicImage
    let width = srgb.width;
    let height = srgb.height;
    let pixels = srgb.data;
    
    let img: RgbImage = ImageBuffer::from_raw(width as u32, height as u32, pixels)
        .ok_or_else(|| ViewerError::RawProcessingError("Failed to create image buffer".to_string()))?;
    
    Ok(DynamicImage::ImageRgb8(img))
}

/// Generate a thumbnail from an image
pub fn generate_thumbnail(image: &DynamicImage, max_size: u32) -> DynamicImage {
    image.thumbnail(max_size, max_size)
}

/// Load a thumbnail efficiently (for standard images, try to use embedded thumbnail)
pub fn load_thumbnail(path: &Path, max_size: u32) -> Result<DynamicImage> {
    // Fall back to loading full image and resizing
    let image = load_image(path)?;
    Ok(generate_thumbnail(&image, max_size))
}

/// Get image dimensions without loading the full image
pub fn get_image_dimensions(path: &Path) -> Result<(u32, u32)> {
    if is_raw_file(path) {
        let raw = rawloader::decode_file(path)
            .map_err(|e| ViewerError::RawProcessingError(e.to_string()))?;
        Ok((raw.width as u32, raw.height as u32))
    } else {
        let dims = image::image_dimensions(path)
            .map_err(|e| ViewerError::ImageLoadError(e.to_string()))?;
        Ok(dims)
    }
}
