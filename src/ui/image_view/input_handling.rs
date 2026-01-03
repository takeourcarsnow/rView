use crate::app::ImageViewerApp;
use egui::{self, Rect};
use image::GenericImageView;

impl ImageViewerApp {
    pub(crate) fn handle_image_input(&mut self, response: &egui::Response, ui: &mut egui::Ui) {
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
            if ui.button("Export Image").clicked() {
                self.export_image();
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

    pub(crate) fn handle_touch_gestures(&mut self, response: &egui::Response, ui: &mut egui::Ui) {
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
            if delta.x.abs() > 50.0 {
                // Threshold for swipe
                if delta.x > 0.0 {
                    self.previous_image();
                } else {
                    self.next_image();
                }
            }
        }
    }

    pub(crate) fn pick_color_at(&mut self, pos: egui::Pos2, view_rect: Rect) {
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
