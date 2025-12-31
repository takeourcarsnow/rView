use crate::errors::{Result, ViewerError};
use image::{DynamicImage, ImageBuffer, RgbImage, Rgba, RgbaImage};
use std::path::Path;
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use num_cpus;

lazy_static::lazy_static! {
    static ref RAW_PROCESSING_POOL: rayon::ThreadPool = {
        let num_threads = 1; // Use single thread to avoid multi-threading issues with rawloader
        ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .thread_name(|i| format!("raw-processor-{}", i))
            .stack_size(8 * 1024 * 1024) // 8MB stack to prevent overflow
            .build()
            .expect("Failed to create RAW processing thread pool")
    };
}

// GPU acceleration stubs were removed: not used in current codebase.

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

    // Check file size to prevent loading extremely large images that could cause crashes
    if let Ok(metadata) = std::fs::metadata(path) {
        let file_size = metadata.len();
        // Limit to 500MB to prevent memory issues
        if file_size > 500 * 1024 * 1024 {
            return Err(ViewerError::ImageLoadError { 
                path: path.to_path_buf(), 
                message: format!("File too large: {}MB (max 500MB)", file_size / (1024 * 1024)) 
            });
        }
    }

    crate::profiler::with_profiler(|p| p.start_timer("image_load"));
    let result = if is_raw_file(path) {
        load_raw_image(path)
    } else {
        load_standard_image(path)
    };
    crate::profiler::with_profiler(|p| p.end_timer("image_load"));

    // Check image dimensions to prevent creating textures that are too large
    match &result {
        Ok(img) => {
            let (width, height) = img.dimensions();
            let megapixels = (width as u64 * height as u64) / 1_000_000;
            if megapixels > 100 {
                return Err(ViewerError::ImageLoadError { 
                    path: path.to_path_buf(), 
                    message: format!("Image too large: {}MP (max 100MP)", megapixels) 
                });
            }
        }
        Err(_) => {}
    }

    result
}

fn load_standard_image(path: &Path) -> Result<DynamicImage> {
    // For large files (>50MB), use memory mapping to avoid loading entire file into RAM
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.len() > 50 * 1024 * 1024 { // 50MB threshold
            return load_image_memory_mapped(path);
        }
    }

    image::open(path)
        .map_err(|e| ViewerError::ImageLoadError { path: path.to_path_buf(), message: e.to_string() })
}

fn load_image_memory_mapped(path: &Path) -> Result<DynamicImage> {
    use std::fs::File;
    use memmap2::Mmap;

    let file = File::open(path)
        .map_err(|e| ViewerError::ImageLoadError { path: path.to_path_buf(), message: e.to_string() })?;

    let mmap = unsafe { Mmap::map(&file) }
        .map_err(|e| ViewerError::ImageLoadError { path: path.to_path_buf(), message: format!("Memory mapping failed: {}", e) })?;

    image::load_from_memory(&mmap)
        .map_err(|e| ViewerError::ImageLoadError { path: path.to_path_buf(), message: e.to_string() })
}

