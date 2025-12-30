use crate::image_loader;
use crate::metadata::FileOperation;
use crate::settings::ColorLabel;
use std::path::PathBuf;

use super::ImageViewerApp;

impl ImageViewerApp {
    // Rotation
    pub fn rotate_left(&mut self) {
        self.rotate_by(-90.0);
    }

    pub fn rotate_right(&mut self) {
        self.rotate_by(90.0);
    }

    fn rotate_by(&mut self, degrees: f32) {
        if let Some(path) = self.get_current_path() {
            if let Some(image) = &self.current_image {
                let previous_rotation = self.rotation;
                self.rotation = (self.rotation + degrees) % 360.0;

                // Apply rotation to the image
                let rotated_image = image_loader::rotate_image(image, degrees as i32);

                // Update the current image and texture
                self.set_current_image(&path, rotated_image);

                self.undo_history.push(FileOperation::Rotate {
                    path: path.clone(),
                    degrees: degrees as i32,
                    previous_rotation,
                });

                self.show_status(&format!("Rotated {}°", degrees as i32));
            }
        }
    }

    // Slideshow
    pub fn toggle_slideshow(&mut self) {
        self.slideshow_active = !self.slideshow_active;
        self.slideshow_timer = 0.0;
    }

    // Ratings
    pub fn set_rating(&mut self, rating: u8) {
        if let Some(path) = self.get_current_path() {
            let previous_rating = self.metadata_db.get(&path).rating;
            self.metadata_db.set_rating(path.clone(), rating);
            self.metadata_db.save();

            self.undo_history.push(FileOperation::Rate {
                path: path.clone(),
                rating,
                previous_rating,
            });

            self.show_status(&format!("Rating: {}", "★".repeat(rating as usize)));
        }
    }

    // Color labels
    pub fn set_color_label(&mut self, color: ColorLabel) {
        if let Some(path) = self.get_current_path() {
            let previous_color_label = self.metadata_db.get(&path).color_label;
            self.metadata_db.set_color_label(path.clone(), color);
            self.metadata_db.save();

            self.undo_history.push(FileOperation::Label {
                path: path.clone(),
                color_label: color,
                previous_color_label,
            });
        }
    }

    // File operations
    pub fn delete_current_image(&mut self) {
        if let Some(path) = self.get_current_path() {
            // Backup metadata before deletion
            let metadata_backup = serde_json::to_string(&self.metadata_db.get(&path)).ok();

            if self.settings.delete_to_trash {
                if trash::delete(&path).is_ok() {
                    self.undo_history.push(FileOperation::Delete {
                        original_path: path.clone(),
                        trash_path: None, // TODO: Get actual trash path if possible
                        metadata_backup,
                    });
                }
            } else {
                let _ = std::fs::remove_file(&path);
            }

            if let Some(&idx) = self.filtered_list.get(self.current_index) {
                self.image_list.remove(idx);
            }
            self.image_cache.remove(&path);
            self.thumbnail_textures.remove(&path);

            self.apply_filter();

            if self.current_index >= self.filtered_list.len() && !self.filtered_list.is_empty() {
                self.current_index = self.filtered_list.len() - 1;
            }

            if !self.filtered_list.is_empty() {
                self.load_current_image();
            } else {
                self.current_texture = None;
                self.current_image = None;
            }

            self.show_status("Image deleted");
        }
    }

    pub fn move_to_folder(&mut self, dest_folder: PathBuf) {
        if let Some(path) = self.get_current_path() {
            let filename = path.file_name().unwrap_or_default();
            let dest_path = dest_folder.join(filename);

            if std::fs::rename(&path, &dest_path).is_ok() {
                self.undo_history.push(FileOperation::Move {
                    from: path.clone(),
                    to: dest_path,
                });

                if let Some(&idx) = self.filtered_list.get(self.current_index) {
                    self.image_list.remove(idx);
                }
                self.apply_filter();

                if self.current_index >= self.filtered_list.len() && !self.filtered_list.is_empty() {
                    self.current_index = self.filtered_list.len() - 1;
                }

                if !self.filtered_list.is_empty() {
                    self.load_current_image();
                }

                self.show_status(&format!("Moved to {}", dest_folder.display()));
            }
        }
    }

