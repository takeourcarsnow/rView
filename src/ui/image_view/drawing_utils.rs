use crate::app::ImageViewerApp;
use crate::settings::GridType;
use egui::{self, Color32, CornerRadius, Rect, Stroke, StrokeKind, Vec2};

impl ImageViewerApp {
    pub(crate) fn draw_overlays(&self, ui: &mut egui::Ui, image_rect: Rect) {
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

        // Custom overlay
        if self.settings.show_custom_overlay {
            if let Some(tex) = &self.custom_overlay_texture {
                let alpha = (self.settings.overlay_opacity * 255.0) as u8;
                ui.painter().image(
                    tex.id(),
                    image_rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                );
            }
        }

        // Frame (drawn after overlays so it appears on top)
        if self.settings.show_frame {
            if let Some(tex) = &self.frame_texture {
                let alpha = (self.settings.frame_opacity * 255.0) as u8;
                ui.painter().image(
                    tex.id(),
                    image_rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                );
            }
        }

        // Crop overlay
        if self.crop_mode {
            self.draw_crop_overlay(ui, image_rect);
        }

        // Grid overlay
        if self.settings.show_grid_overlay {
            self.draw_grid_overlay(ui, image_rect);
        }
    }

    pub(crate) fn draw_grid_overlay(&self, ui: &mut egui::Ui, rect: Rect) {
        let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 100));
        let painter = ui.painter();

        match self.settings.grid_type {
            GridType::Off => {}
            GridType::RuleOfThirds => {
                // Vertical lines
                let x1 = rect.left() + rect.width() / 3.0;
                let x2 = rect.left() + rect.width() * 2.0 / 3.0;
                painter.line_segment(
                    [egui::pos2(x1, rect.top()), egui::pos2(x1, rect.bottom())],
                    stroke,
                );
                painter.line_segment(
                    [egui::pos2(x2, rect.top()), egui::pos2(x2, rect.bottom())],
                    stroke,
                );

                // Horizontal lines
                let y1 = rect.top() + rect.height() / 3.0;
                let y2 = rect.top() + rect.height() * 2.0 / 3.0;
                painter.line_segment(
                    [egui::pos2(rect.left(), y1), egui::pos2(rect.right(), y1)],
                    stroke,
                );
                painter.line_segment(
                    [egui::pos2(rect.left(), y2), egui::pos2(rect.right(), y2)],
                    stroke,
                );
            }
            GridType::GoldenRatio => {
                let phi = 1.618_034;

                // Vertical
                let x1 = rect.left() + rect.width() / phi;
                let x2 = rect.right() - rect.width() / phi;
                painter.line_segment(
                    [egui::pos2(x1, rect.top()), egui::pos2(x1, rect.bottom())],
                    stroke,
                );
                painter.line_segment(
                    [egui::pos2(x2, rect.top()), egui::pos2(x2, rect.bottom())],
                    stroke,
                );

                // Horizontal
                let y1 = rect.top() + rect.height() / phi;
                let y2 = rect.bottom() - rect.height() / phi;
                painter.line_segment(
                    [egui::pos2(rect.left(), y1), egui::pos2(rect.right(), y1)],
                    stroke,
                );
                painter.line_segment(
                    [egui::pos2(rect.left(), y2), egui::pos2(rect.right(), y2)],
                    stroke,
                );
            }
            GridType::Diagonal => {
                painter.line_segment([rect.left_top(), rect.right_bottom()], stroke);
                painter.line_segment([rect.right_top(), rect.left_bottom()], stroke);
            }
            GridType::Center | GridType::Square => {
                let center = rect.center();
                painter.line_segment(
                    [
                        egui::pos2(center.x, rect.top()),
                        egui::pos2(center.x, rect.bottom()),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        egui::pos2(rect.left(), center.y),
                        egui::pos2(rect.right(), center.y),
                    ],
                    stroke,
                );
            }
        }
    }

    pub(crate) fn draw_checkered_background(&self, ui: &mut egui::Ui, rect: Rect) {
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
                        rect.top() + row as f32 * checker_size,
                    ),
                    Vec2::splat(checker_size),
                )
                .intersect(rect);

                ui.painter()
                    .rect_filled(checker_rect, CornerRadius::ZERO, color);
            }
        }
    }

    pub(crate) fn draw_loupe(&self, ui: &mut egui::Ui) {
        if let (Some(pos), Some(tex)) = (&self.loupe_position, &self.current_texture) {
            let loupe_size = self.settings.loupe_size;
            let loupe_zoom = self.settings.loupe_zoom;

            // Calculate image rectangle (same as in render_single_view)
            let rect = ui.available_rect_before_wrap();
            let tex_size = tex.size_vec2();
            let display_size = tex_size * self.zoom;

            // Guard against degenerate display sizes
            if display_size.x.abs() < 1e-6 || display_size.y.abs() < 1e-6 {
                return;
            }

            let image_rect = Rect::from_center_size(rect.center() + self.pan_offset, display_size);

            // Check if cursor is over the image
            if !image_rect.contains(*pos) {
                return;
            }

            // Draw background circle to mask the corners
            ui.painter()
                .circle_filled(*pos, loupe_size / 2.0, Color32::BLACK);

            // Draw rectangle where the magnified image will be painted (slightly inset to fit inside circle)
            let draw_size = Vec2::splat(loupe_size * 0.9);
            let draw_rect = Rect::from_center_size(*pos, draw_size);

            // Calculate the position in the original texture coordinates
            let relative_pos = *pos - image_rect.min;
            let texture_uv = relative_pos / display_size;

            // Size of the area to sample in texture coordinates
            let sample_size_uv = draw_size / (tex_size * loupe_zoom);

            // Calculate UV rectangle centered on the cursor position
            let uv_min = (texture_uv - sample_size_uv / 2.0).clamp(Vec2::ZERO, Vec2::splat(1.0));
            let uv_max = (texture_uv + sample_size_uv / 2.0).clamp(Vec2::ZERO, Vec2::splat(1.0));

            // Paint the magnified portion into the draw_rect
            ui.painter().image(
                tex.id(),
                draw_rect,
                Rect::from_min_max(uv_min.to_pos2(), uv_max.to_pos2()),
                Color32::WHITE,
            );

            // Border
            ui.painter()
                .circle_stroke(*pos, loupe_size / 2.0, Stroke::new(2.0, Color32::WHITE));

            // Crosshair
            ui.painter().line_segment(
                [
                    egui::pos2(pos.x - 10.0, pos.y),
                    egui::pos2(pos.x + 10.0, pos.y),
                ],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 150)),
            );
            ui.painter().line_segment(
                [
                    egui::pos2(pos.x, pos.y - 10.0),
                    egui::pos2(pos.x, pos.y + 10.0),
                ],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 150)),
            );
        }
    }

    pub(crate) fn draw_color_info(&self, ui: &mut egui::Ui, rect: Rect) {
        if let Some((r, g, b)) = self.picked_color {
            let info_rect = Rect::from_min_size(
                rect.right_bottom() - Vec2::new(150.0, 80.0),
                Vec2::new(140.0, 70.0),
            );

            ui.painter().rect_filled(
                info_rect,
                CornerRadius::same(4),
                Color32::from_rgba_unmultiplied(0, 0, 0, 200),
            );

            // Color swatch
            let swatch_rect = Rect::from_min_size(
                info_rect.left_top() + Vec2::new(8.0, 8.0),
                Vec2::splat(30.0),
            );
            ui.painter().rect_filled(
                swatch_rect,
                CornerRadius::same(2),
                Color32::from_rgb(r, g, b),
            );

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

    pub(crate) fn draw_crop_overlay(&self, ui: &mut egui::Ui, image_rect: Rect) {
        let painter = ui.painter();

        // Draw semi-transparent overlay outside crop area
        if let Some(crop_rect) = self.crop_rect {
            let overlay_color = Color32::from_rgba_unmultiplied(0, 0, 0, 150);

            // Top area
            if crop_rect.min.y > image_rect.min.y {
                let top_rect = Rect::from_min_max(
                    egui::pos2(image_rect.min.x, image_rect.min.y),
                    egui::pos2(image_rect.max.x, crop_rect.min.y),
                );
                painter.rect_filled(top_rect, CornerRadius::ZERO, overlay_color);
            }

            // Bottom area
            if crop_rect.max.y < image_rect.max.y {
                let bottom_rect = Rect::from_min_max(
                    egui::pos2(image_rect.min.x, crop_rect.max.y),
                    egui::pos2(image_rect.max.x, image_rect.max.y),
                );
                painter.rect_filled(bottom_rect, CornerRadius::ZERO, overlay_color);
            }

            // Left area
            if crop_rect.min.x > image_rect.min.x {
                let left_rect = Rect::from_min_max(
                    egui::pos2(image_rect.min.x, crop_rect.min.y),
                    egui::pos2(crop_rect.min.x, crop_rect.max.y),
                );
                painter.rect_filled(left_rect, CornerRadius::ZERO, overlay_color);
            }

            // Right area
            if crop_rect.max.x < image_rect.max.x {
                let right_rect = Rect::from_min_max(
                    egui::pos2(crop_rect.max.x, crop_rect.min.y),
                    egui::pos2(image_rect.max.x, crop_rect.max.y),
                );
                painter.rect_filled(right_rect, CornerRadius::ZERO, overlay_color);
            }

            // Draw crop rectangle border
            let border_stroke = Stroke::new(2.0, Color32::from_rgb(255, 255, 255));
            painter.rect_stroke(
                crop_rect,
                CornerRadius::ZERO,
                border_stroke,
                StrokeKind::Inside,
            );

            // Draw corner handles
            let handle_size = 8.0;
            let handle_color = Color32::from_rgb(255, 255, 255);
            let handle_stroke = Stroke::new(1.0, Color32::from_rgb(0, 0, 0));

            // Corner positions
            let corners = [
                crop_rect.min,                                // top-left
                egui::pos2(crop_rect.max.x, crop_rect.min.y), // top-right
                crop_rect.max,                                // bottom-right
                egui::pos2(crop_rect.min.x, crop_rect.max.y), // bottom-left
            ];

            for corner in corners {
                let handle_rect = Rect::from_center_size(corner, Vec2::splat(handle_size));
                painter.rect_filled(handle_rect, CornerRadius::ZERO, handle_color);
                painter.rect_stroke(
                    handle_rect,
                    CornerRadius::ZERO,
                    handle_stroke,
                    StrokeKind::Inside,
                );
            }
        }
    }
}
