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
                    ViewMode::Compare => self.render_compare_view(ui, ctx),
                    ViewMode::Lightbox => {} // Handled separately
                }
            });
    }
    
    fn render_single_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let available = ui.available_size();
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
        } else if self.is_loading {
            // Loading indicator
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Loading...",
                egui::FontId::proportional(24.0),
                Color32::WHITE,
            );
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
    }
    
    fn render_compare_view(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let available = ui.available_size();
        let half_width = available.x / 2.0 - 2.0;
        
        ui.horizontal(|ui| {
            // Left image (current)
            let left_rect = egui::Rect::from_min_size(
                ui.cursor().min,
                Vec2::new(half_width, available.y)
            );
            
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(left_rect), |ui| {
                self.render_compare_image(ui, self.current_index, "Current");
            });
            
            // Separator
            let sep_rect = egui::Rect::from_min_size(
                egui::pos2(left_rect.right(), left_rect.top()),
                Vec2::new(4.0, available.y)
            );
            ui.painter().rect_filled(sep_rect, Rounding::ZERO, Color32::from_rgb(60, 60, 65));
            
            // Right image (compare)
            let right_rect = egui::Rect::from_min_size(
                egui::pos2(sep_rect.right(), left_rect.top()),
                Vec2::new(half_width, available.y)
            );
            
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(right_rect), |ui| {
                if let Some(compare_idx) = self.compare_index {
                    self.render_compare_image(ui, compare_idx, "Compare");
                } else {
                    ui.painter().text(
                        right_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Click a thumbnail to compare",
                        egui::FontId::proportional(16.0),
                        Color32::GRAY,
                    );
                }
            });
        });
    }
    
    fn render_compare_image(&self, ui: &mut egui::Ui, index: usize, label: &str) {
        let rect = ui.available_rect_before_wrap();
        
        // Draw background
        ui.painter().rect_filled(rect, Rounding::ZERO, self.settings.background_color.to_color());
        
        // Get the texture for this index
        let tex = if index == self.current_index {
            self.current_texture.as_ref()
        } else {
            None
        };
        
        if let Some(tex) = tex {
            let tex_size = tex.size_vec2();
            
            // Fit to available space
            let scale = (rect.width() / tex_size.x).min(rect.height() / tex_size.y);
            let display_size = tex_size * scale * self.zoom;
            
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
        }
        
        // Label
        ui.painter().text(
            rect.left_top() + Vec2::new(10.0, 10.0),
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );
    }
    
    fn handle_image_input(&mut self, response: &egui::Response, ui: &egui::Ui) {
        // Pan with drag
        if response.dragged() {
            let delta = response.drag_delta();
            self.pan_offset += delta;
            self.target_pan = self.pan_offset;
        }
        
        // Zoom with scroll
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
        
        // Double-click to reset or zoom to 100%
        if response.double_clicked() {
            if (self.zoom - 1.0).abs() < 0.1 {
                self.reset_view();
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
                self.reset_view();
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
                let phi = 1.618033988749895;
                
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
            
            // Loupe circle background
            let loupe_rect = Rect::from_center_size(*pos, Vec2::splat(loupe_size));
            
            ui.painter().circle_filled(*pos, loupe_size / 2.0, Color32::BLACK);
            
            // Calculate UV coordinates for zoomed portion
            let tex_size = tex.size_vec2();
            let center_uv = (*pos - ui.available_rect_before_wrap().center() - self.pan_offset) / (tex_size * self.zoom);
            let uv_radius = (loupe_size / 2.0) / (tex_size.x * self.zoom * loupe_zoom);
            
            let uv_min = egui::pos2(
                (center_uv.x - uv_radius).clamp(0.0, 1.0),
                (center_uv.y - uv_radius).clamp(0.0, 1.0)
            );
            let uv_max = egui::pos2(
                (center_uv.x + uv_radius).clamp(0.0, 1.0),
                (center_uv.y + uv_radius).clamp(0.0, 1.0)
            );
            
            // Draw zoomed image portion
            ui.painter().image(
                tex.id(),
                loupe_rect,
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
