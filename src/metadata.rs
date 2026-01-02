use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::settings::ColorLabel;
use crate::image_loader::ImageAdjustments;

/// Metadata stored for each image (ratings, labels, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageMetadata {
    pub rating: u8,  // 0-5 stars
    pub color_label: ColorLabel,
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
    pub fn new() -> Self { Self::default() }

    pub fn load() -> Self {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "imageviewer", "ImageViewer") {
            let db_path = proj_dirs.data_dir().join("metadata.json");
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
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "imageviewer", "ImageViewer") {
            let data_dir = proj_dirs.data_dir();
            let _ = std::fs::create_dir_all(data_dir);
            let db_path = data_dir.join("metadata.json");
            if let Ok(content) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(db_path, content);
            }
        }
    }
    
    pub fn get<P: AsRef<std::path::Path>>(&self, path: P) -> ImageMetadata {
        self.images.get(path.as_ref()).cloned().unwrap_or_default()
    }
    
    pub fn set_rating<P: Into<PathBuf>>(&mut self, path: P, rating: u8) {
        let path = path.into();
        let entry = self.images.entry(path).or_default();
        entry.rating = rating.min(5);
    }
    
    pub fn set_color_label<P: Into<PathBuf>>(&mut self, path: P, color: ColorLabel) {
        let path = path.into();
        let entry = self.images.entry(path).or_default();
        entry.color_label = color;
    }
    
    #[allow(dead_code)]
    pub fn toggle_flag<P: AsRef<std::path::Path>>(&mut self, path: P) {
        let path = path.as_ref().to_path_buf();
        let entry = self.images.entry(path).or_default();
        entry.flagged = !entry.flagged;
    }
    
    #[allow(dead_code)]
    pub fn toggle_reject<P: AsRef<std::path::Path>>(&mut self, path: P) {
        let path = path.as_ref().to_path_buf();
        let entry = self.images.entry(path).or_default();
        entry.rejected = !entry.rejected;
    }
    
    #[allow(dead_code)]
    pub fn add_tag<P: Into<PathBuf>>(&mut self, path: P, tag: String) {
        let path = path.into();
        let entry = self.images.entry(path).or_default();
        if !entry.tags.contains(&tag) {
            entry.tags.push(tag);
        }
    }
    
    #[allow(dead_code)]
    pub fn remove_tag<P: AsRef<std::path::Path>>(&mut self, path: P, tag: &str) {
        let path = path.as_ref().to_path_buf();
        let entry = self.images.entry(path).or_default();
        entry.tags.retain(|t| t != tag);
    }
    
    pub fn restore_metadata(&mut self, path: PathBuf, metadata: ImageMetadata) {
        self.images.insert(path, metadata);
    }
    
    /// Get adjustments for an image, returns None if no adjustments are stored
    pub fn get_adjustments<P: AsRef<std::path::Path>>(&self, path: P) -> Option<ImageAdjustments> {
        self.images.get(path.as_ref()).and_then(|m| m.adjustments.clone())
    }
    
    /// Set adjustments for an image (only stores if not default)
    pub fn set_adjustments<P: Into<PathBuf>>(&mut self, path: P, adjustments: &ImageAdjustments) {
        let path = path.into();
        let entry = self.images.entry(path).or_default();
        if adjustments.is_default() {
            entry.adjustments = None;
        } else {
            entry.adjustments = Some(adjustments.clone());
        }
    }
    
    /// Rename a file's metadata entry (update the key in the hashmap)
    pub fn rename_file(&mut self, old_path: &PathBuf, new_path: &PathBuf) {
        if let Some(metadata) = self.images.remove(old_path) {
            self.images.insert(new_path.clone(), metadata);
        }
    }
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
    Adjust {
        path: PathBuf,
        adjustments: crate::image_loader::ImageAdjustments,
        previous_adjustments: Box<crate::image_loader::ImageAdjustments>,
    },
    Rate {
        path: PathBuf,
        rating: u8,
        previous_rating: u8,
    },
    Label {
        path: PathBuf,
        color_label: crate::settings::ColorLabel,
        previous_color_label: crate::settings::ColorLabel,
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
            self.operations.get(self.current_index - 1).map(|op| match op {
                FileOperation::Delete { original_path, .. } => {
                    format!("Delete {}", original_path.file_name().unwrap_or_default().to_string_lossy())
                }
                FileOperation::Move { from, to } => {
                    format!("Move {} to {}",
                        from.file_name().unwrap_or_default().to_string_lossy(),
                        to.parent().unwrap_or(to).display())
                }
                FileOperation::Rename { from, to } => {
                    format!("Rename {} to {}",
                        from.file_name().unwrap_or_default().to_string_lossy(),
                        to.file_name().unwrap_or_default().to_string_lossy())
                }
                FileOperation::Rotate { path, degrees, .. } => {
                    format!("Rotate {} by {}°", path.file_name().unwrap_or_default().to_string_lossy(), degrees)
                }
                FileOperation::Adjust { path, .. } => {
                    format!("Adjust {}", path.file_name().unwrap_or_default().to_string_lossy())
                }
                FileOperation::Rate { path, rating, .. } => {
                    format!("Rate {} with {} stars", path.file_name().unwrap_or_default().to_string_lossy(), rating)
                }
                FileOperation::Label { path, color_label, .. } => {
                    format!("Label {} with {}", path.file_name().unwrap_or_default().to_string_lossy(), color_label.name())
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


