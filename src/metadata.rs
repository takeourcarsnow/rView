use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::settings::ColorLabel;

/// Metadata stored for each image (ratings, labels, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageMetadata {
    pub rating: u8,  // 0-5 stars
    pub color_label: ColorLabel,
    pub tags: Vec<String>,
    pub notes: String,
    pub flagged: bool,
    pub rejected: bool,
}

/// Database of image metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataDb {
    pub images: HashMap<PathBuf, ImageMetadata>,
}

impl MetadataDb {
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
    
    pub fn get(&self, path: &PathBuf) -> ImageMetadata {
        self.images.get(path).cloned().unwrap_or_default()
    }
    
    pub fn set_rating(&mut self, path: PathBuf, rating: u8) {
        let entry = self.images.entry(path).or_default();
        entry.rating = rating.min(5);
    }
    
    pub fn set_color_label(&mut self, path: PathBuf, color: ColorLabel) {
        let entry = self.images.entry(path).or_default();
        entry.color_label = color;
    }
    
    pub fn toggle_flag(&mut self, path: PathBuf) {
        let entry = self.images.entry(path).or_default();
        entry.flagged = !entry.flagged;
    }
    
    pub fn toggle_reject(&mut self, path: PathBuf) {
        let entry = self.images.entry(path).or_default();
        entry.rejected = !entry.rejected;
    }
    
    pub fn add_tag(&mut self, path: PathBuf, tag: String) {
        let entry = self.images.entry(path).or_default();
        if !entry.tags.contains(&tag) {
            entry.tags.push(tag);
        }
    }
    
    pub fn remove_tag(&mut self, path: PathBuf, tag: &str) {
        let entry = self.images.entry(path).or_default();
        entry.tags.retain(|t| t != tag);
    }
    
    pub fn restore_metadata(&mut self, path: PathBuf, metadata: ImageMetadata) {
        self.images.insert(path, metadata);
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
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
    Rotate {
        path: PathBuf,
        degrees: i32,
        previous_rotation: f32,
    },
    Adjust {
        path: PathBuf,
        adjustments: crate::image_loader::ImageAdjustments,
        previous_adjustments: crate::image_loader::ImageAdjustments,
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

    pub fn can_undo(&self) -> bool {
        self.current_index > 0
    }

    pub fn can_redo(&self) -> bool {
        self.current_index < self.operations.len()
    }

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

    pub fn clear(&mut self) {
        self.operations.clear();
        self.current_index = 0;
    }
}

/// Batch rename pattern
#[derive(Debug, Clone)]
pub struct RenamePattern {
    pub prefix: String,
    pub suffix: String,
    pub counter_start: u32,
    pub counter_digits: u32,
    pub use_original_name: bool,
    pub find: String,
    pub replace: String,
}

impl Default for RenamePattern {
    fn default() -> Self {
        Self {
            prefix: String::new(),
            suffix: String::new(),
            counter_start: 1,
            counter_digits: 3,
            use_original_name: true,
            find: String::new(),
            replace: String::new(),
        }
    }
}

impl RenamePattern {
    pub fn apply(&self, original: &str, index: u32) -> String {
        let mut name = if self.use_original_name {
            // Get name without extension
            let parts: Vec<&str> = original.rsplitn(2, '.').collect();
            if parts.len() == 2 {
                parts[1].to_string()
            } else {
                original.to_string()
            }
        } else {
            String::new()
        };
        
        // Apply find/replace
        if !self.find.is_empty() {
            name = name.replace(&self.find, &self.replace);
        }
        
        // Build new name
        let counter = format!("{:0width$}", self.counter_start + index, width = self.counter_digits as usize);
        
        let ext = original.rsplit('.').next().unwrap_or("");
        
        if self.use_original_name {
            format!("{}{}{}.{}", self.prefix, name, self.suffix, ext)
        } else {
            format!("{}{}{}.{}", self.prefix, counter, self.suffix, ext)
        }
    }
}
