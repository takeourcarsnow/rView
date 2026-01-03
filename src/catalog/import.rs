use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;

use super::database::CatalogDb;
use crate::image_loader::extensions::is_supported_image;

/// Import options for catalog
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ImportOptions {
    pub recursive: bool,
    pub skip_existing: bool,
    pub extract_exif: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            skip_existing: true,
            extract_exif: true,
        }
    }
}

/// Import progress callback
pub type ProgressCallback = Box<dyn Fn(usize, usize, &str) + Send>;

#[allow(dead_code)]
impl CatalogDb {
    /// Import a folder into the catalog
    pub fn import_folder<P: AsRef<Path>>(
        &mut self,
        folder_path: P,
        options: ImportOptions,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<usize> {
        let folder_path = folder_path.as_ref();

        // Collect all image files
        let mut image_files = Vec::new();

        let walker = if options.recursive {
            WalkDir::new(folder_path).follow_links(true)
        } else {
            WalkDir::new(folder_path).max_depth(1)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && is_supported_image(path) {
                image_files.push(path.to_path_buf());
            }
        }

        let total = image_files.len();
        let mut imported = 0;

        for (index, image_path) in image_files.iter().enumerate() {
            // Check if already exists
            if options.skip_existing {
                if let Ok(Some(_)) = self.get_image(image_path) {
                    continue;
                }
            }

            // Import the image
            match self.import_image(image_path) {
                Ok(_) => {
                    imported += 1;
                    if let Some(ref callback) = progress_callback {
                        callback(index + 1, total, image_path.to_string_lossy().as_ref());
                    }
                }
                Err(e) => {
                    log::warn!("Failed to import {:?}: {}", image_path, e);
                }
            }
        }

        Ok(imported)
    }

    /// Import a single file into the catalog
    pub fn import_file<P: AsRef<Path>>(&mut self, file_path: P) -> Result<i64> {
        let file_path = file_path.as_ref();

        if !file_path.is_file() {
            anyhow::bail!("Not a file: {:?}", file_path);
        }

        if !is_supported_image(file_path) {
            anyhow::bail!("Unsupported format: {:?}", file_path);
        }

        self.import_image(file_path)
    }

    /// Synchronize folder with catalog (remove deleted files, add new files)
    pub fn sync_folder<P: AsRef<Path>>(&mut self, folder_path: P) -> Result<(usize, usize)> {
        let folder_path = folder_path.as_ref();
        let folder_str = folder_path.to_string_lossy().to_string();

        // Get all images in this folder from catalog
        let catalog_images: Vec<(i64, std::path::PathBuf)> = {
            let mut stmt = self
                .conn
                .prepare("SELECT id, file_path FROM images WHERE folder_path = ?1")?;

            let results: Result<Vec<_>, _> = stmt
                .query_map(rusqlite::params![&folder_str], |row| {
                    Ok((
                        row.get(0)?,
                        std::path::PathBuf::from(row.get::<_, String>(1)?),
                    ))
                })?
                .collect();

            results?
        };

        // Check which ones still exist
        let mut removed = 0;
        for (id, path) in catalog_images {
            if !path.exists() {
                self.remove_image(id)?;
                removed += 1;
            }
        }

        // Import new files
        let options = ImportOptions {
            recursive: false,
            skip_existing: true,
            extract_exif: false,
        };

        let added = self.import_folder(folder_path, options, None)?;

        Ok((added, removed))
    }
}
