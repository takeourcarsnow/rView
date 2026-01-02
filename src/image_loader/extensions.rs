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

use std::path::Path;

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