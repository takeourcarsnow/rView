use crate::app::ImageViewerApp;
use egui::{self, Color32, Margin, RichText, Vec2};

impl ImageViewerApp {
    pub(crate) fn render_statusbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("statusbar")
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(25, 25, 28))
                    .inner_margin(Margin::symmetric(12, 4)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Image info
                    if let Some(path) = self.get_current_path() {
                        let filename = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        ui.label(RichText::new(&filename).color(Color32::WHITE).size(12.0));

                        // Dimensions
                        if let Some(tex) = &self.current_texture {
                            let size = tex.size_vec2();
                            ui.label(
                                RichText::new(format!("{}×{}", size.x as u32, size.y as u32))
                                    .color(Color32::GRAY)
                                    .size(11.0),
                            );
                        }

                        // File size from EXIF
                        if let Some(exif) = &self.current_exif {
                            if let Some(ref size) = exif.file_size {
                                ui.label(RichText::new(size).color(Color32::GRAY).size(11.0));
                            }
                        }

                        // Preview indicator
                        if self.showing_preview {
                            ui.label(
                                RichText::new("[Preview]")
                                    .color(Color32::from_rgb(255, 200, 100))
                                    .size(11.0),
                            );
                        }

                        // Rating
                        let metadata = self.metadata_db.get(&path);
                        if metadata.rating > 0 {
                            ui.label(
                                RichText::new("★".repeat(metadata.rating as usize))
                                    .color(Color32::from_rgb(255, 200, 50))
                                    .size(11.0),
                            );
                        }

                        // Color label
                        if metadata.color_label != crate::settings::ColorLabel::None {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
                            ui.painter().circle_filled(
                                rect.center(),
                                5.0,
                                metadata.color_label.to_color(),
                            );
                        }
                    }

                    // Spacer
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Status message
                        if let Some((msg, time)) = &self.status_message {
                            if time.elapsed().as_secs() < 3 {
                                ui.label(
                                    RichText::new(msg)
                                        .color(Color32::from_rgb(100, 200, 100))
                                        .size(11.0),
                                );
                            }
                        }

                        // Compact sort control (moved from filmstrip)
                        let order_label = match self.settings.sort_order {
                            crate::settings::SortOrder::Ascending => "A-Z",
                            crate::settings::SortOrder::Descending => "Z-A",
                        };
                        egui::ComboBox::from_id_salt("statusbar_sort")
                            .selected_text(format!(
                                "{:?} ({})",
                                self.settings.sort_mode, order_label
                            ))
                            .width(140.0)
                            .show_ui(ui, |ui| {
                                for mode in [
                                    crate::settings::SortMode::Name,
                                    crate::settings::SortMode::Date,
                                    crate::settings::SortMode::Size,
                                    crate::settings::SortMode::Type,
                                    crate::settings::SortMode::Random,
                                ] {
                                    if ui
                                        .selectable_label(
                                            self.settings.sort_mode == mode,
                                            format!("{:?}", mode),
                                        )
                                        .clicked()
                                    {
                                        self.settings.sort_mode = mode;
                                        self.sort_file_list();
                                    }
                                }
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label("Order:");
                                    if ui.button(order_label).clicked() {
                                        self.settings.sort_order = match self.settings.sort_order {
                                            crate::settings::SortOrder::Ascending => {
                                                self.settings.sort_ascending = false;
                                                crate::settings::SortOrder::Descending
                                            }
                                            crate::settings::SortOrder::Descending => {
                                                self.settings.sort_ascending = true;
                                                crate::settings::SortOrder::Ascending
                                            }
                                        };
                                        self.sort_file_list();
                                    }
                                });
                            });

                        // Zoom level
                        ui.label(
                            RichText::new(format!("{:.0}%", self.zoom * 100.0))
                                .color(Color32::GRAY)
                                .size(11.0),
                        );

                        // Image counter
                        if !self.filtered_list.is_empty() {
                            ui.label(
                                RichText::new(format!(
                                    "{} / {}",
                                    self.current_index + 1,
                                    self.filtered_list.len()
                                ))
                                .color(Color32::GRAY)
                                .size(11.0),
                            );
                        }
                    });
                });
            });
    }
}
