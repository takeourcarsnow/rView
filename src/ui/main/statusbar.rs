use crate::app::ImageViewerApp;
use egui::{self, Color32, RichText, Vec2, Margin};

impl ImageViewerApp {
    pub(crate) fn render_statusbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("statusbar")
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(25, 25, 28))
                .inner_margin(Margin::symmetric(12.0, 4.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Image info
                    if let Some(path) = self.get_current_path() {
                        let filename = path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        ui.label(RichText::new(&filename).color(Color32::WHITE).size(12.0));

                        // Dimensions
                        if let Some(tex) = &self.current_texture {
                            let size = tex.size_vec2();
                            ui.label(RichText::new(format!("{}×{}", size.x as u32, size.y as u32))
                                .color(Color32::GRAY).size(11.0));
                        }

                        // File size from EXIF
                        if let Some(exif) = &self.current_exif {
                            if let Some(ref size) = exif.file_size {
                                ui.label(RichText::new(size).color(Color32::GRAY).size(11.0));
                            }
                        }

                        // Preview indicator
                        if self.showing_preview {
                            ui.label(RichText::new("[Preview]")
                                .color(Color32::from_rgb(255, 200, 100)).size(11.0));
                        }

                        // Rating
                        let metadata = self.metadata_db.get(&path);
                        if metadata.rating > 0 {
                            ui.label(RichText::new("★".repeat(metadata.rating as usize))
                                .color(Color32::from_rgb(255, 200, 50)).size(11.0));
                        }

                        // Color label
                        if metadata.color_label != crate::settings::ColorLabel::None {
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 5.0, metadata.color_label.to_color());
                        }
                    }

                    // Spacer
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Status message
                        if let Some((msg, time)) = &self.status_message {
                            if time.elapsed().as_secs() < 3 {
                                ui.label(RichText::new(msg).color(Color32::from_rgb(100, 200, 100)).size(11.0));
                            }
                        }

                        // Zoom level
                        ui.label(RichText::new(format!("{:.0}%", self.zoom * 100.0))
                            .color(Color32::GRAY).size(11.0));

                        // Image counter
                        if !self.filtered_list.is_empty() {
                            ui.label(RichText::new(format!("{} / {}", self.current_index + 1, self.filtered_list.len()))
                                .color(Color32::GRAY).size(11.0));
                        }
                    });
                });
            });
    }
}