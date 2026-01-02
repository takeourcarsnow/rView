use crate::errors::{Result, ViewerError};
use image::DynamicImage;
use std::path::Path;

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
    if super::extensions::is_raw_file(path) {
        if let Ok(thumb) = load_raw_embedded_thumbnail(path, max_size) {
            return Ok(thumb);
        }

        // No embedded thumbnail available; as a fallback, attempt a full RAW decode and generate a thumbnail from it.
        // This is more expensive but ensures files (like some Lightroom-exported DNGs) still show a preview.
        log::warn!("No embedded thumbnail for {:?}; attempting full RAW decode to generate thumbnail", path);
        match super::loader::load_raw_image(path) {
            Ok(img) => return Ok(generate_thumbnail(&img, max_size)),
            Err(e) => {
                log::warn!("Full RAW decode fallback for thumbnail failed for {:?}: {}", path, e);
                return Err(ViewerError::ImageLoadError { path: path.to_path_buf(), message: "No embedded thumbnail for RAW file".to_string() });
            }
        }
    }

    let image = super::loader::load_image(path)?;
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