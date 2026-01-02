use crate::app::ImageViewerApp;
use egui::{self, Color32, Vec2, Rounding};

impl ImageViewerApp {
    pub(crate) fn render_lightbox(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(self.settings.background_color.to_color()))
            .show(ctx, |ui| {
                let _available = ui.available_size();
                let thumb_size = 150.0;
                let padding = 8.0;

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::splat(padding);

                            // Collect data first to avoid borrow conflicts
                            let items: Vec<(usize, usize, std::path::PathBuf)> = self.filtered_list.iter()
                                .enumerate()
                                .filter_map(|(display_idx, &real_idx)| {
                                    self.image_list.get(real_idx).map(|p| (display_idx, real_idx, p.clone()))
                                })
                                .collect();

                            let mut thumbnails_needed: Vec<std::path::PathBuf> = Vec::new();
                            let mut clicked_index: Option<(usize, bool)> = None; // (index, ctrl held)
                            let mut double_clicked_index: Option<usize> = None;

                            for (display_idx, _real_idx, path) in &items {
                                let is_selected = self.selected_indices.contains(display_idx) || *display_idx == self.current_index;

                                let (response, painter) = ui.allocate_painter(
                                    Vec2::splat(thumb_size),
                                    egui::Sense::click()
                                );

                                let rect = response.rect;

                                // Background
                                let bg_color = if is_selected {
                                    Color32::from_rgb(70, 130, 255)
                                } else if response.hovered() {
                                    Color32::from_rgb(50, 50, 55)
                                } else {
                                    Color32::from_rgb(35, 35, 40)
                                };

                                painter.rect_filled(rect, Rounding::same(6.0), bg_color);

                                // Thumbnail (preserve aspect ratio)
                                if let Some(handle) = self.thumbnail_textures.get(path) {
                                    let inner_rect = rect.shrink(4.0);
                                    // Compute texture size and scale to fit while preserving aspect
                                    let tex_size = self.texture_size_from_id(handle.id());
                                    let scale = (inner_rect.width() / tex_size.x).min(inner_rect.height() / tex_size.y);
                                    let display_size = tex_size * scale;
                                    let image_rect = egui::Rect::from_center_size(inner_rect.center(), display_size);
                                    painter.image(
                                        handle.id(),
                                        image_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        Color32::WHITE,
                                    );
                                } else {
                                    thumbnails_needed.push(path.clone());

                                    // Show spinner placeholder while thumbnail is loading
                                    let spinner = self.spinner_char(ui);
                                    painter.text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        spinner,
                                        egui::FontId::proportional(18.0),
                                        Color32::from_rgb(100, 100, 100),
                                    );
                                    ui.ctx().request_repaint();
                                }

                                // Rating stars
                                let metadata = self.metadata_db.get(path);
                                if metadata.rating > 0 {
                                    painter.text(
                                        rect.left_bottom() + Vec2::new(4.0, -4.0),
                                        egui::Align2::LEFT_BOTTOM,
                                        "★".repeat(metadata.rating as usize),
                                        egui::FontId::proportional(10.0),
                                        Color32::from_rgb(255, 200, 50),
                                    );
                                }

                                // Color label
                                if metadata.color_label != crate::settings::ColorLabel::None {
                                    painter.circle_filled(
                                        rect.right_top() + Vec2::new(-8.0, 8.0),
                                        5.0,
                                        metadata.color_label.to_color(),
                                    );
                                }

                                // Click handling
                                if response.clicked() {
                                    let ctrl = ui.input(|i| i.modifiers.ctrl);
                                    clicked_index = Some((*display_idx, ctrl));
                                }

                                // Double click to view
                                if response.double_clicked() {
                                    double_clicked_index = Some(*display_idx);
                                }
                            }

                            // Now apply state changes after the UI loop
                            for path in thumbnails_needed {
                                self.ensure_thumbnail_requested(&path, ctx);
                            }

                            if let Some((idx, ctrl)) = clicked_index {
                                if ctrl {
                                    if self.selected_indices.contains(&idx) {
                                        self.selected_indices.remove(&idx);
                                    } else {
                                        self.selected_indices.insert(idx);
                                    }
                                } else {
                                    self.selected_indices.clear();
                                    self.go_to_index(idx);
                                }
                            }

                            if let Some(idx) = double_clicked_index {
                                self.go_to_index(idx);
                                self.view_mode = crate::app::ViewMode::Single;
                            }
                        });
                    });
            });
    }
}