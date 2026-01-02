use crate::app::ImageViewerApp;
use egui::{self, Color32, RichText, Vec2, Rect};

// Lightroom-inspired color scheme
const LR_BG_DARK: Color32 = Color32::from_rgb(38, 38, 38);
const LR_BG_PANEL: Color32 = Color32::from_rgb(51, 51, 51);
const LR_TEXT_PRIMARY: Color32 = Color32::from_rgb(200, 200, 200);
const LR_TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 140);
const LR_HEADER_BG: Color32 = Color32::from_rgb(45, 45, 45);

pub fn render_navigator_panel(app: &mut ImageViewerApp, ui: &mut egui::Ui) {
    // Panel header background
    let header_rect = ui.available_rect_before_wrap();
    let header_rect = Rect::from_min_size(
        header_rect.min,
        Vec2::new(ui.available_width(), 24.0)
    );

    ui.painter().rect_filled(header_rect, egui::Rounding::ZERO, LR_HEADER_BG);
    ui.painter().hline(
        header_rect.x_range(),
        header_rect.bottom(),
        egui::Stroke::new(1.0, Color32::from_rgb(28, 28, 28))
    );

    // Header text
    ui.painter().text(
        header_rect.left_center() + Vec2::new(8.0, 0.0),
        egui::Align2::LEFT_CENTER,
        "Navigator",
        egui::FontId::proportional(11.0),
        LR_TEXT_PRIMARY,
    );

    // Contents
    ui.add_space(4.0);
    egui::Frame::none()
        .fill(LR_BG_PANEL)
        .inner_margin(egui::Margin::symmetric(8.0, 6.0))
        .show(ui, |ui| {
            let available_width = ui.available_width();
            let nav_height = 140.0;

            // Zoom level buttons (like Lightroom: FIT, FILL, 1:1, custom)
            ui.horizontal(|ui| {
                ui.add_space(4.0);

                let zoom_btn_style = |is_active: bool| {
                    if is_active { LR_TEXT_PRIMARY } else { LR_TEXT_SECONDARY }
                };

                let fit_active = (app.zoom - 1.0).abs() < 0.01 && app.pan_offset == Vec2::ZERO;
                if ui.add(egui::Button::new(RichText::new("FIT").size(10.0).color(zoom_btn_style(fit_active)))
                    .fill(Color32::TRANSPARENT)
                    .stroke(egui::Stroke::NONE)
                    .min_size(Vec2::new(28.0, 16.0)))
                    .clicked() {
                    app.fit_to_window();
                }

                if ui.add(egui::Button::new(RichText::new("100%").size(10.0).color(zoom_btn_style((app.zoom - 1.0).abs() < 0.01)))
                    .fill(Color32::TRANSPARENT)
                    .stroke(egui::Stroke::NONE)
                    .min_size(Vec2::new(32.0, 16.0)))
                    .clicked() {
                    app.zoom = 1.0;
                    app.target_zoom = 1.0;
                }

                if ui.add(egui::Button::new(RichText::new("25%").size(10.0).color(LR_TEXT_SECONDARY))
                    .fill(Color32::TRANSPARENT)
                    .stroke(egui::Stroke::NONE)
                    .min_size(Vec2::new(28.0, 16.0)))
                    .clicked() {
                    app.zoom = 0.25;
                    app.target_zoom = 0.25;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(4.0);
                    // Show current zoom percentage
                    ui.label(RichText::new(format!("{:.0}%", app.zoom * 100.0))
                        .size(10.0)
                        .color(LR_TEXT_SECONDARY));
                });
            });

            ui.add_space(4.0);

            // Navigator preview with viewport rectangle
            let (response, painter) = ui.allocate_painter(
                Vec2::new(available_width - 8.0, nav_height),
                egui::Sense::click_and_drag()
            );
            let nav_rect = response.rect;

            // Background
            painter.rect_filled(nav_rect, egui::Rounding::ZERO, Color32::from_rgb(34, 34, 34));

            // Draw thumbnail preview
            if let Some(tex) = &app.current_texture {
                let tex_size = tex.size_vec2();

                // Calculate scaled size to fit in navigator
                let scale = (nav_rect.width() / tex_size.x).min(nav_rect.height() / tex_size.y);
                let scaled_size = tex_size * scale;

                let image_rect = Rect::from_center_size(nav_rect.center(), scaled_size);

                painter.image(
                    tex.id(),
                    image_rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );

                // Draw viewport rectangle (showing what's visible in main view)
                if app.zoom > 1.0 || app.pan_offset != Vec2::ZERO {
                    // Calculate the viewport rectangle
                    let view_size = app.available_view_size;
                    let display_size = tex_size * app.zoom;

                    // What portion of the image is visible
                    let visible_width = (view_size.x / display_size.x).min(1.0);
                    let visible_height = (view_size.y / display_size.y).min(1.0);

                    // Calculate center offset as a fraction of image
                    let pan_fraction_x = -app.pan_offset.x / display_size.x;
                    let pan_fraction_y = -app.pan_offset.y / display_size.y;

                    // Viewport center in normalized coordinates (0 to 1)
                    let center_x = 0.5 + pan_fraction_x;
                    let center_y = 0.5 + pan_fraction_y;

                    // Viewport rectangle in normalized coordinates
                    let vp_left = (center_x - visible_width / 2.0).max(0.0);
                    let vp_right = (center_x + visible_width / 2.0).min(1.0);
                    let vp_top = (center_y - visible_height / 2.0).max(0.0);
                    let vp_bottom = (center_y + visible_height / 2.0).min(1.0);

                    // Convert to screen coordinates
                    let vp_screen_left = image_rect.left() + vp_left * image_rect.width();
                    let vp_screen_right = image_rect.left() + vp_right * image_rect.width();
                    let vp_screen_top = image_rect.top() + vp_top * image_rect.height();
                    let vp_screen_bottom = image_rect.top() + vp_bottom * image_rect.height();

                    let vp_rect = Rect::from_min_max(
                        egui::pos2(vp_screen_left, vp_screen_top),
                        egui::pos2(vp_screen_right, vp_screen_bottom)
                    );

                    // Draw viewport rectangle
                    painter.rect_stroke(
                        vp_rect,
                        egui::Rounding::ZERO,
                        egui::Stroke::new(2.0, Color32::from_rgb(255, 255, 255)),
                    );

                    // Handle dragging to pan
                    if response.dragged() {
                        let drag_delta = response.drag_delta();
                        let drag_fraction_x = drag_delta.x / image_rect.width();
                        let drag_fraction_y = drag_delta.y / image_rect.height();

                        app.pan_offset.x -= drag_fraction_x * display_size.x;
                        app.pan_offset.y -= drag_fraction_y * display_size.y;

                        // Clamp pan offset to keep image in view
                        let max_pan_x = (display_size.x - view_size.x) / 2.0;
                        let max_pan_y = (display_size.y - view_size.y) / 2.0;
                        app.pan_offset.x = app.pan_offset.x.clamp(-max_pan_x, max_pan_x);
                        app.pan_offset.y = app.pan_offset.y.clamp(-max_pan_y, max_pan_y);
                    }
                }
            }
        });
}