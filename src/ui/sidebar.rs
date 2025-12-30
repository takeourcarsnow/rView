use crate::app::ImageViewerApp;
use crate::image_loader::ImageAdjustments;
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
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Folder tree
                        self.render_folder_tree(ui);
                        ui.add_space(12.0);
                        
                        // Recent files
                        self.render_recent_files(ui);
                        ui.add_space(12.0);
                        
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
                        
                        // EXIF data
                        if self.settings.show_exif {
                            self.render_exif_panel(ui);
                            ui.add_space(12.0);
                        }
                        
                        // Rating & Labels
                        self.render_metadata_panel(ui);
                        ui.add_space(12.0);
                        
                        // Adjustments
                        if self.settings.show_adjustments {
                            self.render_adjustments_panel(ui);
                        }
                    });
            });
    }

    fn render_folder_tree(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Folders").size(14.0).color(Color32::WHITE));
        ui.add_space(4.0);
        
        egui::Frame::none()
            .fill(Color32::from_rgb(25, 25, 30))
            .rounding(Rounding::same(4.0))
            .inner_margin(Margin::same(8.0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("folder_tree")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Recent folders
                        if !self.settings.recent_folders.is_empty() {
                            ui.label(RichText::new("Recent").size(12.0).color(Color32::GRAY));
                            ui.add_space(2.0);
                            
                            let recent_folders: Vec<PathBuf> = self.settings.recent_folders.clone();
                            
                            for folder in &recent_folders {
                                let is_current = self.current_folder.as_ref() == Some(folder);
                                let folder_name = folder.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| folder.display().to_string());
                                
                                let response = ui.add(
                                    egui::Button::new(RichText::new(&folder_name).size(11.0))
                                        .fill(if is_current {
                                            self.settings.accent_color.to_color().linear_multiply(0.3)
                                        } else {
                                            Color32::TRANSPARENT
                                        })
                                        .stroke(egui::Stroke::NONE)
                                        .wrap()
                                );
                                
                                if response.clicked() {
                                    self.load_folder(folder.clone());
                                }
                            }
                            
                            ui.add_space(8.0);
                        }
                        
                        // Bookmarks
                        if !self.settings.bookmarks.is_empty() {
                            ui.label(RichText::new("Bookmarks").size(12.0).color(Color32::GRAY));
                            ui.add_space(2.0);
                            
                            let bookmarks: Vec<crate::settings::Bookmark> = self.settings.bookmarks.clone();
                            
                            for bookmark in &bookmarks {
                                let is_current = match bookmark.bookmark_type {
                                    crate::settings::BookmarkType::Folder => {
                                        self.current_folder.as_ref() == Some(&bookmark.path)
                                    }
                                    crate::settings::BookmarkType::Image => {
                                        self.get_current_path().as_ref() == Some(&bookmark.path)
                                    }
                                };
                                
                                let response = ui.add(
                                    egui::Button::new(RichText::new(&bookmark.name).size(11.0))
                                        .fill(if is_current {
                                            self.settings.accent_color.to_color().linear_multiply(0.3)
                                        } else {
                                            Color32::TRANSPARENT
                                        })
                                        .stroke(egui::Stroke::NONE)
                                        .wrap()
                                );
                                
                                if response.clicked() {
                                    match bookmark.bookmark_type {
                                        crate::settings::BookmarkType::Folder => {
                                            self.load_folder(bookmark.path.clone());
                                        }
                                        crate::settings::BookmarkType::Image => {
                                            self.load_image_file(bookmark.path.clone());
                                        }
                                    }
                                }
                            }
                        }
                    });
            });
    }
    
    fn render_recent_files(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Recent Files").size(14.0).color(Color32::WHITE));
        ui.add_space(4.0);
        
        egui::Frame::none()
            .fill(Color32::from_rgb(25, 25, 30))
            .rounding(Rounding::same(4.0))
            .inner_margin(Margin::same(8.0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("recent_files")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Get recent files from settings (we'll need to add this)
                        // For now, show some example recent files
                        let recent_files = vec![
                            ("IMG_1234.jpg", "2 hours ago"),
                            ("DSC_5678.CR2", "Yesterday"),
                            ("photo_001.png", "3 days ago"),
                        ];
                        
                        for (filename, time) in recent_files {
                            ui.horizontal(|ui| {
                                // Thumbnail placeholder
                                let (rect, _) = ui.allocate_exact_size(Vec2::new(40.0, 40.0), egui::Sense::click());
                                ui.painter().rect_filled(rect, Rounding::same(2.0), Color32::from_rgb(50, 50, 55));
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "📷",
                                    egui::FontId::proportional(16.0),
                                    Color32::GRAY,
                                );
                                
                                ui.vertical(|ui| {
                                    ui.add_space(2.0);
                                    ui.label(RichText::new(filename).size(11.0).color(Color32::WHITE));
                                    ui.label(RichText::new(time).size(10.0).color(Color32::GRAY));
                                });
                            });
                            
                            if ui.input(|i| i.pointer.primary_clicked()) {
                                // Would load the file here
                            }
                        }
                    });
            });
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
                    for i in 0..256.min(histogram[0].len()) {
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
            
            // Background
            painter.rect_filled(rect, Rounding::same(4.0), Color32::from_rgb(20, 20, 25));
            
            // Draw thumbnail
            if let Some(tex) = &self.current_texture {
                let tex_size = tex.size_vec2();
                let scale = (rect.width() / tex_size.x).min(rect.height() / tex_size.y);
                let thumb_size = tex_size * scale;
                
                let thumb_rect = Rect::from_center_size(rect.center(), thumb_size);
                painter.image(
                    tex.id(),
                    thumb_rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                
                // Draw viewport rectangle
                let view_w = (1.0 / self.zoom) * thumb_size.x;
                let view_h = (1.0 / self.zoom) * thumb_size.y;
                let view_x = thumb_rect.center().x - self.pan_offset.x * scale - view_w / 2.0;
                let view_y = thumb_rect.center().y - self.pan_offset.y * scale - view_h / 2.0;
                
                let view_rect = Rect::from_min_size(
                    egui::pos2(view_x, view_y),
                    Vec2::new(view_w, view_h)
                ).intersect(thumb_rect);
                
                painter.rect_stroke(view_rect, Rounding::ZERO, 
                    egui::Stroke::new(2.0, Color32::from_rgb(70, 130, 255)));
            }
        });
    }
    
    fn render_exif_panel(&self, ui: &mut egui::Ui) {
        collapsible_header(ui, "EXIF Data", true, |ui| {
            if let Some(exif) = &self.current_exif {
                exif_row_opt(ui, "Camera", &exif.camera_model);
                exif_row_opt(ui, "Lens", &exif.lens);
                exif_row_opt(ui, "Focal Length", &exif.focal_length);
                exif_row_opt(ui, "Aperture", &exif.aperture);
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
            } else {
                ui.label(RichText::new("No EXIF data").color(Color32::GRAY).size(11.0));
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
    
    fn render_adjustments_panel(&mut self, ui: &mut egui::Ui) {
        collapsible_header(ui, "Adjustments", true, |ui| {
            let mut changed = false;
            
            // Exposure
            ui.horizontal(|ui| {
                ui.label(RichText::new("Exposure").size(11.0).color(Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{:+.1}", self.adjustments.exposure)).size(10.0));
                });
            });
            if ui.add(egui::Slider::new(&mut self.adjustments.exposure, -3.0..=3.0).show_value(false)).changed() {
                changed = true;
            }
            
            // Contrast
            ui.horizontal(|ui| {
                ui.label(RichText::new("Contrast").size(11.0).color(Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{:.0}", self.adjustments.contrast * 100.0 - 100.0)).size(10.0));
                });
            });
            if ui.add(egui::Slider::new(&mut self.adjustments.contrast, 0.5..=2.0).show_value(false)).changed() {
                changed = true;
            }
            
            // Saturation
            ui.horizontal(|ui| {
                ui.label(RichText::new("Saturation").size(11.0).color(Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{:.0}", self.adjustments.saturation * 100.0 - 100.0)).size(10.0));
                });
            });
            if ui.add(egui::Slider::new(&mut self.adjustments.saturation, 0.0..=2.0).show_value(false)).changed() {
                changed = true;
            }
            
            // Temperature
            ui.horizontal(|ui| {
                ui.label(RichText::new("Temperature").size(11.0).color(Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{:+.0}K", self.adjustments.temperature * 1000.0)).size(10.0));
                });
            });
            if ui.add(egui::Slider::new(&mut self.adjustments.temperature, -1.0..=1.0).show_value(false)).changed() {
                changed = true;
            }
            
            // Tint
            ui.horizontal(|ui| {
                ui.label(RichText::new("Tint").size(11.0).color(Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{:+.0}", self.adjustments.tint * 100.0)).size(10.0));
                });
            });
            if ui.add(egui::Slider::new(&mut self.adjustments.tint, -1.0..=1.0).show_value(false)).changed() {
                changed = true;
            }
            
            // Highlights
            ui.horizontal(|ui| {
                ui.label(RichText::new("Highlights").size(11.0).color(Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{:+.0}", self.adjustments.highlights * 100.0)).size(10.0));
                });
            });
            if ui.add(egui::Slider::new(&mut self.adjustments.highlights, -1.0..=1.0).show_value(false)).changed() {
                changed = true;
            }
            
            // Shadows
            ui.horizontal(|ui| {
                ui.label(RichText::new("Shadows").size(11.0).color(Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{:+.0}", self.adjustments.shadows * 100.0)).size(10.0));
                });
            });
            if ui.add(egui::Slider::new(&mut self.adjustments.shadows, -1.0..=1.0).show_value(false)).changed() {
                changed = true;
            }
            
            // Blacks
            ui.horizontal(|ui| {
                ui.label(RichText::new("Blacks").size(11.0).color(Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{:+.0}", self.adjustments.blacks * 100.0)).size(10.0));
                });
            });
            if ui.add(egui::Slider::new(&mut self.adjustments.blacks, -1.0..=1.0).show_value(false)).changed() {
                changed = true;
            }
            
            // Whites
            ui.horizontal(|ui| {
                ui.label(RichText::new("Whites").size(11.0).color(Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{:+.0}", self.adjustments.whites * 100.0)).size(10.0));
                });
            });
            if ui.add(egui::Slider::new(&mut self.adjustments.whites, -1.0..=1.0).show_value(false)).changed() {
                changed = true;
            }
            
            // Sharpening
            ui.horizontal(|ui| {
                ui.label(RichText::new("Sharpening").size(11.0).color(Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{:.0}", self.adjustments.sharpening * 100.0)).size(10.0));
                });
            });
            if ui.add(egui::Slider::new(&mut self.adjustments.sharpening, 0.0..=2.0).show_value(false)).changed() {
                changed = true;
            }
            
            ui.add_space(8.0);
            
            // Reset button
            ui.horizontal(|ui| {
                if ui.button("Reset All").clicked() {
                    self.adjustments = ImageAdjustments::default();
                    changed = true;
                }
                
                // Before/After toggle
                let ba_text = if self.show_original { "Show Edited" } else { "Show Original" };
                if ui.button(ba_text).clicked() {
                    self.show_original = !self.show_original;
                    changed = true;
                }
            });
            
            if changed {
                self.refresh_adjustments();
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
