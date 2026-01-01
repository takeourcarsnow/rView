use crate::errors::{Result, ViewerError};
use image::{DynamicImage, ImageBuffer, RgbImage, Rgba, RgbaImage, GenericImageView};
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

// Film emulation parameters for realistic analog film simulation
#[derive(Debug, Clone, PartialEq)]
pub struct FilmEmulation {
    pub enabled: bool,
    pub is_bw: bool,                    // Whether this is a B&W film (converts color to mono)
    
    // Tone curve control points (shadows, midtones, highlights) - values 0.0 to 1.0
    pub tone_curve_shadows: f32,        // Lift/lower shadows (-1.0 to 1.0)
    pub tone_curve_midtones: f32,       // Adjust midtones (-1.0 to 1.0)
    pub tone_curve_highlights: f32,     // Compress/expand highlights (-1.0 to 1.0)
    
    // S-curve strength for contrast (film characteristic curve)
    pub s_curve_strength: f32,          // 0.0 to 1.0
    
    // Film grain simulation
    pub grain_amount: f32,              // 0.0 to 1.0 (intensity)
    pub grain_size: f32,                // 0.5 to 2.0 (1.0 = normal)
    pub grain_roughness: f32,           // 0.0 to 1.0 (organic variation)
    
    // Halation (light bloom around bright areas, characteristic of film)
    pub halation_amount: f32,           // 0.0 to 1.0
    pub halation_radius: f32,           // Spread of the halation effect
    pub halation_color: [f32; 3],       // RGB tint for halation (usually warm/red)
    
    // Color channel crossover/crosstalk (film layers interact)
    pub red_in_green: f32,              // -0.2 to 0.2
    pub red_in_blue: f32,               // -0.2 to 0.2
    pub green_in_red: f32,              // -0.2 to 0.2
    pub green_in_blue: f32,             // -0.2 to 0.2
    pub blue_in_red: f32,               // -0.2 to 0.2
    pub blue_in_green: f32,             // -0.2 to 0.2
    
    // Color response curves (per-channel gamma/lift)
    pub red_gamma: f32,                 // 0.8 to 1.2
    pub green_gamma: f32,               // 0.8 to 1.2
    pub blue_gamma: f32,                // 0.8 to 1.2
    
    // Black point and white point (film base density and max density)
    pub black_point: f32,               // 0.0 to 0.1 (raised blacks = faded look)
    pub white_point: f32,               // 0.9 to 1.0 (compressed highlights)
    
    // Color cast/tint in shadows and highlights
    pub shadow_tint: [f32; 3],          // RGB tint for shadows
    pub highlight_tint: [f32; 3],       // RGB tint for highlights
    
    // Vignette (natural lens falloff)
    pub vignette_amount: f32,           // 0.0 to 1.0
    pub vignette_softness: f32,         // 0.5 to 2.0
    
    // Film latitude (dynamic range compression)
    pub latitude: f32,                  // 0.0 to 1.0 (higher = more DR recovery)
}

impl Default for FilmEmulation {
    fn default() -> Self {
        Self {
            enabled: false,
            is_bw: false,
            tone_curve_shadows: 0.0,
            tone_curve_midtones: 0.0,
            tone_curve_highlights: 0.0,
            s_curve_strength: 0.0,
            grain_amount: 0.0,
            grain_size: 1.0,
            grain_roughness: 0.5,
            halation_amount: 0.0,
            halation_radius: 1.0,
            halation_color: [1.0, 0.3, 0.1], // Warm red/orange
            red_in_green: 0.0,
            red_in_blue: 0.0,
            green_in_red: 0.0,
            green_in_blue: 0.0,
            blue_in_red: 0.0,
            blue_in_green: 0.0,
            red_gamma: 1.0,
            green_gamma: 1.0,
            blue_gamma: 1.0,
            black_point: 0.0,
            white_point: 1.0,
            shadow_tint: [0.0, 0.0, 0.0],
            highlight_tint: [0.0, 0.0, 0.0],
            vignette_amount: 0.0,
            vignette_softness: 1.0,
            latitude: 0.0,
        }
    }
}

