use crate::app::ImageViewerApp;
use crate::exif_data::ExifInfo;
use crate::metadata::FileOperation;
use egui::{self, Vec2};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub enum RenamePattern {
    AddDatePrefix,      // Add EXIF date as prefix: "2024-01-15_filename.jpg"
    AddDateTimePrefix,  // Add EXIF datetime as prefix: "2024-01-15_14-30-45_filename.jpg"
    ReplaceName,        // Replace entire name with date: "2024-01-15.jpg"
    AddSequence,        // Add sequence number: "001_filename.jpg"
    Custom,             // Custom pattern with placeholders
}

impl Default for RenamePattern {
    fn default() -> Self {
        RenamePattern::AddDatePrefix
    }
}

pub struct BatchRenameState {
    pub show_dialog: bool,
    pub pattern: RenamePattern,
    pub custom_pattern: String,
    pub preview_renames: Vec<(PathBuf, PathBuf)>,
    pub sequence_start: u32,
    pub sequence_padding: u32,
    pub date_format: String,
    pub separator: String,
}

impl Default for BatchRenameState {
    fn default() -> Self {
        Self {
            show_dialog: false,
            pattern: RenamePattern::AddDatePrefix,
            custom_pattern: "{date}_{name}".to_string(),
            preview_renames: Vec::new(),
            sequence_start: 1,
            sequence_padding: 3,
            date_format: "%Y-%m-%d".to_string(),
            separator: "_".to_string(),
        }
    }
}