    pub fn copy_to_folder(&mut self, dest_folder: PathBuf) {
        if let Some(path) = self.get_current_path() {
            let filename = path.file_name().unwrap_or_default();
            let dest_path = dest_folder.join(filename);

            if std::fs::copy(&path, &dest_path).is_ok() {
                self.show_status(&format!("Copied to {}", dest_folder.display()));
            }
        }
    }

    pub fn move_to_selected_folder(&mut self) {
        if let Some(path) = self.get_current_path() {
            if let Some(parent) = path.parent() {
                let selected_folder = parent.join("selected");

                // Create the selected folder if it doesn't exist
                if let Err(e) = std::fs::create_dir_all(&selected_folder) {
                    self.show_status(&format!("Failed to create selected folder: {}", e));
                    return;
                }

                let filename = path.file_name().unwrap_or_default();
                let dest_path = selected_folder.join(filename);

                if std::fs::rename(&path, &dest_path).is_ok() {
                    self.undo_history.push(FileOperation::Move {
                        from: path.clone(),
                        to: dest_path,
                    });

                    if let Some(&idx) = self.filtered_list.get(self.current_index) {
                        self.image_list.remove(idx);
                    }
                    self.image_cache.remove(&path);
                    self.thumbnail_textures.remove(&path);

                    self.apply_filter();

                    if self.current_index >= self.filtered_list.len() && !self.filtered_list.is_empty() {
                        self.current_index = self.filtered_list.len() - 1;
                    }

                    if !self.filtered_list.is_empty() {
                        self.load_current_image();
                    } else {
                        self.current_texture = None;
                        self.current_image = None;
                    }

                    self.show_status(&format!("Moved to {}", selected_folder.display()));
                } else {
                    self.show_status("Failed to move file");
                }
            }
        }
    }

    pub fn undo_last_operation(&mut self) {
        let current_path = self.get_current_path();
        let op = self.undo_history.undo().cloned();

        if let Some(op) = op {
            match op {
                FileOperation::Delete { original_path, trash_path, metadata_backup } => {
                    // Try to restore from trash or show message
                    if let Some(trash_path) = trash_path {
                        if std::fs::rename(trash_path, &original_path).is_ok() {
                            // Restore metadata if available
                            if let Some(metadata_json) = metadata_backup {
                                if let Ok(metadata) = serde_json::from_str::<crate::metadata::ImageMetadata>(&metadata_json) {
                                    self.metadata_db.restore_metadata(original_path.clone(), metadata);
                                }
                            }
                            self.image_list.push(original_path.clone());
                            self.sort_images();
                            self.apply_filter();
                            self.show_status("Undo: File restored");
                        } else {
                            self.show_status(&format!("Cannot undo delete of {}", original_path.display()));
                        }
                    } else {
                        self.show_status(&format!("Cannot undo delete of {}", original_path.display()));
                    }
                }
                FileOperation::Move { from, to } => {
                    if std::fs::rename(&to, &from).is_ok() {
                        self.image_list.push(from.clone());
                        self.sort_images();
                        self.apply_filter();
                        self.show_status("Undo: Move reverted");
                    }
                }
                FileOperation::Rename { from, to } => {
                    if std::fs::rename(&to, &from).is_ok() {
                        if let Some(pos) = self.image_list.iter().position(|p| p == &*to) {
                            self.image_list[pos] = from.clone();
                        }
                        self.show_status("Undo: Rename reverted");
                    }
                }
                FileOperation::Rotate { path, degrees: _degrees, previous_rotation } => {
                    if current_path.as_ref() == Some(&path) {
                        // Calculate the reverse rotation to undo
                        let reverse_degrees = previous_rotation - self.rotation;
                        self.rotation = previous_rotation;

                        if let Some(image) = &self.current_image {
                            let rotated_image = image_loader::rotate_image(image, reverse_degrees as i32);
                            self.set_current_image(&path, rotated_image);
                        }
                    }
                    self.show_status("Undo: Rotation reverted");
                }
                FileOperation::Adjust { path, previous_adjustments, .. } => {
                    if current_path.as_ref() == Some(&path) {
                        self.adjustments = previous_adjustments.clone();
                        self.refresh_adjustments();
                    }
                    self.show_status("Undo: Adjustments reverted");
                }
                FileOperation::Rate { path, previous_rating, .. } => {
                    self.metadata_db.set_rating(path.clone(), previous_rating);
                    self.metadata_db.save();
                    self.show_status("Undo: Rating reverted");
                }
                FileOperation::Label { path, previous_color_label, .. } => {
                    self.metadata_db.set_color_label(path.clone(), previous_color_label);
                    self.metadata_db.save();
                    self.show_status("Undo: Label reverted");
                }
            }
        }
    }