fn load_raw_image(path: &Path) -> Result<DynamicImage> {
    log::info!("Loading RAW image: {:?}", path);
    RAW_PROCESSING_POOL.install(|| {
        // Wrap in catch_unwind to handle panics in rawloader
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            log::debug!("Decoding RAW file: {:?}", path);
            // Attempt to decode with rawloader. If it fails, try a safe fallback: extract an embedded JPEG preview
            // and use it as the image so that DNGs exported by Lightroom that rawloader can't decode still display.
            let raw = match rawloader::decode_file(path) {
                Ok(r) => r,
                Err(e) => {
                    log::error!("rawloader decode_file failed for {:?}: {}", path, e);
                    // Try to extract embedded thumbnail as a fallback
                    match load_raw_embedded_thumbnail(path, 16384) {
                        Ok(thumb) => {
                            log::warn!("Using embedded thumbnail as fallback for {:?}", path);
                            return Ok(thumb);
                        }
                        Err(_) => {
                            return Err(ViewerError::RawProcessingError { path: path.to_path_buf(), message: e.to_string() });
                        }
                    }
                }
            };

            log::debug!("Creating pipeline for {:?}", path);
            let source = imagepipe::ImageSource::Raw(raw);
            let mut pipeline = imagepipe::Pipeline::new_from_source(source)
                .map_err(|e| {
                    log::error!("Pipeline creation failed for {:?}: {}", path, e);
                    ViewerError::RawProcessingError { path: path.to_path_buf(), message: format!("Pipeline error: {}", e) }
                })?;

            log::debug!("Processing image for {:?}", path);
            let srgb = pipeline.output_8bit(None)
                .map_err(|e| {
                    log::error!("Pipeline output failed for {:?}: {}", path, e);
                    ViewerError::RawProcessingError { path: path.to_path_buf(), message: format!("Processing error: {}", e) }
                })?;

            let width = srgb.width;
            let height = srgb.height;
            let pixels = srgb.data;

            log::debug!("Creating image buffer for {:?}, size {}x{}", path, width, height);
            let img: RgbImage = ImageBuffer::from_raw(width as u32, height as u32, pixels)
                .ok_or_else(|| {
                    log::error!("Failed to create image buffer for {:?}", path);
                    ViewerError::RawProcessingError { path: path.to_path_buf(), message: "Failed to create image buffer".to_string() }
                })?;

            log::info!("Successfully loaded RAW image: {:?}", path);
            Ok(DynamicImage::ImageRgb8(img))
        }));

        match result {
            Ok(img) => img,
            Err(_) => {
                log::error!("RAW processing panicked for file: {:?}", path);
                Err(ViewerError::RawProcessingError { path: path.to_path_buf(), message: "Raw processing panicked, possibly due to corrupted file or unsupported format".to_string() })
            },
        }
    })
}

pub fn generate_thumbnail(image: &DynamicImage, max_size: u32) -> DynamicImage {
    image.thumbnail(max_size, max_size)
}

pub fn load_thumbnail(path: &Path, max_size: u32) -> Result<DynamicImage> {
    crate::profiler::with_profiler(|p| p.start_timer("thumbnail_load"));
    let result = load_thumbnail_impl(path, max_size);
    crate::profiler::with_profiler(|p| p.end_timer("thumbnail_load"));
    result
}

fn load_thumbnail_impl(path: &Path, max_size: u32) -> Result<DynamicImage> {
    // For RAW files, try to extract embedded thumbnail first (much faster)
    if is_raw_file(path) {
        if let Ok(thumb) = load_raw_embedded_thumbnail(path, max_size) {
            return Ok(thumb);
        }

        // No embedded thumbnail available; as a fallback, attempt a full RAW decode and generate a thumbnail from it.
        // This is more expensive but ensures files (like some Lightroom-exported DNGs) still show a preview.
        log::warn!("No embedded thumbnail for {:?}; attempting full RAW decode to generate thumbnail", path);
        match load_raw_image(path) {
            Ok(img) => return Ok(generate_thumbnail(&img, max_size)),
            Err(e) => {
                log::warn!("Full RAW decode fallback for thumbnail failed for {:?}: {}", path, e);
                return Err(ViewerError::ImageLoadError { path: path.to_path_buf(), message: "No embedded thumbnail for RAW file".to_string() });
            }
        }
    }
    
    let image = load_image(path)?;
    Ok(generate_thumbnail(&image, max_size))
}

