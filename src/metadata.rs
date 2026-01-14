use crate::image_loader::ImageAdjustments;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Metadata stored for each image (ratings, labels, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageMetadata {
    pub tags: Vec<String>,
    pub notes: String,
    pub flagged: bool,
    pub rejected: bool,
    #[serde(default)]
    pub adjustments: Option<ImageAdjustments>,
}

/// Database of image metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataDb {
    pub images: HashMap<PathBuf, ImageMetadata>,
}

impl MetadataDb {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    fn db_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("com", "imageviewer", "ImageViewer")
            .map(|proj_dirs| proj_dirs.data_dir().join("metadata.json"))
    }

    pub fn load() -> Self {
        if let Some(db_path) = Self::db_path() {
            if db_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&db_path) {
                    if let Ok(db) = serde_json::from_str(&content) {
                        return db;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(db_path) = Self::db_path() {
            if let Some(data_dir) = db_path.parent() {
                let _ = std::fs::create_dir_all(data_dir);
                if let Ok(content) = serde_json::to_string_pretty(self) {
                    let _ = std::fs::write(db_path, content);
                }
            }
        }
    }

    fn get_entry_mut<P: AsRef<std::path::Path>>(&mut self, path: P) -> &mut ImageMetadata {
        self.images.entry(path.as_ref().to_path_buf()).or_default()
    }

    pub fn get<P: AsRef<std::path::Path>>(&self, path: P) -> ImageMetadata {
        self.images.get(path.as_ref()).cloned().unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn toggle_flag<P: AsRef<std::path::Path>>(&mut self, path: P) {
        let entry = self.get_entry_mut(path);
        entry.flagged = !entry.flagged;
    }

    #[allow(dead_code)]
    pub fn toggle_reject<P: AsRef<std::path::Path>>(&mut self, path: P) {
        let entry = self.get_entry_mut(path);
        entry.rejected = !entry.rejected;
    }

    #[allow(dead_code)]
    pub fn add_tag<P: AsRef<std::path::Path>>(&mut self, path: P, tag: String) {
        let entry = self.get_entry_mut(path);
        if !entry.tags.contains(&tag) {
            entry.tags.push(tag);
        }
    }

    #[allow(dead_code)]
    pub fn remove_tag<P: AsRef<std::path::Path>>(&mut self, path: P, tag: &str) {
        let entry = self.get_entry_mut(path);
        entry.tags.retain(|t| t != tag);
    }

    pub fn restore_metadata(&mut self, path: PathBuf, metadata: ImageMetadata) {
        self.images.insert(path, metadata);
    }

    /// Get adjustments for an image, returns None if no adjustments are stored
    pub fn get_adjustments<P: AsRef<std::path::Path>>(&self, path: P) -> Option<ImageAdjustments> {
        self.images
            .get(path.as_ref())
            .and_then(|m| m.adjustments.clone())
    }

    /// Set adjustments for an image (only stores if not default)
    pub fn set_adjustments<P: AsRef<std::path::Path>>(&mut self, path: P, adjustments: &ImageAdjustments) {
        let entry = self.get_entry_mut(path);
        if adjustments.is_default() {
            entry.adjustments = None;
        } else {
            entry.adjustments = Some(adjustments.clone());
        }
    }

    /// Rename a file's metadata entry (update the key in the hashmap)
    pub fn rename_file(&mut self, old_path: &std::path::Path, new_path: &std::path::Path) {
        if let Some(metadata) = self.images.remove(&old_path.to_path_buf()) {
            self.images.insert(new_path.to_path_buf(), metadata);
        }
    }
}

/// Helper function to get a display-friendly file name from a path
fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

/// Undo/Redo history for file operations
#[derive(Debug, Clone)]
pub enum FileOperation {
    Delete {
        original_path: PathBuf,
        trash_path: Option<PathBuf>,
        metadata_backup: Option<String>, // JSON serialized metadata
    },
    Move {
        from: PathBuf,
        to: PathBuf,
    },
    #[allow(dead_code)]
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
    Rotate {
        path: PathBuf,
        degrees: i32,
        previous_rotation: f32,
    },
    #[allow(dead_code)]
    Crop {
        path: PathBuf,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        previous_dimensions: (u32, u32), // (width, height) before crop
    },
    #[allow(dead_code)]
    Adjust {
        path: PathBuf,
        adjustments: crate::image_loader::ImageAdjustments,
        previous_adjustments: Box<crate::image_loader::ImageAdjustments>,
    },
}

#[derive(Debug, Default)]
pub struct UndoHistory {
    operations: Vec<FileOperation>,
    max_size: usize,
    current_index: usize, // For redo support
}

impl UndoHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            operations: Vec::new(),
            max_size,
            current_index: 0,
        }
    }

    pub fn push(&mut self, op: FileOperation) {
        // Remove any operations after current index (for when user does new operation after undo)
        self.operations.truncate(self.current_index);

        self.operations.push(op);
        self.current_index = self.operations.len();

        if self.operations.len() > self.max_size {
            self.operations.remove(0);
            self.current_index = self.current_index.saturating_sub(1);
        }
    }

    pub fn undo(&mut self) -> Option<&FileOperation> {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.operations.get(self.current_index)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<&FileOperation> {
        if self.current_index < self.operations.len() {
            let op = self.operations.get(self.current_index);
            self.current_index += 1;
            op
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn can_undo(&self) -> bool {
        self.current_index > 0
    }

    #[allow(dead_code)]
    pub fn can_redo(&self) -> bool {
        self.current_index < self.operations.len()
    }

    #[allow(dead_code)]
    pub fn last_operation_description(&self) -> Option<String> {
        if self.current_index > 0 {
            self.operations
                .get(self.current_index - 1)
                .map(|op| match op {
                    FileOperation::Delete { original_path, .. } => {
                        format!("Delete {}", file_name(original_path))
                    }
                    FileOperation::Move { from, to } => {
                        format!(
                            "Move {} to {}",
                            file_name(from),
                            to.parent().unwrap_or(to).display()
                        )
                    }
                    FileOperation::Rename { from, to } => {
                        format!(
                            "Rename {} to {}",
                            file_name(from),
                            file_name(to)
                        )
                    }
                    FileOperation::Rotate { path, degrees, .. } => {
                        format!("Rotate {} by {}°", file_name(path), degrees)
                    }
                    FileOperation::Crop {
                        path,
                        width,
                        height,
                        ..
                    } => {
                        format!("Crop {} to {}x{}", file_name(path), width, height)
                    }
                    FileOperation::Adjust { path, .. } => {
                        format!("Adjust {}", file_name(path))
                    }
                })
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.operations.clear();
        self.current_index = 0;
    }
}