    pub fn refresh_adjustments(&mut self) {
        if let (Some(image), Some(path)) = (&self.current_image, self.get_current_path()) {
            self.set_current_image(&path, image.clone());
        }
    }

    pub fn redo_last_operation(&mut self) {
        let current_path = self.get_current_path();
        let op = self.undo_history.redo().cloned();

        if let Some(op) = op {
            match op {
                FileOperation::Delete { original_path, .. } => {
                    // Re-delete the file
                    if trash::delete(&original_path).is_ok() {
                        if let Some(&idx) = self.filtered_list.get(self.current_index) {
                            self.image_list.remove(idx);
                        }
                        self.apply_filter();
                        self.show_status("Redo: File deleted again");
                    }
                }
                FileOperation::Move { from, to } => {
                    if std::fs::rename(&to, &from).is_ok() {
                        if let Some(pos) = self.image_list.iter().position(|p| *p == from) {
                            self.image_list[pos] = from.clone();
                        }
                        self.show_status("Redo: Move reapplied");
                    }
                }
                FileOperation::Rename { from, to } => {
                    if std::fs::rename(&from, &to).is_ok() {
                        if let Some(pos) = self.image_list.iter().position(|p| *p == from) {
                            self.image_list[pos] = to.clone();
                        }
                        self.show_status("Redo: Rename reapplied");
                    }
                }
                FileOperation::Rotate { path, degrees, .. } => {
                    if current_path.as_ref() == Some(&path) {
                        self.rotation = (self.rotation + degrees as f32) % 360.0;

                        if let Some(image) = &self.current_image {
                            let rotated_image = image_loader::rotate_image(image, degrees);
                            self.set_current_image(&path, rotated_image);
                        }
                    }
                    self.show_status("Redo: Rotation reapplied");
                }
                FileOperation::Adjust { path, adjustments, .. } => {
                    if current_path.as_ref() == Some(&path) {
                        self.adjustments = adjustments.clone();
                        self.refresh_adjustments();
                    }
                    self.show_status("Redo: Adjustments reapplied");
                }
                FileOperation::Rate { path, rating, .. } => {
                    self.metadata_db.set_rating(path.clone(), rating);
                    self.metadata_db.save();
                    self.show_status("Redo: Rating reapplied");
                }
                FileOperation::Label { path, color_label, .. } => {
                    self.metadata_db.set_color_label(path.clone(), color_label);
                    self.metadata_db.save();
                    self.show_status("Redo: Label reapplied");
                }
            }
        }
    }

    pub fn show_status(&mut self, message: &str) {
        self.status_message = Some((message.to_string(), std::time::Instant::now()));
    }
}