/// Load embedded JPEG thumbnail from RAW file (very fast). This version attempts to extract an embedded JPEG via EXIF tags
/// but does NOT fall back to full RAW decoding to avoid expensive or unsafe raw processing here.
pub fn load_raw_embedded_thumbnail(path: &Path, max_size: u32) -> Result<DynamicImage> {
    use std::io::{BufReader, Read, Seek, SeekFrom};
    use std::fs::File;

    let file = File::open(path)
        .map_err(|e| ViewerError::ImageLoadError { path: path.to_path_buf(), message: e.to_string() })?;
    let mut bufreader = BufReader::new(&file);

    // Try to read EXIF data which may contain an embedded JPEG thumbnail with offset/length fields
    if let Ok(exif) = exif::Reader::new().read_from_container(&mut bufreader) {
        let mut offset: Option<u64> = None;
        let mut length: Option<u64> = None;
        for field in exif.fields() {
            if field.tag == exif::Tag::JPEGInterchangeFormat {
                if let Some(off) = field.value.get_uint(0) {
                    offset = Some(off as u64);
                }
            }
            if field.tag == exif::Tag::JPEGInterchangeFormatLength {
                if let Some(len) = field.value.get_uint(0) {
                    length = Some(len as u64);
                }
            }
        }

        if let (Some(off), Some(len)) = (offset, length) {
            // Read embedded JPEG bytes directly
            let mut f = File::open(path)
                .map_err(|e| ViewerError::ImageLoadError { path: path.to_path_buf(), message: e.to_string() })?;
            f.seek(SeekFrom::Start(off))
                .map_err(|e| ViewerError::ImageLoadError { path: path.to_path_buf(), message: e.to_string() })?;
            let mut buf = vec![0u8; len as usize];
            f.read_exact(&mut buf)
                .map_err(|e| ViewerError::ImageLoadError { path: path.to_path_buf(), message: e.to_string() })?;
            if let Ok(img) = image::load_from_memory(&buf) {
                return Ok(img.thumbnail(max_size, max_size));
            }
        }
    }

    // No embedded JPEG found via EXIF tags; try scanning the file for JPEG signatures as a fallback
    // (Some DNGs exported by Lightroom place previews without standard EXIF thumbnail tags)
    let data = std::fs::read(path)
        .map_err(|e| ViewerError::ImageLoadError { path: path.to_path_buf(), message: e.to_string() })?;

    // Find JPEG start (0xFFD8) and end (0xFFD9) markers and try the largest candidates first
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    let mut i = 0usize;
    while i + 1 < data.len() {
        if data[i] == 0xFF && data[i + 1] == 0xD8 {
            // found start
            let mut j = i + 2;
            while j + 1 < data.len() {
                if data[j] == 0xFF && data[j + 1] == 0xD9 {
                    candidates.push((i, j + 2)); // end is inclusive index + 1
                    i = j + 2;
                    break;
                }
                j += 1;
            }
        }
        i += 1;
    }

    // Sort by length descending so we try the largest (likely preview) first
    candidates.sort_by(|a, b| (b.1 - b.0).cmp(&(a.1 - a.0)));

    for (s, e) in candidates {
        let slice = &data[s..e];
        // skip obviously too small fragments
        if slice.len() < 512 { continue; }
        if let Ok(img) = image::load_from_memory(slice) {
            log::warn!("Using embedded JPEG scan fallback for {:?} ({} bytes)", path, slice.len());
            return Ok(img.thumbnail(max_size, max_size));
        }
    }

    Err(ViewerError::ImageLoadError { path: path.to_path_buf(), message: "No embedded thumbnail found".to_string() })
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
                -(gray.get_pixel(x-1, y-1).0[0] as f32) +
                 1.0 * gray.get_pixel(x+1, y-1).0[0] as f32 +
                -2.0 * gray.get_pixel(x-1, y).0[0] as f32 +
                 2.0 * gray.get_pixel(x+1, y).0[0] as f32 +
                -(gray.get_pixel(x-1, y+1).0[0] as f32) +
                 1.0 * gray.get_pixel(x+1, y+1).0[0] as f32;
                
            let gy = 
                -(gray.get_pixel(x-1, y-1).0[0] as f32) +
                -2.0 * gray.get_pixel(x, y-1).0[0] as f32 +
                -(gray.get_pixel(x+1, y-1).0[0] as f32) +
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
    let contrast_factor = adj.contrast;
    
    // Saturation factor
    let sat_factor = adj.saturation;
    
    // Temperature adjustments
    let temp_r_add = if adj.temperature > 0.0 { adj.temperature * 25.5 } else { adj.temperature * 15.3 };
    let temp_b_sub = if adj.temperature > 0.0 { adj.temperature * 15.3 } else { adj.temperature * 25.5 };
    
    // Brightness addition
    let brightness_add = adj.brightness * 2.55;
    
    // Blacks adjustment (lift shadows)
    let blacks_add = adj.blacks * 25.5; // -1.0 to +1.0 -> -25.5 to +25.5
    
    // Whites adjustment (reduce highlights)
    let whites_mult = 1.0 - adj.whites * 0.1; // -1.0 to +1.0 -> 0.9 to 1.1
    
    // Shadows adjustment (gamma-like curve for shadows)
    let shadow_lift = adj.shadows * 0.5; // -1.0 to +1.0 -> -0.5 to +0.5
    
    // Highlights adjustment (compress highlights)
    let highlight_compress = adj.highlights * 0.7; // -1.0 to +1.0 -> -0.7 to +0.7
    
    // Tint adjustments
    let tint_r_add = if adj.tint > 0.0 { adj.tint * 12.75 } else { 0.0 };
    let tint_b_add = if adj.tint > 0.0 { adj.tint * 12.75 } else { 0.0 };
    let tint_g_sub = if adj.tint < 0.0 { -adj.tint * 20.4 } else { 0.0 };
    
    // Sharpening (simplified)
    let sharpen_strength = adj.sharpening * 0.5;
    
    // Process pixels in parallel for maximum CPU utilization
    let mut samples = img.as_flat_samples_mut();
    let raw_pixels = samples.as_mut_slice();
    let pixels_per_chunk = (raw_pixels.len() / num_cpus::get()).max(4); // 4 bytes per pixel (RGBA)
    
    raw_pixels.par_chunks_mut(pixels_per_chunk).for_each(|chunk| {
        for pixel in chunk.chunks_mut(4) {
            if pixel.len() < 4 { continue; } // Skip incomplete pixels
            
            let mut r = pixel[0] as f32;
            let mut g = pixel[1] as f32;
            let mut b = pixel[2] as f32;
            let a = pixel[3] as f32;
            
            // Apply exposure
            r *= exposure_mult;
            g *= exposure_mult;
            b *= exposure_mult;
            
            // Blacks adjustment (lift shadows)
            r += blacks_add;
            g += blacks_add;
            b += blacks_add;
            
            // Whites adjustment (reduce highlights)
            r *= whites_mult;
            g *= whites_mult;
            b *= whites_mult;
            
            // Shadows adjustment (gamma-like curve for shadows)
            if shadow_lift < 0.0 {
                let gamma = 1.0 - shadow_lift;
                r = r.powf(gamma);
                g = g.powf(gamma);
                b = b.powf(gamma);
            }
            
            // Highlights adjustment (compress highlights)
            if highlight_compress > 0.0 {
                let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
                let highlight_mask = ((luminance - 0.5f32) / 0.5f32).max(0.0).min(1.0);
                let compress = 1.0 - highlight_compress * highlight_mask;
                r *= compress;
                g *= compress;
                b *= compress;
            }
            
            // Brightness
            r += brightness_add;
            g += brightness_add;
            b += brightness_add;
            
            // Contrast
            r = ((r / 255.0 - 0.5) * contrast_factor + 0.5) * 255.0;
            g = ((g / 255.0 - 0.5) * contrast_factor + 0.5) * 255.0;
            b = ((b / 255.0 - 0.5) * contrast_factor + 0.5) * 255.0;
            
            // Temperature
            r += temp_r_add;
            b -= temp_b_sub;
            
            // Tint
            r += tint_r_add;
            g -= tint_g_sub;
            b += tint_b_add;
            
            // Saturation
            let gray = 0.299 * r + 0.587 * g + 0.114 * b;
            r = gray + (r - gray) * sat_factor;
            g = gray + (g - gray) * sat_factor;
            b = gray + (b - gray) * sat_factor;
            
            // Basic sharpening (simplified unsharp mask approximation)
            if sharpen_strength > 0.0 {
                let sharpened = r + (r - gray) * sharpen_strength;
                r = r + (sharpened - r) * sharpen_strength;
                let sharpened = g + (g - gray) * sharpen_strength;
                g = g + (sharpened - g) * sharpen_strength;
                let sharpened = b + (b - gray) * sharpen_strength;
                b = b + (sharpened - b) * sharpen_strength;
            }
            
            // Clamp values
            pixel[0] = r.clamp(0.0, 255.0) as u8;
            pixel[1] = g.clamp(0.0, 255.0) as u8;
            pixel[2] = b.clamp(0.0, 255.0) as u8;
            pixel[3] = a as u8; // Alpha unchanged
        }
    });
    
    DynamicImage::ImageRgba8(img)
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


