use crate::errors::{Result, ViewerError};
use image::{DynamicImage, GenericImageView, ImageBuffer, RgbImage};
use rayon::ThreadPoolBuilder;
use std::path::Path;

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

pub fn load_image(path: &Path) -> Result<DynamicImage> {
    if !path.exists() {
        return Err(ViewerError::FileNotFound {
            path: path.to_path_buf(),
        });
    }

    // Check file size to prevent loading extremely large images that could cause crashes
    if let Ok(metadata) = std::fs::metadata(path) {
        let file_size = metadata.len();
        // Limit to 500MB to prevent memory issues
        if file_size > 500 * 1024 * 1024 {
            return Err(ViewerError::ImageLoadError {
                path: path.to_path_buf(),
                message: format!(
                    "File too large: {}MB (max 500MB)",
                    file_size / (1024 * 1024)
                ),
            });
        }
    }

    crate::profiler::with_profiler(|p| p.start_timer("image_load"));
    let result = if super::extensions::is_raw_file(path) {
        load_raw_image(path)
    } else {
        load_standard_image(path)
    };
    crate::profiler::with_profiler(|p| p.end_timer("image_load"));

    // Check image dimensions to prevent creating textures that are too large
    if let Ok(img) = &result {
        let (width, height) = img.dimensions();
        let megapixels = (width as u64 * height as u64) / 1_000_000;
        if megapixels > 100 {
            return Err(ViewerError::ImageLoadError {
                path: path.to_path_buf(),
                message: format!("Image too large: {}MP (max 100MP)", megapixels),
            });
        }
    }

    result
}

fn load_standard_image(path: &Path) -> Result<DynamicImage> {
    // For large files (>50MB), use memory mapping to avoid loading entire file into RAM
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.len() > 50 * 1024 * 1024 {
            // 50MB threshold
            return load_image_memory_mapped(path);
        }
    }

    image::open(path).map_err(|e| ViewerError::ImageLoadError {
        path: path.to_path_buf(),
        message: e.to_string(),
    })
}

fn load_image_memory_mapped(path: &Path) -> Result<DynamicImage> {
    use memmap2::Mmap;
    use std::fs::File;

    let file = File::open(path).map_err(|e| ViewerError::ImageLoadError {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;

    let mmap = unsafe { Mmap::map(&file) }.map_err(|e| ViewerError::ImageLoadError {
        path: path.to_path_buf(),
        message: format!("Memory mapping failed: {}", e),
    })?;

    image::load_from_memory(&mmap).map_err(|e| ViewerError::ImageLoadError {
        path: path.to_path_buf(),
        message: e.to_string(),
    })
}

pub fn load_raw_image(path: &Path) -> Result<DynamicImage> {
    log::info!("Loading RAW image: {:?}", path);
    RAW_PROCESSING_POOL.install(|| {
        // Wrap in catch_unwind to handle panics in rawloader
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            log::debug!("Decoding RAW file: {:?}", path);
            // Attempt to decode with rawloader. If it fails, try a safe fallback: extract an embedded JPEG preview
            // and use it as the image so that DNGs that rawloader can't decode still display.
            let raw = match rawloader::decode_file(path) {
                Ok(r) => r,
                Err(e) => {
                    log::error!("rawloader decode_file failed for {:?}: {}", path, e);
                    // Try to extract embedded thumbnail as a fallback
                    match super::thumbnail::load_raw_embedded_thumbnail(path, 16384) {
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