// Apply basic adjustments (non-destructive preview)
#[derive(Debug, Clone, PartialEq)]
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
    pub film: FilmEmulation, // Film emulation parameters
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
            film: FilmEmulation::default(),
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
        self.sharpening == 0.0 &&
        !self.film.enabled
    }

    pub fn apply_preset(&mut self, preset: FilmPreset) {
        *self = match preset {
            FilmPreset::None => ImageAdjustments::default(),
            
            // Kodak Portra 400 - Professional portrait film
            // Known for: Warm tones, excellent skin tones, subtle grain, wide latitude
            FilmPreset::Portra400 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.05,
                brightness: 3.0,
                saturation: 0.95,
                highlights: -0.15,
                shadows: 0.15,
                temperature: 0.08,
                tint: 0.03,
                blacks: 0.08,
                whites: -0.08,
                sharpening: 0.2,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: false,
                    tone_curve_shadows: 0.1,
                    tone_curve_midtones: 0.02,
                    tone_curve_highlights: -0.08,
                    s_curve_strength: 0.15,
                    grain_amount: 0.12,
                    grain_size: 1.0,
                    grain_roughness: 0.4,
                    halation_amount: 0.05,
                    halation_radius: 1.2,
                    halation_color: [1.0, 0.4, 0.2],
                    red_in_green: 0.02,
                    red_in_blue: 0.0,
                    green_in_red: 0.01,
                    green_in_blue: 0.01,
                    blue_in_red: 0.0,
                    blue_in_green: 0.02,
                    red_gamma: 0.98,
                    green_gamma: 1.0,
                    blue_gamma: 1.02,
                    black_point: 0.02,
                    white_point: 0.98,
                    shadow_tint: [0.02, 0.01, 0.0],
                    highlight_tint: [0.02, 0.01, -0.01],
                    vignette_amount: 0.08,
                    vignette_softness: 1.5,
                    latitude: 0.7,
                },
            },
            
            // Kodak Portra 160 - Fine grain portrait film  
            // Known for: Very fine grain, softer contrast, excellent for overexposure
            FilmPreset::Portra160 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.0,
                brightness: 5.0,
                saturation: 0.92,
                highlights: -0.1,
                shadows: 0.18,
                temperature: 0.05,
                tint: 0.02,
                blacks: 0.05,
                whites: -0.05,
                sharpening: 0.15,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: false,
                    tone_curve_shadows: 0.12,
                    tone_curve_midtones: 0.03,
                    tone_curve_highlights: -0.06,
                    s_curve_strength: 0.1,
                    grain_amount: 0.06,
                    grain_size: 0.8,
                    grain_roughness: 0.35,
                    halation_amount: 0.03,
                    halation_radius: 1.0,
                    halation_color: [1.0, 0.45, 0.25],
                    red_in_green: 0.015,
                    red_in_blue: 0.0,
                    green_in_red: 0.01,
                    green_in_blue: 0.01,
                    blue_in_red: 0.0,
                    blue_in_green: 0.015,
                    red_gamma: 0.99,
                    green_gamma: 1.0,
                    blue_gamma: 1.01,
                    black_point: 0.015,
                    white_point: 0.99,
                    shadow_tint: [0.015, 0.008, 0.0],
                    highlight_tint: [0.015, 0.008, -0.005],
                    vignette_amount: 0.06,
                    vignette_softness: 1.6,
                    latitude: 0.8,
                },
            },
            
            // Kodak Portra 800 - High-speed portrait film
            // Known for: More grain, warm tones, good in low light
            FilmPreset::Portra800 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.08,
                brightness: 2.0,
                saturation: 0.98,
                highlights: -0.18,
                shadows: 0.1,
                temperature: 0.12,
                tint: 0.05,
                blacks: 0.1,
                whites: -0.1,
                sharpening: 0.25,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: false,
                    tone_curve_shadows: 0.08,
                    tone_curve_midtones: 0.03,
                    tone_curve_highlights: -0.1,
                    s_curve_strength: 0.18,
                    grain_amount: 0.22,
                    grain_size: 1.15,
                    grain_roughness: 0.5,
                    halation_amount: 0.06,
                    halation_radius: 1.3,
                    halation_color: [1.0, 0.38, 0.18],
                    red_in_green: 0.025,
                    red_in_blue: 0.01,
                    green_in_red: 0.015,
                    green_in_blue: 0.015,
                    blue_in_red: 0.005,
                    blue_in_green: 0.025,
                    red_gamma: 0.97,
                    green_gamma: 1.0,
                    blue_gamma: 1.03,
                    black_point: 0.025,
                    white_point: 0.97,
                    shadow_tint: [0.025, 0.012, 0.0],
                    highlight_tint: [0.025, 0.012, -0.01],
                    vignette_amount: 0.1,
                    vignette_softness: 1.4,
                    latitude: 0.65,
                },
            },
            
            // Kodak T-Max 400 - Professional B&W film
            // Known for: Fine grain for speed, high acutance, modern T-grain
            FilmPreset::TMax400 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.2,
                brightness: 0.0,
                saturation: 1.0, // Will be converted to B&W by film emulation
                highlights: 0.05,
                shadows: -0.05,
                temperature: 0.0,
                tint: 0.0,
                blacks: -0.05,
                whites: 0.05,
                sharpening: 0.5,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: true,
                    tone_curve_shadows: -0.05,
                    tone_curve_midtones: 0.05,
                    tone_curve_highlights: 0.08,
                    s_curve_strength: 0.25,
                    grain_amount: 0.15,
                    grain_size: 0.9,
                    grain_roughness: 0.45,
                    halation_amount: 0.02,
                    halation_radius: 0.8,
                    halation_color: [0.8, 0.8, 0.8],
                    red_in_green: 0.0,
                    red_in_blue: 0.0,
                    green_in_red: 0.0,
                    green_in_blue: 0.0,
                    blue_in_red: 0.0,
                    blue_in_green: 0.0,
                    red_gamma: 1.0,
                    green_gamma: 1.0,
                    blue_gamma: 1.0,
                    black_point: 0.01,
                    white_point: 0.99,
                    shadow_tint: [0.0, 0.0, 0.0],
                    highlight_tint: [0.0, 0.0, 0.0],
                    vignette_amount: 0.05,
                    vignette_softness: 1.3,
                    latitude: 0.6,
                },
            },
            
            // Kodak T-Max 100 - Ultra fine grain B&W
            // Known for: Extremely fine grain, high resolution, smooth tones
            FilmPreset::TMax100 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.15,
                brightness: 2.0,
                saturation: 1.0,
                highlights: 0.08,
                shadows: -0.08,
                temperature: 0.0,
                tint: 0.0,
                blacks: -0.08,
                whites: 0.08,
                sharpening: 0.4,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: true,
                    tone_curve_shadows: -0.03,
                    tone_curve_midtones: 0.03,
                    tone_curve_highlights: 0.06,
                    s_curve_strength: 0.2,
                    grain_amount: 0.08,
                    grain_size: 0.7,
                    grain_roughness: 0.35,
                    halation_amount: 0.015,
                    halation_radius: 0.7,
                    halation_color: [0.85, 0.85, 0.85],
                    red_in_green: 0.0,
                    red_in_blue: 0.0,
                    green_in_red: 0.0,
                    green_in_blue: 0.0,
                    blue_in_red: 0.0,
                    blue_in_green: 0.0,
                    red_gamma: 1.0,
                    green_gamma: 1.0,
                    blue_gamma: 1.0,
                    black_point: 0.008,
                    white_point: 0.995,
                    shadow_tint: [0.0, 0.0, 0.0],
                    highlight_tint: [0.0, 0.0, 0.0],
                    vignette_amount: 0.04,
                    vignette_softness: 1.4,
                    latitude: 0.55,
                },
            },
            
            // Fujifilm Provia 100F - Professional slide film
            // Known for: Neutral colors, fine grain, accurate reproduction
            FilmPreset::Provia100 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.15,
                brightness: 0.0,
                saturation: 1.1,
                highlights: -0.08,
                shadows: 0.05,
                temperature: 0.03,
                tint: 0.02,
                blacks: 0.02,
                whites: -0.03,
                sharpening: 0.35,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: false,
                    tone_curve_shadows: 0.03,
                    tone_curve_midtones: 0.0,
                    tone_curve_highlights: -0.05,
                    s_curve_strength: 0.22,
                    grain_amount: 0.08,
                    grain_size: 0.85,
                    grain_roughness: 0.4,
                    halation_amount: 0.02,
                    halation_radius: 0.9,
                    halation_color: [0.9, 0.5, 0.3],
                    red_in_green: 0.01,
                    red_in_blue: 0.005,
                    green_in_red: 0.01,
                    green_in_blue: 0.01,
                    blue_in_red: 0.005,
                    blue_in_green: 0.01,
                    red_gamma: 1.0,
                    green_gamma: 1.0,
                    blue_gamma: 0.99,
                    black_point: 0.01,
                    white_point: 0.99,
                    shadow_tint: [0.005, 0.0, 0.005],
                    highlight_tint: [0.0, 0.0, 0.0],
                    vignette_amount: 0.05,
                    vignette_softness: 1.5,
                    latitude: 0.4,
                },
            },
            
            // Fujifilm Astia 100F - Soft portrait slide film
            // Known for: Soft contrast, pleasing skin tones, subtle colors
            FilmPreset::Astia100 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.08,
                brightness: 3.0,
                saturation: 1.05,
                highlights: -0.12,
                shadows: 0.1,
                temperature: 0.02,
                tint: 0.01,
                blacks: 0.05,
                whites: -0.05,
                sharpening: 0.25,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: false,
                    tone_curve_shadows: 0.08,
                    tone_curve_midtones: 0.02,
                    tone_curve_highlights: -0.06,
                    s_curve_strength: 0.15,
                    grain_amount: 0.07,
                    grain_size: 0.8,
                    grain_roughness: 0.38,
                    halation_amount: 0.025,
                    halation_radius: 1.0,
                    halation_color: [0.95, 0.55, 0.35],
                    red_in_green: 0.015,
                    red_in_blue: 0.005,
                    green_in_red: 0.01,
                    green_in_blue: 0.01,
                    blue_in_red: 0.005,
                    blue_in_green: 0.015,
                    red_gamma: 0.99,
                    green_gamma: 1.0,
                    blue_gamma: 1.0,
                    black_point: 0.012,
                    white_point: 0.99,
                    shadow_tint: [0.01, 0.005, 0.01],
                    highlight_tint: [0.005, 0.0, 0.0],
                    vignette_amount: 0.06,
                    vignette_softness: 1.6,
                    latitude: 0.45,
                },
            },
            
            // Ilford HP5 Plus 400 - Classic B&W film
            // Known for: Wide latitude, punchy contrast, classic grain structure
            FilmPreset::Hp5 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.25,
                brightness: -2.0,
                saturation: 1.0,
                highlights: 0.1,
                shadows: -0.1,
                temperature: 0.0,
                tint: 0.0,
                blacks: -0.1,
                whites: 0.1,
                sharpening: 0.6,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: true,
                    tone_curve_shadows: -0.08,
                    tone_curve_midtones: 0.08,
                    tone_curve_highlights: 0.12,
                    s_curve_strength: 0.3,
                    grain_amount: 0.25,
                    grain_size: 1.1,
                    grain_roughness: 0.6,
                    halation_amount: 0.03,
                    halation_radius: 1.0,
                    halation_color: [0.75, 0.75, 0.75],
                    red_in_green: 0.0,
                    red_in_blue: 0.0,
                    green_in_red: 0.0,
                    green_in_blue: 0.0,
                    blue_in_red: 0.0,
                    blue_in_green: 0.0,
                    red_gamma: 1.0,
                    green_gamma: 1.0,
                    blue_gamma: 1.0,
                    black_point: 0.015,
                    white_point: 0.98,
                    shadow_tint: [0.0, 0.0, 0.0],
                    highlight_tint: [0.0, 0.0, 0.0],
                    vignette_amount: 0.08,
                    vignette_softness: 1.2,
                    latitude: 0.7,
                },
            },
            
            // Fujifilm Velvia 50 - Vivid slide film
            // Known for: Extremely saturated colors, high contrast, punchy
            FilmPreset::Velvia50 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.25,
                brightness: -3.0,
                saturation: 1.35,
                highlights: -0.2,
                shadows: 0.15,
                temperature: 0.1,
                tint: 0.08,
                blacks: 0.1,
                whites: -0.12,
                sharpening: 0.5,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: false,
                    tone_curve_shadows: 0.05,
                    tone_curve_midtones: -0.03,
                    tone_curve_highlights: -0.1,
                    s_curve_strength: 0.35,
                    grain_amount: 0.06,
                    grain_size: 0.75,
                    grain_roughness: 0.35,
                    halation_amount: 0.04,
                    halation_radius: 1.1,
                    halation_color: [1.0, 0.35, 0.15],
                    red_in_green: -0.02,
                    red_in_blue: -0.01,
                    green_in_red: -0.01,
                    green_in_blue: 0.02,
                    blue_in_red: -0.01,
                    blue_in_green: -0.02,
                    red_gamma: 0.95,
                    green_gamma: 0.98,
                    blue_gamma: 1.02,
                    black_point: 0.008,
                    white_point: 0.98,
                    shadow_tint: [0.02, 0.0, 0.01],
                    highlight_tint: [0.01, 0.005, -0.01],
                    vignette_amount: 0.07,
                    vignette_softness: 1.4,
                    latitude: 0.35,
                },
            },
            
            // Fujifilm Velvia 100 - Vivid slide film (more latitude)
            // Known for: Saturated but slightly less extreme than Velvia 50
            FilmPreset::Velvia100 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.2,
                brightness: -2.0,
                saturation: 1.28,
                highlights: -0.15,
                shadows: 0.12,
                temperature: 0.08,
                tint: 0.06,
                blacks: 0.08,
                whites: -0.1,
                sharpening: 0.45,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: false,
                    tone_curve_shadows: 0.06,
                    tone_curve_midtones: -0.02,
                    tone_curve_highlights: -0.08,
                    s_curve_strength: 0.3,
                    grain_amount: 0.07,
                    grain_size: 0.8,
                    grain_roughness: 0.38,
                    halation_amount: 0.035,
                    halation_radius: 1.05,
                    halation_color: [1.0, 0.38, 0.18],
                    red_in_green: -0.015,
                    red_in_blue: -0.008,
                    green_in_red: -0.008,
                    green_in_blue: 0.015,
                    blue_in_red: -0.008,
                    blue_in_green: -0.015,
                    red_gamma: 0.96,
                    green_gamma: 0.98,
                    blue_gamma: 1.01,
                    black_point: 0.01,
                    white_point: 0.985,
                    shadow_tint: [0.015, 0.0, 0.008],
                    highlight_tint: [0.008, 0.004, -0.008],
                    vignette_amount: 0.06,
                    vignette_softness: 1.45,
                    latitude: 0.4,
                },
            },
            
            // Kodak Gold 200 - Consumer color negative
            // Known for: Warm tones, nostalgic look, moderate saturation
            FilmPreset::KodakGold200 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.1,
                brightness: 4.0,
                saturation: 1.08,
                highlights: -0.08,
                shadows: 0.08,
                temperature: 0.15,
                tint: 0.05,
                blacks: 0.05,
                whites: -0.05,
                sharpening: 0.3,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: false,
                    tone_curve_shadows: 0.1,
                    tone_curve_midtones: 0.03,
                    tone_curve_highlights: -0.06,
                    s_curve_strength: 0.18,
                    grain_amount: 0.15,
                    grain_size: 1.05,
                    grain_roughness: 0.55,
                    halation_amount: 0.05,
                    halation_radius: 1.25,
                    halation_color: [1.0, 0.45, 0.2],
                    red_in_green: 0.025,
                    red_in_blue: 0.01,
                    green_in_red: 0.02,
                    green_in_blue: 0.015,
                    blue_in_red: 0.005,
                    blue_in_green: 0.02,
                    red_gamma: 0.96,
                    green_gamma: 0.99,
                    blue_gamma: 1.04,
                    black_point: 0.02,
                    white_point: 0.975,
                    shadow_tint: [0.03, 0.015, 0.0],
                    highlight_tint: [0.02, 0.01, -0.01],
                    vignette_amount: 0.1,
                    vignette_softness: 1.3,
                    latitude: 0.6,
                },
            },
            
            // Fujifilm 400H - Professional portrait film (discontinued)
            // Known for: Pastel colors, lifted shadows, creamy skin tones
            FilmPreset::Fuji400H => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.02,
                brightness: 5.0,
                saturation: 0.9,
                highlights: -0.12,
                shadows: 0.2,
                temperature: 0.05,
                tint: 0.03,
                blacks: 0.12,
                whites: -0.08,
                sharpening: 0.2,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: false,
                    tone_curve_shadows: 0.15,
                    tone_curve_midtones: 0.05,
                    tone_curve_highlights: -0.05,
                    s_curve_strength: 0.1,
                    grain_amount: 0.12,
                    grain_size: 0.95,
                    grain_roughness: 0.45,
                    halation_amount: 0.04,
                    halation_radius: 1.15,
                    halation_color: [0.9, 0.5, 0.35],
                    red_in_green: 0.02,
                    red_in_blue: 0.015,
                    green_in_red: 0.015,
                    green_in_blue: 0.02,
                    blue_in_red: 0.01,
                    blue_in_green: 0.025,
                    red_gamma: 0.98,
                    green_gamma: 1.0,
                    blue_gamma: 1.02,
                    black_point: 0.03,
                    white_point: 0.98,
                    shadow_tint: [0.01, 0.015, 0.02],
                    highlight_tint: [0.01, 0.005, 0.0],
                    vignette_amount: 0.07,
                    vignette_softness: 1.5,
                    latitude: 0.75,
                },
            },
            
            // Kodak Tri-X 400 - Classic B&W film
            // Known for: Iconic grain, high contrast, great tonal range
            FilmPreset::TriX400 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.3,
                brightness: -3.0,
                saturation: 1.0,
                highlights: 0.12,
                shadows: -0.12,
                temperature: 0.0,
                tint: 0.0,
                blacks: -0.12,
                whites: 0.12,
                sharpening: 0.7,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: true,
                    tone_curve_shadows: -0.1,
                    tone_curve_midtones: 0.1,
                    tone_curve_highlights: 0.15,
                    s_curve_strength: 0.35,
                    grain_amount: 0.3,
                    grain_size: 1.2,
                    grain_roughness: 0.65,
                    halation_amount: 0.035,
                    halation_radius: 1.1,
                    halation_color: [0.7, 0.7, 0.7],
                    red_in_green: 0.0,
                    red_in_blue: 0.0,
                    green_in_red: 0.0,
                    green_in_blue: 0.0,
                    blue_in_red: 0.0,
                    blue_in_green: 0.0,
                    red_gamma: 1.0,
                    green_gamma: 1.0,
                    blue_gamma: 1.0,
                    black_point: 0.012,
                    white_point: 0.975,
                    shadow_tint: [0.0, 0.0, 0.0],
                    highlight_tint: [0.0, 0.0, 0.0],
                    vignette_amount: 0.1,
                    vignette_softness: 1.15,
                    latitude: 0.65,
                },
            },
            
            // Ilford Delta 3200 - High speed B&W film
            // Known for: Very coarse grain, excellent low light, gritty look
            FilmPreset::Delta3200 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.35,
                brightness: -5.0,
                saturation: 1.0,
                highlights: 0.15,
                shadows: -0.15,
                temperature: 0.0,
                tint: 0.0,
                blacks: -0.15,
                whites: 0.15,
                sharpening: 0.8,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: true,
                    tone_curve_shadows: -0.12,
                    tone_curve_midtones: 0.12,
                    tone_curve_highlights: 0.18,
                    s_curve_strength: 0.4,
                    grain_amount: 0.45,
                    grain_size: 1.5,
                    grain_roughness: 0.75,
                    halation_amount: 0.04,
                    halation_radius: 1.3,
                    halation_color: [0.65, 0.65, 0.65],
                    red_in_green: 0.0,
                    red_in_blue: 0.0,
                    green_in_red: 0.0,
                    green_in_blue: 0.0,
                    blue_in_red: 0.0,
                    blue_in_green: 0.0,
                    red_gamma: 1.0,
                    green_gamma: 1.0,
                    blue_gamma: 1.0,
                    black_point: 0.02,
                    white_point: 0.97,
                    shadow_tint: [0.0, 0.0, 0.0],
                    highlight_tint: [0.0, 0.0, 0.0],
                    vignette_amount: 0.12,
                    vignette_softness: 1.1,
                    latitude: 0.55,
                },
            },
            
            // Kodak Ektar 100 - Fine grain color negative
            // Known for: Extremely fine grain, vivid colors, high saturation
            FilmPreset::Ektar100 => ImageAdjustments {
                exposure: 0.0,
                contrast: 1.18,
                brightness: 0.0,
                saturation: 1.25,
                highlights: -0.1,
                shadows: 0.08,
                temperature: 0.05,
                tint: 0.03,
                blacks: 0.03,
                whites: -0.05,
                sharpening: 0.4,
                film: FilmEmulation {
                    enabled: true,
                    is_bw: false,
                    tone_curve_shadows: 0.04,
                    tone_curve_midtones: -0.02,
                    tone_curve_highlights: -0.08,
                    s_curve_strength: 0.25,
                    grain_amount: 0.05,
                    grain_size: 0.7,
                    grain_roughness: 0.3,
                    halation_amount: 0.025,
                    halation_radius: 0.9,
                    halation_color: [1.0, 0.4, 0.2],
                    red_in_green: -0.01,
                    red_in_blue: -0.005,
                    green_in_red: -0.005,
                    green_in_blue: 0.01,
                    blue_in_red: -0.005,
                    blue_in_green: -0.01,
                    red_gamma: 0.97,
                    green_gamma: 0.99,
                    blue_gamma: 1.02,
                    black_point: 0.008,
                    white_point: 0.99,
                    shadow_tint: [0.01, 0.005, 0.0],
                    highlight_tint: [0.005, 0.002, -0.005],
                    vignette_amount: 0.05,
                    vignette_softness: 1.5,
                    latitude: 0.5,
                },
            },
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilmPreset {
    None,
    Portra400,
    Portra160,
    Portra800,
    TMax400,
    TMax100,
    Provia100,
    Astia100,
    Hp5,
    Velvia50,
    Velvia100,
    KodakGold200,
    Fuji400H,
    TriX400,
    Delta3200,
    Ektar100,
}

