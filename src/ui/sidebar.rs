use crate::app::ImageViewerApp;
use crate::settings::ColorLabel;
use egui::{self, Color32, RichText, Vec2, Rounding, Margin, Rect};
use std::path::PathBuf;

impl ImageViewerApp {
    pub fn render_sidebar(&mut self, ctx: &egui::Context) {
        if !self.settings.show_sidebar {
            return;
        }
        
        egui::SidePanel::right("sidebar")
            .resizable(true)
            .default_width(280.0)
            .min_width(200.0)
            .max_width(400.0)
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(30, 30, 35))
                .inner_margin(Margin::same(8.0)))
            .show(ctx, |ui| {
                // Allow vertical auto-shrink so the file browser doesn't force an excessively tall panel
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        // Folder tree (collapsible)
                        collapsible_header(ui, "File Browser", true, |ui| {
                            self.render_folder_tree(ui);
                        });
                        ui.add_space(12.0);
                        
                        // (recent files removed)
                        // ui.add_space(12.0);
                        
                        // Histogram
                        if self.settings.show_histogram {
                            self.render_histogram_panel(ui);
                            ui.add_space(12.0);
                        }
                        
                        // Minimap
                        if self.zoom > 1.0 {
                            self.render_minimap(ui);
                            ui.add_space(12.0);
                        }
                        
                        // EXIF data (full panel only - avoids duplicated summary)
                        if self.settings.show_exif {
                            self.render_exif_panel(ui);
                            ui.add_space(12.0);
                        }
                        
                        // Rating & Labels
                        self.render_metadata_panel(ui);
                        ui.add_space(12.0);
                    });
            });
    }

    fn render_folder_tree(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(Color32::from_rgb(25, 25, 30))
            .rounding(Rounding::same(4.0))
            .inner_margin(Margin::same(8.0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("file_tree")
                    // Shrink vertically to content instead of expanding to full panel height
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        // Show drives/root directories
                        #[cfg(windows)]
                        {
                            use std::path::Path;
                            
                            // Windows drives
                            for drive in b'A'..=b'Z' {
                                let drive_str = format!("{}:\\", drive as char);
                                let drive_path = Path::new(&drive_str);
                                if drive_path.exists() {
                                    self.render_tree_node(ui, drive_path.to_path_buf(), 0);
                                }
                            }
                        }
                        
                        #[cfg(unix)]
                        {
                            // Unix root
                            self.render_tree_node(ui, PathBuf::from("/"), 0);
                        }
                        
                        #[cfg(not(any(windows, unix)))]
                        {
                            // Fallback - show current directory
                            if let Ok(current) = std::env::current_dir() {
                                if let Some(parent) = current.parent() {
                                    self.render_tree_node(ui, parent.to_path_buf(), 0);
                                }
                            }
                        }
                    });
            });
    }
    
    fn render_tree_node(&mut self, ui: &mut egui::Ui, path: PathBuf, depth: usize) {
        if depth > 10 {
            return; // Prevent infinite recursion
        }
        
        let is_expanded = self.expanded_dirs.contains(&path);
        let is_current_folder = self.current_folder.as_ref() == Some(&path);
        
        // Get directory name
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        
        // Indentation
        let indent = depth as f32 * 12.0;
        
        ui.horizontal(|ui| {
            ui.add_space(indent);
            
            // Expand/collapse button
            let expand_response = if path.is_dir() {
                // Use simple ASCII markers to ensure correct rendering on all fonts
                let icon = if is_expanded { "v" } else { ">" };
                ui.add(egui::Button::new(RichText::new(icon).size(10.0))
                    .fill(Color32::TRANSPARENT)
                    .stroke(egui::Stroke::NONE)
                    .min_size(Vec2::new(16.0, 16.0)))
            } else {
                ui.add_space(16.0);
                ui.separator()
            };
            
            if expand_response.clicked() && path.is_dir() {
                if is_expanded {
                    self.expanded_dirs.remove(&path);
                } else {
                    self.expanded_dirs.insert(path.clone());
                }
            }
            
            // Directory/file button
            let mut button = egui::Button::new(RichText::new(&name).size(11.0))
                .fill(if is_current_folder {
                    self.settings.accent_color.to_color().linear_multiply(0.3)
                } else {
                    Color32::TRANSPARENT
                })
                .stroke(egui::Stroke::NONE)
                .wrap();
            
            if path.is_file() {
                button = button.min_size(Vec2::new(0.0, 18.0));
            }
            
            let response = ui.add(button);
            
            if response.clicked() {
                if path.is_dir() {
                    self.load_folder(path.clone());
                } else if path.is_file() {
                    self.load_image_file(path.clone());
                }
            }
        });
        
        // Render children if expanded
        if is_expanded && path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                let mut dirs = Vec::new();
                let mut files = Vec::new();
                
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        // Skip hidden directories
                        if !entry_path.file_name()
                            .map(|n| n.to_string_lossy().starts_with('.'))
                            .unwrap_or(false) {
                            dirs.push(entry_path);
                        }
                    } else if entry_path.is_file() {
                        // Only show image files
                        if crate::image_loader::is_supported_image(&entry_path) {
                            files.push(entry_path);
                        }
                    }
                }
                
                // Sort directories and files
                dirs.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
                files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
                
                // Render directories first, then files
                for dir_path in dirs {
                    self.render_tree_node(ui, dir_path, depth + 1);
                }
                
                for file_path in files {
                    self.render_tree_node(ui, file_path, depth + 1);
                }
            }
        }
    }
    
    
    fn render_histogram_panel(&self, ui: &mut egui::Ui) {
        collapsible_header(ui, "Histogram", true, |ui| {
            let height = 100.0;
            let (response, painter) = ui.allocate_painter(
                Vec2::new(ui.available_width(), height),
                egui::Sense::hover()
            );
            let rect = response.rect;
            
            // Background
            painter.rect_filled(rect, Rounding::same(4.0), Color32::from_rgb(20, 20, 25));
            
            // histogram_data is Vec<Vec<u32>> where [0]=red, [1]=green, [2]=blue
            if let Some(histogram) = &self.histogram_data {
                if histogram.len() >= 3 {
                    let w = rect.width();
                    let h = rect.height() - 4.0;
                    
                    // Find max for scaling
                    let max_val = histogram[0].iter()
                        .chain(histogram[1].iter())
                        .chain(histogram[2].iter())
                        .max()
                        .copied()
                        .unwrap_or(1) as f32;
                    
                    // Draw histograms
                    for (i, _) in histogram[0].iter().enumerate().take(256.min(histogram[0].len())) {
                        let x = rect.left() + (i as f32 / 255.0) * w;
                        
                        // Red
                        let r_h = (histogram[0][i] as f32 / max_val) * h;
                        painter.line_segment(
                            [egui::pos2(x, rect.bottom() - 2.0), egui::pos2(x, rect.bottom() - 2.0 - r_h)],
                            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 0, 0, 100)),
                        );
                        
                        // Green
                        let g_h = (histogram[1][i] as f32 / max_val) * h;
                        painter.line_segment(
                            [egui::pos2(x, rect.bottom() - 2.0), egui::pos2(x, rect.bottom() - 2.0 - g_h)],
                            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 255, 0, 100)),
                        );
                        
                        // Blue
                        let b_h = (histogram[2][i] as f32 / max_val) * h;
                        painter.line_segment(
                            [egui::pos2(x, rect.bottom() - 2.0), egui::pos2(x, rect.bottom() - 2.0 - b_h)],
                            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 255, 100)),
                        );
                    }
                }
            }
            
            // RGB toggle buttons
            ui.horizontal(|ui| {
                ui.small("R");
                ui.small("G");
                ui.small("B");
                ui.small("L");
            });
        });
    }
    
    fn render_minimap(&self, ui: &mut egui::Ui) {
        collapsible_header(ui, "Navigator", true, |ui| {
            let size = 150.0;
            let (response, painter) = ui.allocate_painter(
                Vec2::splat(size),
                egui::Sense::click_and_drag()
            );
            let rect = response.rect;
        });
    }


    
    fn render_exif_panel(&mut self, ui: &mut egui::Ui) {
        collapsible_header(ui, "EXIF Data", true, |ui| {
            if let Some(exif) = &self.current_exif {
                // If EXIF object exists but contains no meaningful fields, show a helpful message
                if !exif.has_data() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("No EXIF data found for this image").color(Color32::GRAY).size(11.0));
                        if ui.button("Reload EXIF").clicked() {
                            if let Some(path) = self.get_current_path() {
                                let path_clone = path.clone();
                                let tx = self.loader_tx.clone();
                                self.set_status_message(format!("Reloading EXIF for {}", path_clone.display()));
                                std::thread::spawn(move || {
                                    let exif = crate::exif_data::ExifInfo::from_file(&path_clone);
                                    let _ = tx.send(super::LoaderMessage::ExifLoaded(path_clone, Box::new(exif)));
                                });
                            }
                        }
                    });
                } else {
                    exif_row_opt(ui, "Camera", &exif.camera_model);
                    exif_row_opt(ui, "Lens", &exif.lens);
                    let fl = exif.focal_length_formatted();
                    if !fl.is_empty() { exif_row(ui, "Focal Length", &fl); }
                    let ap = exif.aperture_formatted();
                    if !ap.is_empty() { exif_row(ui, "Aperture", &ap); }
                    exif_row_opt(ui, "Shutter", &exif.shutter_speed);
                    exif_row_opt(ui, "ISO", &exif.iso);
                    exif_row_opt(ui, "Date", &exif.date_taken);
                    exif_row_opt(ui, "Dimensions", &exif.dimensions);
                    if exif.gps_latitude.is_some() && exif.gps_longitude.is_some() {
                        let gps = format!("{:.4}, {:.4}", 
                            exif.gps_latitude.unwrap_or(0.0), 
                            exif.gps_longitude.unwrap_or(0.0));
                        exif_row(ui, "GPS", &gps);
                    }

                    ui.add_space(6.0);
                    // Manual reload button
                    ui.horizontal(|ui| {
                        if ui.button("Reload EXIF").clicked() {
                            if let Some(path) = self.get_current_path() {
                                let path_clone = path.clone();
                                let tx = self.loader_tx.clone();
                                self.set_status_message(format!("Reloading EXIF for {}", path_clone.display()));
                                std::thread::spawn(move || {
                                    let exif = crate::exif_data::ExifInfo::from_file(&path_clone);
                                    let _ = tx.send(super::LoaderMessage::ExifLoaded(path_clone, Box::new(exif)));
                                });
                            }
                        }
                    });
                }

            } else {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("No EXIF data found").color(Color32::GRAY).size(11.0));
                    if ui.button("Reload EXIF").clicked() {
                        if let Some(path) = self.get_current_path() {
                            let path_clone = path.clone();
                            let tx = self.loader_tx.clone();
                            self.set_status_message(format!("Reloading EXIF for {}", path_clone.display()));
                            std::thread::spawn(move || {
                                let exif = crate::exif_data::ExifInfo::from_file(&path_clone);
                                let _ = tx.send(super::LoaderMessage::ExifLoaded(path_clone, Box::new(exif)));
                            });
                        }
                    }
                });
            }
        });
    }
    
    fn render_metadata_panel(&mut self, ui: &mut egui::Ui) {
        collapsible_header(ui, "Rating & Labels", true, |ui| {
            if let Some(path) = self.get_current_path() {
                let metadata = self.metadata_db.get(&path);
                
                // Star rating
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Rating:").size(11.0).color(Color32::GRAY));
                    
                    for i in 1..=5 {
                        let star = if i <= metadata.rating { "★" } else { "☆" };
                        let color = if i <= metadata.rating {
                            Color32::from_rgb(255, 200, 50)
                        } else {
                            Color32::from_rgb(100, 100, 100)
                        };
                        
                        if ui.add(egui::Button::new(RichText::new(star).size(16.0).color(color))
                            .fill(Color32::TRANSPARENT)
                            .min_size(Vec2::new(20.0, 20.0)))
                            .clicked() {
                            let new_rating = if metadata.rating == i { 0 } else { i };
                            self.set_rating(new_rating);
                        }
                    }
                });
                
                ui.add_space(4.0);
                
                // Color labels
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Label:").size(11.0).color(Color32::GRAY));
                    
                    for label in [ColorLabel::None, ColorLabel::Red, ColorLabel::Yellow, 
                                  ColorLabel::Green, ColorLabel::Blue, ColorLabel::Purple] {
                        let is_selected = metadata.color_label == label;
                        let color = label.to_color();
                        
                        let (rect, response) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::click());
                        
                        if label == ColorLabel::None {
                            ui.painter().rect_stroke(rect, Rounding::same(2.0),
                                egui::Stroke::new(1.0, Color32::GRAY));
                            if is_selected {
                                ui.painter().line_segment(
                                    [rect.left_top() + Vec2::new(3.0, 3.0), rect.right_bottom() - Vec2::new(3.0, 3.0)],
                                    egui::Stroke::new(1.0, Color32::GRAY),
                                );
                            }
                        } else {
                            ui.painter().rect_filled(rect, Rounding::same(2.0), color);
                            if is_selected {
                                ui.painter().rect_stroke(rect, Rounding::same(2.0),
                                    egui::Stroke::new(2.0, Color32::WHITE));
                            }
                        }
                        
                        if response.clicked() {
                            self.set_color_label(label);
                        }
                    }
                });
                
                // Tags
                ui.add_space(8.0);
                ui.label(RichText::new("Tags:").size(11.0).color(Color32::GRAY));
                
                ui.horizontal_wrapped(|ui| {
                    for tag in metadata.tags.clone() {
                        ui.add(egui::Button::new(RichText::new(&tag).size(10.0))
                            .fill(Color32::from_rgb(50, 50, 55))
                            .rounding(Rounding::same(10.0)));
                    }
                });
            }
        });
    }
}

fn collapsible_header<R>(
    ui: &mut egui::Ui,
    title: &str,
    default_open: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::CollapsingResponse<R> {
    egui::CollapsingHeader::new(RichText::new(title).size(12.0).strong())
        .default_open(default_open)
        .show(ui, add_contents)
}

fn exif_row(ui: &mut egui::Ui, label: &str, value: &str) {
    if value.is_empty() || value == "Unknown" {
        return;
    }
    
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{}:", label)).size(10.0).color(Color32::GRAY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(10.0).color(Color32::from_rgb(200, 200, 200)));
        });
    });
}

fn exif_row_opt(ui: &mut egui::Ui, label: &str, value: &Option<String>) {
    if let Some(v) = value {
        if !v.is_empty() && v != "Unknown" {
            exif_row(ui, label, v);
        }
    }
}
