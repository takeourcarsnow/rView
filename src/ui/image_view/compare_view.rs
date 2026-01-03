use crate::app::{ImageViewerApp, LoaderMessage, ViewMode};
use egui::{self, Color32, CornerRadius, Rect};

impl ImageViewerApp {
    pub(crate) fn render_compare_view_public(&mut self, ctx: &egui::Context) {
        self.render_compare_view(ctx);
    }

    pub(crate) fn render_compare_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(self.settings.background_color.to_color()))
            .show(ctx, |ui| {
                let available = ui.available_size();

                // Determine two display indices to compare
                let mut sel: Vec<usize> = self.selected_indices.iter().cloned().collect();
                sel.sort_unstable();
                if sel.len() < 2 {
                    if self.filtered_list.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label("No images available to compare");
                        });
                        return;
                    }
                    let a = self.current_index;
                    let b = (self.current_index + 1) % self.filtered_list.len();
                    sel = vec![a, b];
                } else if sel.len() > 2 {
                    sel.truncate(2);
                }

                // Helper to get path for display index
                let get_path =
                    |app: &ImageViewerApp, display_idx: usize| -> Option<std::path::PathBuf> {
                        app.filtered_list
                            .get(display_idx)
                            .and_then(|&real_idx| app.image_list.get(real_idx).cloned())
                    };

                let left_path = get_path(self, sel[0]);
                let right_path = get_path(self, sel[1]);

                // Layout two panels side-by-side
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    // Left panel
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            let size_half =
                                egui::Vec2::new(available.x * 0.5 - 16.0, available.y - 24.0);
                            let (left_resp, left_painter) =
                                ui.allocate_painter(size_half, egui::Sense::click());
                            let rect = left_resp.rect;
                            left_painter.rect_filled(
                                rect,
                                CornerRadius::same(2),
                                Color32::from_rgb(20, 20, 22),
                            );

                            if let Some(path) = &left_path {
                                // Use current texture if comparing current image
                                let tex_id_opt = if sel[0] == self.current_index {
                                    self.current_texture.as_ref().map(|t| t.id())
                                } else {
                                    self.thumbnail_textures.get(path).map(|t| t.id())
                                };
                                if let Some(tex_id) = tex_id_opt {
                                    let tex_size = self.texture_size_from_id(tex_id);
                                    let base_scale =
                                        (rect.width() / tex_size.x).min(rect.height() / tex_size.y);
                                    let scaled = tex_size * base_scale * self.compare_zoom[0];
                                    let inner_rect = Rect::from_center_size(
                                        rect.center() + self.compare_pan[0],
                                        scaled,
                                    );
                                    left_painter.image(
                                        tex_id,
                                        inner_rect,
                                        Rect::from_min_max(
                                            egui::pos2(0.0, 0.0),
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        Color32::WHITE,
                                    );

                                    // Zoom with scroll when hovering
                                    if left_resp.hovered() {
                                        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                                        if scroll != 0.0 {
                                            let factor = 1.0 + scroll * 0.01; // larger per-tick zoom
                                            self.compare_zoom[0] =
                                                (self.compare_zoom[0] * factor).clamp(0.1, 16.0);
                                            ui.ctx().request_repaint();

                                            // If we are zooming a thumbnail (not current full image), request a larger preview once
                                            if !self.thumbnail_requests.contains(path)
                                                && self.compare_zoom[0] > 1.5
                                                && !self
                                                    .compare_large_preview_requests
                                                    .contains(path)
                                            {
                                                let path_clone = path.clone();
                                                let tx = self.loader_tx.clone();
                                                let cache = self.image_cache.clone();
                                                self.compare_large_preview_requests
                                                    .insert(path.clone());
                                                rayon::spawn(move || {
                                                    if let Ok(thumb) =
                                                        crate::image_loader::load_thumbnail(
                                                            &path_clone,
                                                            2048,
                                                        )
                                                    {
                                                        let thumb_clone = thumb.clone();
                                                        cache.insert_thumbnail(
                                                            path_clone.clone(),
                                                            thumb_clone,
                                                        );
                                                        let _ = tx.send(
                                                            LoaderMessage::ThumbnailLoaded(
                                                                path_clone, thumb,
                                                            ),
                                                        );
                                                    }
                                                });
                                            }
                                        }

                                        // Pan with drag
                                        if left_resp.dragged() {
                                            let delta = left_resp.drag_delta();
                                            self.compare_pan[0] += delta;
                                        }
                                    }
                                } else {
                                    if !self.thumbnail_requests.contains(path) {
                                        self.ensure_thumbnail_requested(path, ctx);
                                    }
                                    left_painter.text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        self.spinner_char(ui),
                                        egui::FontId::proportional(24.0),
                                        Color32::from_rgb(130, 130, 130),
                                    );
                                }

                                // EXIF overlay for this slot
                                if self.settings.show_exif_overlay {
                                    if let Some(exif) = self.compare_exifs.get(path) {
                                        let overlay_pos = rect.left_bottom()
                                            + egui::Vec2::new(12.0, -12.0 - 48.0);
                                        let overlay_rect = Rect::from_min_size(
                                            overlay_pos,
                                            egui::Vec2::new(280.0, 48.0),
                                        );
                                        left_painter.rect_filled(
                                            overlay_rect,
                                            CornerRadius::same(6),
                                            Color32::from_rgba_unmultiplied(0, 0, 0, 160),
                                        );
                                        let camera = exif
                                            .camera_model
                                            .clone()
                                            .unwrap_or_else(|| "Unknown".to_string());
                                        left_painter.text(
                                            overlay_rect.left_top() + egui::Vec2::new(8.0, 6.0),
                                            egui::Align2::LEFT_TOP,
                                            camera,
                                            egui::FontId::proportional(12.0),
                                            Color32::WHITE,
                                        );
                                        left_painter.text(
                                            overlay_rect.left_bottom() + egui::Vec2::new(8.0, -6.0),
                                            egui::Align2::LEFT_BOTTOM,
                                            format!(
                                                "{} • {}",
                                                exif.focal_length_formatted(),
                                                exif.aperture_formatted()
                                            ),
                                            egui::FontId::proportional(11.0),
                                            Color32::from_rgb(200, 200, 200),
                                        );
                                    } else {
                                        let path_clone = path.clone();
                                        self.spawn_loader(move || {
                                            let exif =
                                                crate::exif_data::ExifInfo::from_file(&path_clone);
                                            Some(LoaderMessage::ExifLoaded(
                                                path_clone,
                                                Box::new(exif),
                                            ))
                                        });
                                    }
                                } else if !self.thumbnail_requests.contains(path) {
                                    self.ensure_thumbnail_requested(path, ctx);
                                }

                                // Click to make this the current image
                                if left_resp.clicked() {
                                    if let Some(dp) = left_path.clone() {
                                        if let Some(idx) =
                                            self.image_list.iter().position(|p| p == &dp)
                                        {
                                            // find display index among filtered_list
                                            if let Some(pos) =
                                                self.filtered_list.iter().position(|&r| r == idx)
                                            {
                                                self.go_to_index(pos);
                                            }
                                        }
                                    }
                                }
                                // Context menu for left image
                                left_resp.context_menu(|ui: &mut egui::Ui| {
                                    if ui.button("View").clicked() {
                                        if let Some(dp) = left_path.clone() {
                                            if let Some(idx) =
                                                self.image_list.iter().position(|p| p == &dp)
                                            {
                                                if let Some(pos) = self
                                                    .filtered_list
                                                    .iter()
                                                    .position(|&r| r == idx)
                                                {
                                                    self.go_to_index(pos);
                                                }
                                            }
                                        }
                                        ui.close_menu();
                                    }

                                    ui.separator();
                                    if ui.button("Delete").clicked() {
                                        if let Some(dp) = left_path.clone() {
                                            if let Some(idx) =
                                                self.image_list.iter().position(|p| p == &dp)
                                            {
                                                if let Some(pos) = self
                                                    .filtered_list
                                                    .iter()
                                                    .position(|&r| r == idx)
                                                {
                                                    self.current_index = pos;
                                                    self.delete_current_image();
                                                }
                                            }
                                        }
                                        ui.close_menu();
                                    }
                                    ui.separator();

                                    // Add to Collection submenu
                                    if let Some(ref catalog_db) = self.catalog_db {
                                        if let Ok(collections) = catalog_db.get_collections() {
                                            if !collections.is_empty() {
                                                ui.menu_button(
                                                    "Add to Collection",
                                                    |ui: &mut egui::Ui| {
                                                        for collection in collections {
                                                            let label =
                                                                format!("📁 {}", collection.name);
                                                            if ui.button(&label).clicked() {
                                                                if let Some(path) =
                                                                    left_path.clone()
                                                                {
                                                                    let _ = self
                                                                        .add_path_to_collection(
                                                                            path,
                                                                            collection.id,
                                                                        );
                                                                }
                                                                ui.close_menu();
                                                            }
                                                        }
                                                    },
                                                );
                                                ui.separator();
                                            }
                                        }
                                    }

                                    ui.menu_button("Rating", |ui: &mut egui::Ui| {
                                        for r in 0..=5 {
                                            let stars = if r == 0 {
                                                "None".to_string()
                                            } else {
                                                "★".repeat(r)
                                            };
                                            if ui.button(stars).clicked() {
                                                if let Some(dp) = left_path.clone() {
                                                    if let Some(idx) = self
                                                        .image_list
                                                        .iter()
                                                        .position(|p| p == &dp)
                                                    {
                                                        if let Some(pos) = self
                                                            .filtered_list
                                                            .iter()
                                                            .position(|&r| r == idx)
                                                        {
                                                            self.current_index = pos;
                                                            self.set_rating(r as u8);
                                                        }
                                                    }
                                                }
                                                ui.close_menu();
                                            }
                                        }
                                    });
                                });
                            } else {
                                left_painter.text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "No image",
                                    egui::FontId::proportional(18.0),
                                    Color32::GRAY,
                                );
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Right panel - symmetric to left
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            let size_half =
                                egui::Vec2::new(available.x * 0.5 - 16.0, available.y - 24.0);
                            let (right_resp, right_painter) =
                                ui.allocate_painter(size_half, egui::Sense::click());
                            let rect = right_resp.rect;
                            right_painter.rect_filled(
                                rect,
                                CornerRadius::same(2),
                                Color32::from_rgb(20, 20, 22),
                            );

                            if let Some(path) = &right_path {
                                let tex_id_opt = if sel[1] == self.current_index {
                                    self.current_texture.as_ref().map(|t| t.id())
                                } else {
                                    self.thumbnail_textures.get(path).map(|t| t.id())
                                };
                                if let Some(tex_id) = tex_id_opt {
                                    let tex_size = self.texture_size_from_id(tex_id);
                                    let base_scale =
                                        (rect.width() / tex_size.x).min(rect.height() / tex_size.y);
                                    let scaled = tex_size * base_scale * self.compare_zoom[1];
                                    let inner_rect = Rect::from_center_size(rect.center(), scaled);
                                    right_painter.image(
                                        tex_id,
                                        inner_rect,
                                        Rect::from_min_max(
                                            egui::pos2(0.0, 0.0),
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        Color32::WHITE,
                                    );

                                    if right_resp.hovered() {
                                        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                                        if scroll != 0.0 {
                                            let factor = 1.0 + scroll * 0.01;
                                            self.compare_zoom[1] =
                                                (self.compare_zoom[1] * factor).clamp(0.1, 16.0);
                                            ui.ctx().request_repaint();

                                            if !self.thumbnail_requests.contains(path)
                                                && self.compare_zoom[1] > 1.5
                                                && !self
                                                    .compare_large_preview_requests
                                                    .contains(path)
                                            {
                                                let path_clone = path.clone();
                                                let tx = self.loader_tx.clone();
                                                let cache = self.image_cache.clone();
                                                self.compare_large_preview_requests
                                                    .insert(path.clone());
                                                rayon::spawn(move || {
                                                    if let Ok(thumb) =
                                                        crate::image_loader::load_thumbnail(
                                                            &path_clone,
                                                            2048,
                                                        )
                                                    {
                                                        let thumb_clone = thumb.clone();
                                                        cache.insert_thumbnail(
                                                            path_clone.clone(),
                                                            thumb_clone,
                                                        );
                                                        let _ = tx.send(
                                                            LoaderMessage::ThumbnailLoaded(
                                                                path_clone, thumb,
                                                            ),
                                                        );
                                                    }
                                                });
                                            }
                                        }

                                        if right_resp.dragged() {
                                            let delta = right_resp.drag_delta();
                                            self.compare_pan[1] += delta;
                                        }
                                    }
                                } else {
                                    if !self.thumbnail_requests.contains(path) {
                                        self.ensure_thumbnail_requested(path, ctx);
                                    }
                                    right_painter.text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        self.spinner_char(ui),
                                        egui::FontId::proportional(24.0),
                                        Color32::from_rgb(130, 130, 130),
                                    );
                                }

                                if let Some(exif) = self.compare_exifs.get(path) {
                                    let overlay_pos =
                                        rect.left_bottom() + egui::Vec2::new(12.0, -12.0 - 48.0);
                                    let overlay_rect = Rect::from_min_size(
                                        overlay_pos,
                                        egui::Vec2::new(280.0, 48.0),
                                    );
                                    right_painter.rect_filled(
                                        overlay_rect,
                                        CornerRadius::same(6),
                                        Color32::from_rgba_unmultiplied(0, 0, 0, 160),
                                    );
                                    right_painter.text(
                                        overlay_rect.left_top() + egui::Vec2::new(8.0, 6.0),
                                        egui::Align2::LEFT_TOP,
                                        exif.camera_model
                                            .clone()
                                            .unwrap_or_else(|| "Unknown".to_string()),
                                        egui::FontId::proportional(12.0),
                                        Color32::WHITE,
                                    );
                                    right_painter.text(
                                        overlay_rect.left_bottom() + egui::Vec2::new(8.0, -6.0),
                                        egui::Align2::LEFT_BOTTOM,
                                        format!(
                                            "{} • ISO {}",
                                            exif.aperture.clone().unwrap_or_default(),
                                            exif.iso.clone().unwrap_or_default()
                                        ),
                                        egui::FontId::proportional(11.0),
                                        Color32::from_rgb(200, 200, 200),
                                    );
                                } else {
                                    let p2 = path.clone();
                                    self.spawn_loader(move || {
                                        let exif = crate::exif_data::ExifInfo::from_file(&p2);
                                        Some(LoaderMessage::ExifLoaded(p2, Box::new(exif)))
                                    });
                                }

                                if right_resp.clicked() {
                                    if let Some(dp) = right_path.clone() {
                                        if let Some(idx) =
                                            self.image_list.iter().position(|p| p == &dp)
                                        {
                                            if let Some(pos) =
                                                self.filtered_list.iter().position(|&r| r == idx)
                                            {
                                                self.go_to_index(pos);
                                            }
                                        }
                                    }
                                }

                                // Context menu for right image
                                right_resp.context_menu(|ui: &mut egui::Ui| {
                                    if ui.button("View").clicked() {
                                        if let Some(dp) = right_path.clone() {
                                            if let Some(idx) =
                                                self.image_list.iter().position(|p| p == &dp)
                                            {
                                                if let Some(pos) = self
                                                    .filtered_list
                                                    .iter()
                                                    .position(|&r| r == idx)
                                                {
                                                    self.go_to_index(pos);
                                                }
                                            }
                                        }
                                        ui.close_menu();
                                    }

                                    ui.separator();
                                    if ui.button("Delete").clicked() {
                                        if let Some(dp) = right_path.clone() {
                                            if let Some(idx) =
                                                self.image_list.iter().position(|p| p == &dp)
                                            {
                                                if let Some(pos) = self
                                                    .filtered_list
                                                    .iter()
                                                    .position(|&r| r == idx)
                                                {
                                                    self.current_index = pos;
                                                    self.delete_current_image();
                                                }
                                            }
                                        }
                                        ui.close_menu();
                                    }
                                    ui.separator();

                                    // Add to Collection submenu
                                    if let Some(ref catalog_db) = self.catalog_db {
                                        if let Ok(collections) = catalog_db.get_collections() {
                                            if !collections.is_empty() {
                                                ui.menu_button(
                                                    "Add to Collection",
                                                    |ui: &mut egui::Ui| {
                                                        for collection in collections {
                                                            let label =
                                                                format!("📁 {}", collection.name);
                                                            if ui.button(&label).clicked() {
                                                                if let Some(path) =
                                                                    right_path.clone()
                                                                {
                                                                    let _ = self
                                                                        .add_path_to_collection(
                                                                            path,
                                                                            collection.id,
                                                                        );
                                                                }
                                                                ui.close_menu();
                                                            }
                                                        }
                                                    },
                                                );
                                                ui.separator();
                                            }
                                        }
                                    }

                                    ui.menu_button("Rating", |ui: &mut egui::Ui| {
                                        for r in 0..=5 {
                                            let stars = if r == 0 {
                                                "None".to_string()
                                            } else {
                                                "★".repeat(r)
                                            };
                                            if ui.button(stars).clicked() {
                                                if let Some(dp) = right_path.clone() {
                                                    if let Some(idx) = self
                                                        .image_list
                                                        .iter()
                                                        .position(|p| p == &dp)
                                                    {
                                                        if let Some(pos) = self
                                                            .filtered_list
                                                            .iter()
                                                            .position(|&r| r == idx)
                                                        {
                                                            self.current_index = pos;
                                                            self.set_rating(r as u8);
                                                        }
                                                    }
                                                }
                                                ui.close_menu();
                                            }
                                        }
                                    });
                                });
                            } else {
                                right_painter.text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "No image",
                                    egui::FontId::proportional(18.0),
                                    Color32::GRAY,
                                );
                            }
                        });
                    });

                    ui.add_space(8.0);
                });

                // Footer controls for compare
                ui.horizontal(|ui| {
                    if ui.button("Swap Sides").clicked() {
                        sel.swap(0, 1);
                        // Update selected_indices to reflect swap if applicable
                        self.selected_indices.clear();
                        self.selected_indices.insert(sel[0]);
                        self.selected_indices.insert(sel[1]);
                    }
                    if ui.button("Close Compare").clicked() {
                        self.view_mode = ViewMode::Single;
                    }
                });
            });
    }
}