impl ImageViewerApp {
    pub fn render_batch_rename_dialog(&mut self, ctx: &egui::Context) {
        if !self.batch_rename_state.show_dialog {
            return;
        }

        let mut open = true;
        egui::Window::new("Batch Rename Images")
            .open(&mut open)
            .default_width(600.0)
            .default_height(500.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .resizable(true)
            .show(ctx, |ui| {
                ui.label("Rename multiple images using EXIF date or other patterns.");
                ui.add_space(8.0);

                // Pattern selection
                ui.horizontal(|ui| {
                    ui.label("Pattern:");
                    egui::ComboBox::from_id_salt("rename_pattern")
                        .selected_text(format!("{:?}", self.batch_rename_state.pattern))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.batch_rename_state.pattern, RenamePattern::AddDatePrefix, "Add Date Prefix (YYYY-MM-DD_name)");
                            ui.selectable_value(&mut self.batch_rename_state.pattern, RenamePattern::AddDateTimePrefix, "Add DateTime Prefix");
                            ui.selectable_value(&mut self.batch_rename_state.pattern, RenamePattern::ReplaceName, "Replace with Date");
                            ui.selectable_value(&mut self.batch_rename_state.pattern, RenamePattern::AddSequence, "Add Sequence Number");
                            ui.selectable_value(&mut self.batch_rename_state.pattern, RenamePattern::Custom, "Custom Pattern");
                        });
                });

                // Pattern-specific options
                ui.add_space(8.0);
                match self.batch_rename_state.pattern {
                    RenamePattern::AddSequence => {
                        ui.horizontal(|ui| {
                            ui.label("Start number:");
                            ui.add(egui::DragValue::new(&mut self.batch_rename_state.sequence_start).range(1..=9999));
                            ui.label("Padding:");
                            ui.add(egui::DragValue::new(&mut self.batch_rename_state.sequence_padding).range(1..=6));
                        });
                    }
                    RenamePattern::Custom => {
                        ui.horizontal(|ui| {
                            ui.label("Pattern:");
                            ui.text_edit_singleline(&mut self.batch_rename_state.custom_pattern);
                        });
                        ui.label(egui::RichText::new("Placeholders: {date}, {datetime}, {name}, {seq}, {camera}, {ext}").small().weak());
                    }
                    _ => {}
                }

                ui.horizontal(|ui| {
                    ui.label("Separator:");
                    ui.text_edit_singleline(&mut self.batch_rename_state.separator);
                });

                ui.add_space(8.0);
                ui.separator();

                // Selection info
                let selected_count = if self.selected_indices.is_empty() {
                    self.filtered_list.len()
                } else {
                    self.selected_indices.len()
                };
                ui.label(format!("Images to rename: {} ({})", 
                    selected_count,
                    if self.selected_indices.is_empty() { "all in folder" } else { "selected" }
                ));

                // Generate preview button
                if ui.button("Generate Preview").clicked() {
                    self.generate_batch_rename_preview();
                }

                ui.add_space(8.0);

                // Preview scroll area
                ui.label("Preview:");
                egui::ScrollArea::vertical()
                    .max_height(250.0)
                    .show(ui, |ui| {
                        if self.batch_rename_state.preview_renames.is_empty() {
                            ui.label(egui::RichText::new("Click 'Generate Preview' to see rename results").weak());
                        } else {
                            egui::Grid::new("rename_preview_grid")
                                .num_columns(3)
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("Original").strong());
                                    ui.label("");
                                    ui.label(egui::RichText::new("New Name").strong());
                                    ui.end_row();

                                    for (original, new_name) in &self.batch_rename_state.preview_renames {
                                        let orig_name = original.file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_default();
                                        let new_name_str = new_name.file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_default();

                                        ui.label(&orig_name);
                                        ui.label("→");
                                        if orig_name == new_name_str {
                                            ui.label(egui::RichText::new(&new_name_str).weak());
                                        } else {
                                            ui.label(egui::RichText::new(&new_name_str).color(egui::Color32::from_rgb(100, 200, 100)));
                                        }
                                        ui.end_row();
                                    }
                                });
                        }
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Action buttons
                ui.horizontal(|ui| {
                    let can_rename = !self.batch_rename_state.preview_renames.is_empty();
                    
                    if ui.add_enabled(can_rename, egui::Button::new("Rename All")).clicked() {
                        self.execute_batch_rename();
                        self.batch_rename_state.show_dialog = false;
                    }
                    
                    if ui.button("Cancel").clicked() {
                        self.batch_rename_state.show_dialog = false;
                    }
                });
            });

        if !open {
            self.batch_rename_state.show_dialog = false;
        }
    }

    fn generate_batch_rename_preview(&mut self) {
        self.batch_rename_state.preview_renames.clear();

        // Get the list of images to rename
        let paths_to_rename: Vec<PathBuf> = if self.selected_indices.is_empty() {
            // All images in folder
            self.filtered_list.iter()
                .filter_map(|&idx| self.image_list.get(idx).cloned())
                .collect()
        } else {
            // Only selected images
            self.selected_indices.iter()
                .filter_map(|&display_idx| {
                    self.filtered_list.get(display_idx)
                        .and_then(|&real_idx| self.image_list.get(real_idx))
                        .cloned()
                })
                .collect()
        };

        let mut sequence = self.batch_rename_state.sequence_start;
        let separator = &self.batch_rename_state.separator.clone();
        let pattern = self.batch_rename_state.pattern.clone();
        let custom_pattern = self.batch_rename_state.custom_pattern.clone();
        let padding = self.batch_rename_state.sequence_padding as usize;

        for path in paths_to_rename {
            let exif = ExifInfo::from_file(&path);
            let new_name = self.generate_new_filename(&path, &exif, &pattern, &custom_pattern, separator, sequence, padding);
            
            if let Some(parent) = path.parent() {
                let new_path = parent.join(&new_name);
                self.batch_rename_state.preview_renames.push((path.clone(), new_path));
            }
            
            sequence += 1;
        }
    }

    fn generate_new_filename(
        &self,
        path: &PathBuf,
        exif: &ExifInfo,
        pattern: &RenamePattern,
        custom_pattern: &str,
        separator: &str,
        sequence: u32,
        padding: usize,
    ) -> String {
        let file_stem = path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string());
        
        let extension = path.extension()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "jpg".to_string());

        // Parse date from EXIF (format: "2024:01:15 14:30:45")
        let (date_str, datetime_str) = if let Some(ref date_taken) = exif.date_taken {
            let cleaned = date_taken.replace("\"", "").trim().to_string();
            let parts: Vec<&str> = cleaned.split_whitespace().collect();
            
            let date = if !parts.is_empty() {
                parts[0].replace(":", "-")
            } else {
                "unknown-date".to_string()
            };
            
            let datetime = if parts.len() > 1 {
                format!("{}{}{}", date, separator, parts[1].replace(":", "-"))
            } else {
                date.clone()
            };
            
            (date, datetime)
        } else {
            ("unknown-date".to_string(), "unknown-datetime".to_string())
        };

        let camera = exif.camera_model.clone()
            .unwrap_or_else(|| "unknown".to_string())
            .replace(" ", "-")
            .replace("/", "-");

        let seq_str = format!("{:0>width$}", sequence, width = padding);

        match pattern {
            RenamePattern::AddDatePrefix => {
                format!("{}{}{}.{}", date_str, separator, file_stem, extension)
            }
            RenamePattern::AddDateTimePrefix => {
                format!("{}{}{}.{}", datetime_str, separator, file_stem, extension)
            }
            RenamePattern::ReplaceName => {
                format!("{}.{}", date_str, extension)
            }
            RenamePattern::AddSequence => {
                format!("{}{}{}.{}", seq_str, separator, file_stem, extension)
            }
            RenamePattern::Custom => {
                let result = custom_pattern
                    .replace("{date}", &date_str)
                    .replace("{datetime}", &datetime_str)
                    .replace("{name}", &file_stem)
                    .replace("{seq}", &seq_str)
                    .replace("{camera}", &camera)
                    .replace("{ext}", &extension);
                
                // Ensure extension is present
                if !result.contains('.') {
                    format!("{}.{}", result, extension)
                } else {
                    result
                }
            }
        }
    }

    fn execute_batch_rename(&mut self) {
        let renames = self.batch_rename_state.preview_renames.clone();
        let mut success_count = 0;
        let mut error_count = 0;

        for (original, new_path) in renames {
            // Skip if names are the same
            if original == new_path {
                continue;
            }

            // Check if target already exists
            if new_path.exists() {
                log::warn!("Cannot rename {:?} to {:?}: target already exists", original, new_path);
                error_count += 1;
                continue;
            }

            // Perform the rename
            match std::fs::rename(&original, &new_path) {
                Ok(_) => {
                    // Update image list
                    if let Some(pos) = self.image_list.iter().position(|p| *p == original) {
                        self.image_list[pos] = new_path.clone();
                    }
                    
                    // Add to undo history
                    self.undo_history.push(FileOperation::Rename { 
                        from: original.clone(), 
                        to: new_path.clone() 
                    });
                    
                    // Update thumbnail cache key
                    if let Some(tex) = self.thumbnail_textures.remove(&original) {
                        self.thumbnail_textures.insert(new_path.clone(), tex);
                    }
                    
                    // Update metadata
                    self.metadata_db.rename_file(&original, &new_path);
                    
                    success_count += 1;
                }
                Err(e) => {
                    log::error!("Failed to rename {:?} to {:?}: {}", original, new_path, e);
                    error_count += 1;
                }
            }
        }

        // Save metadata after all renames
        self.metadata_db.save();
        
        // Clear preview
        self.batch_rename_state.preview_renames.clear();

        // Show status
        if error_count == 0 {
            self.show_status(&format!("Renamed {} files successfully", success_count));
        } else {
            self.show_status(&format!("Renamed {} files, {} failed", success_count, error_count));
        }
    }

    pub fn handle_batch_rename_key(&mut self) {
        if self.image_list.is_empty() {
            return;
        }
        self.batch_rename_state.show_dialog = true;
        self.batch_rename_state.preview_renames.clear();
    }
}