impl FilmPreset {
    pub fn name(&self) -> &'static str {
        match self {
            FilmPreset::None => "None",
            FilmPreset::Portra400 => "Portra 400",
            FilmPreset::Portra160 => "Portra 160",
            FilmPreset::Portra800 => "Portra 800",
            FilmPreset::TMax400 => "T-Max 400",
            FilmPreset::TMax100 => "T-Max 100",
            FilmPreset::Provia100 => "Provia 100",
            FilmPreset::Astia100 => "Astia 100",
            FilmPreset::Hp5 => "HP5 Plus",
            FilmPreset::Velvia50 => "Velvia 50",
            FilmPreset::Velvia100 => "Velvia 100",
            FilmPreset::KodakGold200 => "Kodak Gold 200",
            FilmPreset::Fuji400H => "Fuji 400H",
            FilmPreset::TriX400 => "Tri-X 400",
            FilmPreset::Delta3200 => "Delta 3200",
            FilmPreset::Ektar100 => "Ektar 100",
        }
    }

    pub fn all() -> &'static [FilmPreset] {
        &[
            FilmPreset::None,
            FilmPreset::Portra400,
            FilmPreset::Portra160,
            FilmPreset::Portra800,
            FilmPreset::TMax400,
            FilmPreset::TMax100,
            FilmPreset::Provia100,
            FilmPreset::Astia100,
            FilmPreset::Hp5,
            FilmPreset::Velvia50,
            FilmPreset::Velvia100,
            FilmPreset::KodakGold200,
            FilmPreset::Fuji400H,
            FilmPreset::TriX400,
            FilmPreset::Delta3200,
            FilmPreset::Ektar100,
        ]
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
    
    // Film emulation parameters
    let film = &adj.film;
    let film_enabled = film.enabled;
    
    // Pre-generate grain texture for consistent grain pattern
    // Using a simple hash-based pseudo-random for reproducibility
    let grain_seed = 12345u64;
    
    // Process pixels in parallel for maximum CPU utilization
    let mut samples = img.as_flat_samples_mut();
    let raw_pixels = samples.as_mut_slice();
    let pixels_per_chunk = (raw_pixels.len() / num_cpus::get()).max(4); // 4 bytes per pixel (RGBA)
    
    // Calculate center for vignette
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let max_dist = (center_x * center_x + center_y * center_y).sqrt();
    
    raw_pixels.par_chunks_mut(pixels_per_chunk).enumerate().for_each(|(chunk_idx, chunk)| {
        let chunk_start = chunk_idx * pixels_per_chunk;
        
        for (local_idx, pixel) in chunk.chunks_mut(4).enumerate() {
            if pixel.len() < 4 { continue; } // Skip incomplete pixels
            
            let pixel_idx = chunk_start / 4 + local_idx;
            let px = (pixel_idx % width as usize) as f32;
            let py = (pixel_idx / width as usize) as f32;
            
            let mut r = pixel[0] as f32 / 255.0;
            let mut g = pixel[1] as f32 / 255.0;
            let mut b = pixel[2] as f32 / 255.0;
            let a = pixel[3] as f32;
            
            // ============ FILM EMULATION (applied first for characteristic curve) ============
            if film_enabled {
                // B&W conversion for monochrome films (uses proper luminance weights)
                if film.is_bw {
                    // Use film-like spectral sensitivity (red-sensitive for classic B&W look)
                    let luminance = 0.30 * r + 0.59 * g + 0.11 * b;
                    r = luminance;
                    g = luminance;
                    b = luminance;
                }
                
                // Color channel crossover/crosstalk (film layer interaction)
                if !film.is_bw {
                    let orig_r = r;
                    let orig_g = g;
                    let orig_b = b;
                    r = orig_r + orig_g * film.green_in_red + orig_b * film.blue_in_red;
                    g = orig_g + orig_r * film.red_in_green + orig_b * film.blue_in_green;
                    b = orig_b + orig_r * film.red_in_blue + orig_g * film.green_in_blue;
                }
                
                // Per-channel gamma (color response curves)
                r = r.max(0.0).powf(film.red_gamma);
                g = g.max(0.0).powf(film.green_gamma);
                b = b.max(0.0).powf(film.blue_gamma);
                
                // Film latitude (dynamic range compression - recover shadows/highlights)
                if film.latitude > 0.0 {
                    let latitude_factor = film.latitude * 0.5;
                    // Soft-clip highlights
                    r = r / (1.0 + r * latitude_factor);
                    g = g / (1.0 + g * latitude_factor);
                    b = b / (1.0 + b * latitude_factor);
                    // Compensate for compression
                    let comp = 1.0 + latitude_factor * 0.5;
                    r *= comp;
                    g *= comp;
                    b *= comp;
                }
                
                // Tone curve (S-curve for film characteristic curve)
                if film.s_curve_strength > 0.0 {
                    let s = film.s_curve_strength;
                    // Apply sigmoid-like S-curve
                    r = apply_s_curve(r, s);
                    g = apply_s_curve(g, s);
                    b = apply_s_curve(b, s);
                }
                
                // Tone curve control points (shadows, midtones, highlights)
                r = apply_tone_curve(r, film.tone_curve_shadows, film.tone_curve_midtones, film.tone_curve_highlights);
                g = apply_tone_curve(g, film.tone_curve_shadows, film.tone_curve_midtones, film.tone_curve_highlights);
                b = apply_tone_curve(b, film.tone_curve_shadows, film.tone_curve_midtones, film.tone_curve_highlights);
                
                // Black point and white point (film base density)
                let bp = film.black_point;
                let wp = film.white_point;
                let range = wp - bp;
                if range > 0.01 {
                    r = bp + r * range;
                    g = bp + g * range;
                    b = bp + b * range;
                }
                
                // Shadow and highlight tinting
                let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
                let shadow_amount = (1.0 - luminance * 2.0).max(0.0).min(1.0);
                let highlight_amount = ((luminance - 0.5) * 2.0).max(0.0).min(1.0);
                
                r += film.shadow_tint[0] * shadow_amount + film.highlight_tint[0] * highlight_amount;
                g += film.shadow_tint[1] * shadow_amount + film.highlight_tint[1] * highlight_amount;
                b += film.shadow_tint[2] * shadow_amount + film.highlight_tint[2] * highlight_amount;
            }
            
            // Convert to 0-255 range for standard adjustments
            r *= 255.0;
            g *= 255.0;
            b *= 255.0;
            
            // ============ STANDARD ADJUSTMENTS ============
            
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
                r = r.max(0.0).powf(gamma);
                g = g.max(0.0).powf(gamma);
                b = b.max(0.0).powf(gamma);
            }
            
            // Highlights adjustment (compress highlights)
            if highlight_compress > 0.0 {
                let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
                let highlight_mask = ((luminance - 127.5) / 127.5).max(0.0).min(1.0);
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
            
            // Saturation (skip for B&W film)
            if !film_enabled || !film.is_bw {
                let gray = 0.299 * r + 0.587 * g + 0.114 * b;
                r = gray + (r - gray) * sat_factor;
                g = gray + (g - gray) * sat_factor;
                b = gray + (b - gray) * sat_factor;
            }
            
            // Basic sharpening (simplified unsharp mask approximation)
            if sharpen_strength > 0.0 {
                let gray = 0.299 * r + 0.587 * g + 0.114 * b;
                let sharpened = r + (r - gray) * sharpen_strength;
                r = r + (sharpened - r) * sharpen_strength;
                let sharpened = g + (g - gray) * sharpen_strength;
                g = g + (sharpened - g) * sharpen_strength;
                let sharpened = b + (b - gray) * sharpen_strength;
                b = b + (sharpened - b) * sharpen_strength;
            }
            
            // ============ FILM POST-PROCESSING ============
            if film_enabled {
                // Vignette (natural lens falloff)
                if film.vignette_amount > 0.0 {
                    let dx = px - center_x;
                    let dy = py - center_y;
                    let dist = (dx * dx + dy * dy).sqrt() / max_dist;
                    let vignette = 1.0 - film.vignette_amount * (dist / film.vignette_softness).powf(2.0);
                    let vignette = vignette.max(0.0).min(1.0);
                    r *= vignette;
                    g *= vignette;
                    b *= vignette;
                }
                
                // Film grain (applied last for realistic appearance)
                if film.grain_amount > 0.0 {
                    // Generate pseudo-random grain based on pixel position
                    let grain = generate_film_grain(
                        px as u32, 
                        py as u32, 
                        grain_seed,
                        film.grain_size,
                        film.grain_roughness
                    );
                    
                    // Grain intensity varies with luminance (more visible in midtones)
                    let lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
                    let grain_mask = 4.0 * lum * (1.0 - lum); // Peaks at midtones
                    let grain_strength = film.grain_amount * 255.0 * 0.15 * grain_mask;
                    
                    r += grain * grain_strength;
                    g += grain * grain_strength;
                    b += grain * grain_strength;
                }
                
                // Halation (subtle glow around bright areas)
                // Note: Full halation requires multi-pass blur, this is a simplified version
                if film.halation_amount > 0.0 {
                    let luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
                    let halation_mask = ((luminance - 0.7) / 0.3).max(0.0).min(1.0);
                    let halation_strength = film.halation_amount * halation_mask * 30.0;
                    r += film.halation_color[0] * halation_strength;
                    g += film.halation_color[1] * halation_strength;
                    b += film.halation_color[2] * halation_strength;
                }
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

/// Apply S-curve contrast enhancement (film characteristic curve)
#[inline]
fn apply_s_curve(x: f32, strength: f32) -> f32 {
    // Attempt to simulate Hurter-Driffield (H&D) curve
    let x = x.clamp(0.0, 1.0);
    let midpoint = 0.5;
    let steepness = 1.0 + strength * 3.0;
    
    // Sigmoid function centered at midpoint
    let sigmoid = 1.0 / (1.0 + (-steepness * (x - midpoint)).exp());
    // Normalize to 0-1 range
    let min_sig = 1.0 / (1.0 + (steepness * midpoint).exp());
    let max_sig = 1.0 / (1.0 + (-steepness * (1.0 - midpoint)).exp());
    
    let normalized = (sigmoid - min_sig) / (max_sig - min_sig);
    // Blend between linear and S-curve based on strength
    x * (1.0 - strength) + normalized * strength
}

/// Apply tone curve adjustments for shadows, midtones, and highlights
#[inline]
fn apply_tone_curve(x: f32, shadows: f32, midtones: f32, highlights: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    
    // Shadow region (0-0.33)
    // Midtone region (0.33-0.66)  
    // Highlight region (0.66-1.0)
    
    let shadow_weight = (1.0 - x * 3.0).max(0.0).min(1.0);
    let highlight_weight = ((x - 0.66) * 3.0).max(0.0).min(1.0);
    let midtone_weight = 1.0 - shadow_weight - highlight_weight;
    
    // Apply adjustments weighted by region
    let adjustment = shadows * shadow_weight * 0.15 
                   + midtones * midtone_weight * 0.1
                   + highlights * highlight_weight * 0.15;
    
    (x + adjustment).clamp(0.0, 1.0)
}

/// Generate film grain using pseudo-random noise
#[inline]
fn generate_film_grain(x: u32, y: u32, seed: u64, size: f32, roughness: f32) -> f32 {
    // Scale coordinates by grain size
    let scale = 1.0 / size;
    let sx = (x as f32 * scale) as u32;
    let sy = (y as f32 * scale) as u32;
    
    // Simple hash function for pseudo-random values
    let mut hash = seed;
    hash ^= sx as u64;
    hash = hash.wrapping_mul(0x517cc1b727220a95);
    hash ^= sy as u64;
    hash = hash.wrapping_mul(0x517cc1b727220a95);
    hash ^= hash >> 32;
    
    // Convert to -1 to 1 range
    let noise = (hash as f32 / u64::MAX as f32) * 2.0 - 1.0;
    
    // Add roughness variation (multi-octave noise approximation)
    let mut rough_noise = noise;
    if roughness > 0.0 {
        hash = hash.wrapping_mul(0x517cc1b727220a95);
        let noise2 = (hash as f32 / u64::MAX as f32) * 2.0 - 1.0;
        rough_noise = noise * (1.0 - roughness * 0.5) + noise2 * roughness * 0.5;
    }
    
    rough_noise
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


