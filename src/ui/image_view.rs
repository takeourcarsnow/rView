use crate::app::{ImageViewerApp, ViewMode};
use crate::settings::{BackgroundColor, GridType};
use egui::{self, Color32, Vec2, Rect, Rounding, Stroke};
use image::GenericImageView;

impl ImageViewerApp {
    pub fn render_main_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(self.settings.background_color.to_color()))
            .show(ctx, |ui| {
                match self.view_mode {
                    ViewMode::Single => self.render_single_view(ui, ctx),
                    ViewMode::Lightbox => {} , // Handled separately
                    ViewMode::Compare => self.render_compare_view(ctx),
                }
            });
    }
    
    fn render_single_view(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let available = ui.available_size();
        self.available_view_size = available; // Store for fit functions
        let response = ui.allocate_response(available, egui::Sense::click_and_drag());
        let rect = response.rect;
        
        // Handle mouse input
        self.handle_image_input(&response, ui);
        
        // Draw checkered background if selected
        if self.settings.background_color == BackgroundColor::Checkered {
            self.draw_checkered_background(ui, rect);
        }
        
        // Draw image
        if let Some(tex) = &self.current_texture {
            let tex_size = tex.size_vec2();
            let display_size = tex_size * self.zoom;
            
            let image_rect = Rect::from_center_size(
                rect.center() + self.pan_offset,
                display_size
            );
            
            ui.painter().image(
                tex.id(),
                image_rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            
            // Draw overlays
            self.draw_overlays(ui, image_rect);

            // Show EXIF overlay on top of image if enabled (overlay toggle controls only overlay)
            if self.settings.show_exif_overlay {
                if let Some(ref exif) = self.current_exif {
                    self.draw_exif_overlay(ui, image_rect, exif);
                }
            }
            
            // Show "Loading full resolution..." indicator for previews
            if self.showing_preview && self.is_loading {
                let indicator_rect = Rect::from_min_size(
                    rect.left_top() + Vec2::new(10.0, 10.0),
                    Vec2::new(200.0, 30.0)
                );
                ui.painter().rect_filled(
                    indicator_rect, 
                    Rounding::same(6.0), 
                    Color32::from_rgba_unmultiplied(0, 0, 0, 200)
                );
                ui.painter().text(
                    indicator_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "⟳ Loading full resolution...",
                    egui::FontId::proportional(12.0),
                    Color32::from_rgb(200, 200, 200),
                );
            }
        } else if self.is_loading {
            // Loading indicator with animation
            let time = ui.input(|i| i.time);
            let dots = ".".repeat(((time * 2.0) as usize % 4) + 1);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("Loading{}", dots),
                egui::FontId::proportional(24.0),
                Color32::from_rgb(180, 180, 180),
            );
            ui.ctx().request_repaint();
        } else if let Some(ref error) = self.load_error {
            // Error message
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("Error: {}", error),
                egui::FontId::proportional(18.0),
                Color32::from_rgb(255, 100, 100),
            );
        } else if self.image_list.is_empty() {
            // Drop hint
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Drop images or folders here\nor use Ctrl+O to open",
                egui::FontId::proportional(20.0),
                Color32::from_rgb(150, 150, 150),
            );
        }
        
        // Draw loupe if enabled
        if self.settings.loupe_enabled {
            self.draw_loupe(ui);
        }
        
        // Draw color picker info
        if self.picked_color.is_some() {
            self.draw_color_info(ui, rect);
        }

        // Note: EXIF overlay shown inline above when drawing the image so it has access to image_rect
    }
    
    fn draw_exif_overlay(&self, ui: &mut egui::Ui, image_rect: Rect, exif: &crate::exif_data::ExifInfo) {
        // Small semi-transparent overlay in bottom-left showing camera/date/settings
        let overlay_size = egui::Vec2::new(320.0, 44.0);
        let pos = image_rect.left_bottom() + egui::Vec2::new(12.0, -12.0 - overlay_size.y);
        let overlay_rect = Rect::from_min_size(pos, overlay_size);

        ui.painter().rect_filled(overlay_rect, Rounding::same(6.0), Color32::from_rgba_unmultiplied(0,0,0,180));

        if !exif.has_data() {
            ui.painter().text(
                overlay_rect.center(),
                egui::Align2::CENTER_CENTER,
                "No EXIF data",
                egui::FontId::proportional(13.0),
                Color32::WHITE,
            );
        } else {
            let camera = exif.camera_model.clone().unwrap_or_else(|| "Unknown Camera".to_string());
            let date = exif.date_taken.clone().unwrap_or_else(|| "Unknown Date".to_string());
            let settings = format!("{} • {} • ISO {}", exif.focal_length_formatted(), exif.aperture_formatted(), exif.iso.clone().unwrap_or_default());

            ui.painter().text(
                overlay_rect.left_top() + egui::Vec2::new(8.0, 6.0),
                egui::Align2::LEFT_TOP,
                camera,
                egui::FontId::proportional(13.0),
                Color32::WHITE,
            );
            ui.painter().text(
                overlay_rect.left_bottom() + egui::Vec2::new(8.0, -6.0),
                egui::Align2::LEFT_BOTTOM,
                format!("{} — {}", settings, date),
                egui::FontId::proportional(11.0),
                Color32::from_rgb(200,200,200),
            );
        }
    }

    /// Return the pixel size of a texture given its `TextureId` by inspecting
    /// the current texture and cached thumbnails. Falls back to a sensible
    /// default if the texture is unknown.
    pub(crate) fn texture_size_from_id(&self, id: egui::TextureId) -> Vec2 {
        if let Some(ref t) = self.current_texture {
            if t.id() == id {
                return t.size_vec2();
            }
        }
        for (_path, tex) in self.thumbnail_textures.iter() {
            if tex.id() == id {
                return tex.size_vec2();
            }
        }
        // Fallback default size to avoid division by zero
        Vec2::new(800.0, 600.0)
    }

    pub fn render_compare_view_public(&mut self, ctx: &egui::Context) {
        self.render_compare_view(ctx);
    }

    fn render_compare_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(self.settings.background_color.to_color()))
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
                let get_path = |app: &ImageViewerApp, display_idx: usize| -> Option<std::path::PathBuf> {
                    app.filtered_list.get(display_idx).and_then(|&real_idx| app.image_list.get(real_idx).cloned())
                };

                let left_path = get_path(self, sel[0]);
                let right_path = get_path(self, sel[1]);

                // Layout two panels side-by-side
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    // Left panel
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            let size_half = egui::Vec2::new(available.x * 0.5 - 16.0, available.y - 24.0);
                            let (left_resp, left_painter) = ui.allocate_painter(size_half, egui::Sense::click());
                            let rect = left_resp.rect;
                            left_painter.rect_filled(rect, Rounding::same(2.0), Color32::from_rgb(20,20,22));

                            if let Some(path) = &left_path {
                                // Use current texture if comparing current image
                                let tex_id_opt = if sel[0] == self.current_index { self.current_texture.as_ref().map(|t| t.id()) } else { self.thumbnail_textures.get(path).map(|t| t.id()) };
                                if let Some(tex_id) = tex_id_opt {
                                    let tex_size = self.texture_size_from_id(tex_id);
                                    let base_scale = (rect.width() / tex_size.x).min(rect.height() / tex_size.y);
                                    let scaled = tex_size * base_scale * self.compare_zoom[0];
                                    let inner_rect = Rect::from_center_size(rect.center() + self.compare_pan[0], scaled);
                                    left_painter.image(tex_id, inner_rect, Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(1.0,1.0)), Color32::WHITE);

                                    // Zoom with scroll when hovering
                                    if left_resp.hovered() {
                                        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                                        if scroll != 0.0 {
                                            let factor = 1.0 + scroll * 0.01; // larger per-tick zoom
                                            self.compare_zoom[0] = (self.compare_zoom[0] * factor).clamp(0.1, 16.0);
                                            ui.ctx().request_repaint();

                                            // If we are zooming a thumbnail (not current full image), request a larger preview once
                                            if !self.thumbnail_requests.contains(path) && self.compare_zoom[0] > 1.5 && !self.compare_large_preview_requests.contains(path) {
                                                let path_clone = path.clone();
                                                let tx = self.loader_tx.clone();
                                                let cache = self.image_cache.clone();
                                                self.compare_large_preview_requests.insert(path.clone());
                                                rayon::spawn(move || {
                                                    if let Ok(thumb) = crate::image_loader::load_thumbnail(&path_clone, 2048) {
                                                        cache.insert_thumbnail(path_clone.clone(), thumb.clone());
                                                        let _ = tx.send(super::LoaderMessage::ThumbnailLoaded(path_clone, thumb));
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
                                    left_painter.text(rect.center(), egui::Align2::CENTER_CENTER, self.spinner_char(ui), egui::FontId::proportional(24.0), Color32::from_rgb(130,130,130));
                                }

                                // EXIF overlay for this slot
                                if self.settings.show_exif_overlay {
                                    if let Some(exif) = self.compare_exifs.get(path) {
                                        let overlay_pos = rect.left_bottom() + egui::Vec2::new(12.0, -12.0 - 48.0);
                                        let overlay_rect = Rect::from_min_size(overlay_pos, egui::Vec2::new(280.0, 48.0));
                                        left_painter.rect_filled(overlay_rect, Rounding::same(6.0), Color32::from_rgba_unmultiplied(0,0,0,160));
                                        let camera = exif.camera_model.clone().unwrap_or_else(|| "Unknown".to_string());
                                        left_painter.text(overlay_rect.left_top() + egui::Vec2::new(8.0, 6.0), egui::Align2::LEFT_TOP, camera, egui::FontId::proportional(12.0), Color32::WHITE);
                                        left_painter.text(overlay_rect.left_bottom() + egui::Vec2::new(8.0, -6.0), egui::Align2::LEFT_BOTTOM, format!("{} • {}", exif.focal_length_formatted(), exif.aperture_formatted()), egui::FontId::proportional(11.0), Color32::from_rgb(200,200,200));
                                    } else {
                                        let path_clone = path.clone();
                                        self.spawn_loader(move || {
                                            let exif = crate::exif_data::ExifInfo::from_file(&path_clone);
                                            Some(super::LoaderMessage::ExifLoaded(path_clone, Box::new(exif)))
                                        });
                                    }
                                } else {
                                    if !self.thumbnail_requests.contains(path) {
                                        self.ensure_thumbnail_requested(path, ctx);
                                    }
                                }

                                // Click to make this the current image
                                if left_resp.clicked() {
                                    if let Some(dp) = left_path.clone() {
                                        if let Some(idx) = self.image_list.iter().position(|p| p == &dp) {
                                            // find display index among filtered_list
                                            if let Some(pos) = self.filtered_list.iter().position(|&r| r == idx) {
                                                self.current_index = pos;
                                                self.load_current_image();
                                            }
                                        }
                                    }
                                }
                            } else {
                                left_painter.text(rect.center(), egui::Align2::CENTER_CENTER, "No image", egui::FontId::proportional(18.0), Color32::GRAY);
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Right panel - symmetric to left
                    ui.vertical(|ui| {
                        ui.group(|ui| {
                            let size_half = egui::Vec2::new(available.x * 0.5 - 16.0, available.y - 24.0);
                            let (right_resp, right_painter) = ui.allocate_painter(size_half, egui::Sense::click());
                            let rect = right_resp.rect;
                            right_painter.rect_filled(rect, Rounding::same(2.0), Color32::from_rgb(20,20,22));

                            if let Some(path) = &right_path {
                                let tex_id_opt = if sel[1] == self.current_index { self.current_texture.as_ref().map(|t| t.id()) } else { self.thumbnail_textures.get(path).map(|t| t.id()) };
                                if let Some(tex_id) = tex_id_opt {
                                    let tex_size = self.texture_size_from_id(tex_id);
                                    let base_scale = (rect.width() / tex_size.x).min(rect.height() / tex_size.y);
                                    let scaled = tex_size * base_scale * self.compare_zoom[1];
                                    let inner_rect = Rect::from_center_size(rect.center(), scaled);
                                    right_painter.image(tex_id, inner_rect, Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(1.0,1.0)), Color32::WHITE);

                                    if right_resp.hovered() {
                                        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                                        if scroll != 0.0 {
                                            let factor = 1.0 + scroll * 0.01;
                                            self.compare_zoom[1] = (self.compare_zoom[1] * factor).clamp(0.1, 16.0);
                                            ui.ctx().request_repaint();

                                            if !self.thumbnail_requests.contains(path) && self.compare_zoom[1] > 1.5 && !self.compare_large_preview_requests.contains(path) {
                                                let path_clone = path.clone();
                                                let tx = self.loader_tx.clone();
                                                let cache = self.image_cache.clone();
                                                self.compare_large_preview_requests.insert(path.clone());
                                                rayon::spawn(move || {
                                                    if let Ok(thumb) = crate::image_loader::load_thumbnail(&path_clone, 2048) {
                                                        cache.insert_thumbnail(path_clone.clone(), thumb.clone());
                                                        let _ = tx.send(super::LoaderMessage::ThumbnailLoaded(path_clone, thumb));
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
                                        self.ensure_thumbnail_requested(path, &ctx);
                                    }
                                    right_painter.text(rect.center(), egui::Align2::CENTER_CENTER, self.spinner_char(ui), egui::FontId::proportional(24.0), Color32::from_rgb(130,130,130));
                                }

                                if let Some(exif) = self.compare_exifs.get(path) {
                                    let overlay_pos = rect.left_bottom() + egui::Vec2::new(12.0, -12.0 - 48.0);
                                    let overlay_rect = Rect::from_min_size(overlay_pos, egui::Vec2::new(280.0, 48.0));
                                    right_painter.rect_filled(overlay_rect, Rounding::same(6.0), Color32::from_rgba_unmultiplied(0,0,0,160));
                                    right_painter.text(overlay_rect.left_top() + egui::Vec2::new(8.0, 6.0), egui::Align2::LEFT_TOP, exif.camera_model.clone().unwrap_or_else(|| "Unknown".to_string()), egui::FontId::proportional(12.0), Color32::WHITE);
                                    right_painter.text(overlay_rect.left_bottom() + egui::Vec2::new(8.0, -6.0), egui::Align2::LEFT_BOTTOM, format!("{} • ISO {}", exif.aperture.clone().unwrap_or_default(), exif.iso.clone().unwrap_or_default()), egui::FontId::proportional(11.0), Color32::from_rgb(200,200,200));
                                } else {
                                    let p2 = path.clone();
                                    self.spawn_loader(move || {
                                        let exif = crate::exif_data::ExifInfo::from_file(&p2);
                                        Some(super::LoaderMessage::ExifLoaded(p2, Box::new(exif)))
                                    });
                                }

                                if right_resp.clicked() {
                                    if let Some(dp) = right_path.clone() {
                                        if let Some(idx) = self.image_list.iter().position(|p| p == &dp) {
                                            if let Some(pos) = self.filtered_list.iter().position(|&r| r == idx) {
                                                self.current_index = pos;
                                                self.load_current_image();
                                            }
                                        }
                                    }
                                }
                            } else {
                                right_painter.text(rect.center(), egui::Align2::CENTER_CENTER, "No image", egui::FontId::proportional(18.0), Color32::GRAY);
                            }
                        });
                    });

                    ui.add_space(8.0);
                });

                // Footer controls for compare
                ui.horizontal(|ui| {
                    if ui.button("Swap Sides").clicked() {
                        let temp = sel[0];
                        sel[0] = sel[1];
                        sel[1] = temp;
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

    
    fn handle_image_input(&mut self, response: &egui::Response, ui: &egui::Ui) {
        // Handle touch gestures
        self.handle_touch_gestures(response, ui);
        
        // Pan with drag
        if response.dragged() {
            let delta = response.drag_delta();
            self.pan_offset += delta;
            self.target_pan = self.pan_offset;
        }
        
        // Zoom with scroll
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                let zoom_factor = 1.0 + scroll_delta * 0.001;
                let new_zoom = (self.target_zoom * zoom_factor).clamp(0.1, 20.0);
                
                // Zoom towards mouse position
                if let Some(mouse_pos) = response.hover_pos() {
                    let mouse_rel = mouse_pos - response.rect.center() - self.pan_offset;
                    let zoom_change = new_zoom / self.target_zoom;
                    self.target_pan = self.pan_offset - mouse_rel * (zoom_change - 1.0);
                }
                
                self.target_zoom = new_zoom;
                
                if !self.settings.smooth_zoom {
                    self.zoom = self.target_zoom;
                    self.pan_offset = self.target_pan;
                }
            }
        }
        
        // Double-click to toggle between 100% and fit
        if response.double_clicked() {
            if (self.zoom - 1.0).abs() < 0.1 {
                self.fit_to_window();
            } else {
                self.zoom_to(1.0);
            }
        }
        
        // Right-click context menu
        response.context_menu(|ui| {
            if ui.button("Zoom 100%").clicked() {
                self.zoom_to(1.0);
                ui.close_menu();
            }
            if ui.button("Fit to Window").clicked() {
                self.fit_to_window();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Rotate Left").clicked() {
                self.rotate_left();
                ui.close_menu();
            }
            if ui.button("Rotate Right").clicked() {
                self.rotate_right();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Copy Path").clicked() {
                self.copy_to_clipboard();
                ui.close_menu();
            }
            if ui.button("Open in File Manager").clicked() {
                self.open_in_file_manager();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Set as Wallpaper").clicked() {
                self.set_as_wallpaper();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Delete").clicked() {
                self.delete_current_image();
                ui.close_menu();
            }
        });
        
        // Update loupe position
        if self.settings.loupe_enabled {
            if let Some(pos) = response.hover_pos() {
                self.loupe_position = Some(pos);
            }
        }
        
        // Color picker
        if response.clicked_by(egui::PointerButton::Middle) {
            if let Some(pos) = response.interact_pointer_pos() {
                self.pick_color_at(pos, response.rect);
            }
        }
    }
        fn handle_touch_gestures(&mut self, response: &egui::Response, ui: &egui::Ui) {
        let input = ui.input(|i| i.clone());
        
        // Pinch-to-zoom gesture
        if let Some(multi_touch) = input.multi_touch() {
            if multi_touch.zoom_delta > 1.0 {
                // Calculate zoom factor from pinch
                let zoom_factor = multi_touch.zoom_delta;
                let new_zoom = (self.target_zoom * zoom_factor).clamp(0.1, 20.0);
                
                // Zoom towards the center of the touch points
                let touch_center = multi_touch.start_pos;
                let touch_rel = touch_center - response.rect.center() - self.pan_offset;
                let zoom_change = new_zoom / self.target_zoom;
                self.target_pan = self.pan_offset - touch_rel * (zoom_change - 1.0);
                
                self.target_zoom = new_zoom;
                
                if !self.settings.smooth_zoom {
                    self.zoom = self.target_zoom;
                    self.pan_offset = self.target_pan;
                }
            }
        }
        
        // Swipe navigation (two-finger swipe)
        if input.pointer.secondary_down() && input.pointer.primary_down() {
            let delta = input.pointer.delta();
            if delta.x.abs() > 50.0 { // Threshold for swipe
                if delta.x > 0.0 {
                    self.previous_image();
                } else {
                    self.next_image();
                }
            }
        }
    }
        fn draw_overlays(&self, ui: &mut egui::Ui, image_rect: Rect) {
        // Focus peaking overlay
        if self.settings.show_focus_peaking {
            if let Some(tex) = &self.focus_peaking_texture {
                ui.painter().image(
                    tex.id(),
                    image_rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        
        // Zebra overlay
        if self.settings.show_zebras {
            if let Some(tex) = &self.zebra_texture {
                ui.painter().image(
                    tex.id(),
                    image_rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        
        // Grid overlay
        if self.settings.show_grid_overlay {
            self.draw_grid_overlay(ui, image_rect);
        }
    }
    
    fn draw_grid_overlay(&self, ui: &mut egui::Ui, rect: Rect) {
        let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 100));
        let painter = ui.painter();
        
        match self.settings.grid_type {
            GridType::Off => {}
            GridType::RuleOfThirds => {
                // Vertical lines
                let x1 = rect.left() + rect.width() / 3.0;
                let x2 = rect.left() + rect.width() * 2.0 / 3.0;
                painter.line_segment([egui::pos2(x1, rect.top()), egui::pos2(x1, rect.bottom())], stroke);
                painter.line_segment([egui::pos2(x2, rect.top()), egui::pos2(x2, rect.bottom())], stroke);
                
                // Horizontal lines
                let y1 = rect.top() + rect.height() / 3.0;
                let y2 = rect.top() + rect.height() * 2.0 / 3.0;
                painter.line_segment([egui::pos2(rect.left(), y1), egui::pos2(rect.right(), y1)], stroke);
                painter.line_segment([egui::pos2(rect.left(), y2), egui::pos2(rect.right(), y2)], stroke);
            }
            GridType::GoldenRatio => {
                let phi = 1.618_034;
                
                // Vertical
                let x1 = rect.left() + rect.width() / phi;
                let x2 = rect.right() - rect.width() / phi;
                painter.line_segment([egui::pos2(x1, rect.top()), egui::pos2(x1, rect.bottom())], stroke);
                painter.line_segment([egui::pos2(x2, rect.top()), egui::pos2(x2, rect.bottom())], stroke);
                
                // Horizontal
                let y1 = rect.top() + rect.height() / phi;
                let y2 = rect.bottom() - rect.height() / phi;
                painter.line_segment([egui::pos2(rect.left(), y1), egui::pos2(rect.right(), y1)], stroke);
                painter.line_segment([egui::pos2(rect.left(), y2), egui::pos2(rect.right(), y2)], stroke);
            }
            GridType::Diagonal => {
                painter.line_segment([rect.left_top(), rect.right_bottom()], stroke);
                painter.line_segment([rect.right_top(), rect.left_bottom()], stroke);
            }
            GridType::Center | GridType::Square => {
                let center = rect.center();
                painter.line_segment([egui::pos2(center.x, rect.top()), egui::pos2(center.x, rect.bottom())], stroke);
                painter.line_segment([egui::pos2(rect.left(), center.y), egui::pos2(rect.right(), center.y)], stroke);
            }
        }
    }
    
    fn draw_checkered_background(&self, ui: &mut egui::Ui, rect: Rect) {
        let checker_size = 16.0;
        let color1 = Color32::from_rgb(60, 60, 60);
        let color2 = Color32::from_rgb(80, 80, 80);
        
        let cols = (rect.width() / checker_size).ceil() as i32;
        let rows = (rect.height() / checker_size).ceil() as i32;
        
        for row in 0..rows {
            for col in 0..cols {
                let color = if (row + col) % 2 == 0 { color1 } else { color2 };
                let checker_rect = Rect::from_min_size(
                    egui::pos2(
                        rect.left() + col as f32 * checker_size,
                        rect.top() + row as f32 * checker_size
                    ),
                    Vec2::splat(checker_size)
                ).intersect(rect);
                
                ui.painter().rect_filled(checker_rect, Rounding::ZERO, color);
            }
        }
    }
    
    fn draw_loupe(&self, ui: &mut egui::Ui) {
        if let (Some(pos), Some(tex)) = (&self.loupe_position, &self.current_texture) {
            let loupe_size = self.settings.loupe_size;
            let loupe_zoom = self.settings.loupe_zoom;

            // Calculate image rectangle (same as in render_single_view)
            let _available = ui.available_size();
            let rect = ui.available_rect_before_wrap();
            let tex_size = tex.size_vec2();
            let display_size = tex_size * self.zoom;

            let image_rect = Rect::from_center_size(
                rect.center() + self.pan_offset,
                display_size
            );

            // Loupe circle background (draw slightly larger to hide rectangle corners)
            ui.painter().circle_filled(*pos, loupe_size / 2.0, Color32::BLACK);

            // Clamp sampling position to image to avoid disappearing when cursor is at edges
            let sample_pos = egui::pos2(
                pos.x.clamp(image_rect.left(), image_rect.right()),
                pos.y.clamp(image_rect.top(), image_rect.bottom()),
            );

            // Inset draw rect so image corners don't show outside the circular border
            let draw_rect = Rect::from_center_size(*pos, Vec2::splat(loupe_size * 0.9));

            // Calculate UV coordinates based on sampling position relative to image
            let relative_pos = sample_pos - image_rect.min;
            // Avoid division by zero
            let display_w = if display_size.x.abs() < 1e-6 { 1.0 } else { display_size.x };
            let display_h = if display_size.y.abs() < 1e-6 { 1.0 } else { display_size.y };
            let uv = egui::pos2(relative_pos.x / display_w, relative_pos.y / display_h);

            // Calculate UV radius separately for X and Y to respect aspect ratio
            let mut uv_radius_x = (loupe_size / 2.0) / (display_w * loupe_zoom).max(1e-6);
            let mut uv_radius_y = (loupe_size / 2.0) / (display_h * loupe_zoom).max(1e-6);

            // Ensure minimum non-zero radius to avoid degenerate uv rects
            let min_radius_x = 1.0 / tex.size_vec2().x.max(1.0);
            let min_radius_y = 1.0 / tex.size_vec2().y.max(1.0);
            if uv_radius_x < min_radius_x { uv_radius_x = min_radius_x; }
            if uv_radius_y < min_radius_y { uv_radius_y = min_radius_y; }

            let mut uv_min_x = (uv.x - uv_radius_x).clamp(0.0, 1.0);
            let mut uv_max_x = (uv.x + uv_radius_x).clamp(0.0, 1.0);
            let mut uv_min_y = (uv.y - uv_radius_y).clamp(0.0, 1.0);
            let mut uv_max_y = (uv.y + uv_radius_y).clamp(0.0, 1.0);

            // If any axis collapsed (edge cases), expand slightly to create a tiny region
            if uv_max_x <= uv_min_x {
                let mid = (uv_min_x + uv_max_x) * 0.5;
                uv_min_x = (mid - 0.005).clamp(0.0, 1.0);
                uv_max_x = (mid + 0.005).clamp(0.0, 1.0);
            }
            if uv_max_y <= uv_min_y {
                let mid = (uv_min_y + uv_max_y) * 0.5;
                uv_min_y = (mid - 0.005).clamp(0.0, 1.0);
                uv_max_y = (mid + 0.005).clamp(0.0, 1.0);
            }

            let uv_min = egui::pos2(uv_min_x, uv_min_y);
            let uv_max = egui::pos2(uv_max_x, uv_max_y);

            // Draw zoomed image portion into inset draw rect
            ui.painter().image(
                tex.id(),
                draw_rect,
                Rect::from_min_max(uv_min, uv_max),
                Color32::WHITE,
            );

            // Border
            ui.painter().circle_stroke(*pos, loupe_size / 2.0, Stroke::new(2.0, Color32::WHITE));

            // Crosshair
            ui.painter().line_segment(
                [egui::pos2(pos.x - 10.0, pos.y), egui::pos2(pos.x + 10.0, pos.y)],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 150)),
            );
            ui.painter().line_segment(
                [egui::pos2(pos.x, pos.y - 10.0), egui::pos2(pos.x, pos.y + 10.0)],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 150)),
            );
        }
    }
    
    fn draw_color_info(&self, ui: &mut egui::Ui, rect: Rect) {
        if let Some((r, g, b)) = self.picked_color {
            let info_rect = Rect::from_min_size(
                rect.right_bottom() - Vec2::new(150.0, 80.0),
                Vec2::new(140.0, 70.0)
            );
            
            ui.painter().rect_filled(info_rect, Rounding::same(4.0), Color32::from_rgba_unmultiplied(0, 0, 0, 200));
            
            // Color swatch
            let swatch_rect = Rect::from_min_size(
                info_rect.left_top() + Vec2::new(8.0, 8.0),
                Vec2::splat(30.0)
            );
            ui.painter().rect_filled(swatch_rect, Rounding::same(2.0), Color32::from_rgb(r, g, b));
            
            // RGB values
            ui.painter().text(
                swatch_rect.right_center() + Vec2::new(8.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!("R: {}\nG: {}\nB: {}", r, g, b),
                egui::FontId::monospace(10.0),
                Color32::WHITE,
            );
            
            // Hex value
            ui.painter().text(
                info_rect.left_bottom() + Vec2::new(8.0, -8.0),
                egui::Align2::LEFT_BOTTOM,
                format!("#{:02X}{:02X}{:02X}", r, g, b),
                egui::FontId::monospace(11.0),
                Color32::WHITE,
            );
        }
    }
    
    fn pick_color_at(&mut self, pos: egui::Pos2, view_rect: Rect) {
        if let Some(img) = &self.current_image {
            // Convert screen position to image coordinates
            let img_center = view_rect.center() + self.pan_offset;
            let rel_pos = pos - img_center;
            
            let img_x = (img.width() as f32 / 2.0 + rel_pos.x / self.zoom) as u32;
            let img_y = (img.height() as f32 / 2.0 + rel_pos.y / self.zoom) as u32;
            
            if img_x < img.width() && img_y < img.height() {
                let pixel = img.get_pixel(img_x, img_y);
                self.picked_color = Some((pixel[0], pixel[1], pixel[2]));
            }
        }
    }
}
