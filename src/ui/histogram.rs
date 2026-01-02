use crate::app::ImageViewerApp;
use egui::{self, Color32, RichText, Vec2, Rounding, Stroke, Rect};

// Lightroom-inspired color scheme
const LR_BG_INPUT: Color32 = Color32::from_rgb(34, 34, 34);

pub fn render_histogram_panel(app: &ImageViewerApp, ui: &mut egui::Ui) {
    lr_collapsible_panel(ui, "Histogram", true, |ui| {
        let height = 80.0;
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width() - 8.0, height),
            egui::Sense::hover()
        );
        let rect = response.rect;

        // Background
        painter.rect_filled(rect, Rounding::same(2.0), LR_BG_INPUT);

        if let Some(histogram) = &app.histogram_data {
            if histogram.len() >= 3 {
                let w = rect.width() - 4.0;
                let h = rect.height() - 4.0;
                let offset = 2.0;

                // Find max for scaling (use log scale for better visualization)
                let max_val = histogram[0].iter()
                    .chain(histogram[1].iter())
                    .chain(histogram[2].iter())
                    .max()
                    .copied()
                    .unwrap_or(1) as f32;

                // Draw filled histograms with transparency
                let num_bins = 256.min(histogram[0].len());
                for i in 0..num_bins {
                    let x = rect.left() + offset + (i as f32 / 255.0) * w;
                    let base_y = rect.bottom() - offset;

                    // Red channel
                    let r_h = (histogram[0][i] as f32 / max_val).sqrt() * h;
                    painter.line_segment(
                        [egui::pos2(x, base_y), egui::pos2(x, base_y - r_h)],
                        egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 80, 80, 120)),
                    );

                    // Green channel
                    let g_h = (histogram[1][i] as f32 / max_val).sqrt() * h;
                    painter.line_segment(
                        [egui::pos2(x, base_y), egui::pos2(x, base_y - g_h)],
                        egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(80, 255, 80, 120)),
                    );

                    // Blue channel
                    let b_h = (histogram[2][i] as f32 / max_val).sqrt() * h;
                    painter.line_segment(
                        [egui::pos2(x, base_y), egui::pos2(x, base_y - b_h)],
                        egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(80, 80, 255, 120)),
                    );
                }
            }
        }
    });
}

// Lightroom-style collapsible panel
fn lr_collapsible_panel<R>(
    ui: &mut egui::Ui,
    title: &str,
    default_open: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::CollapsingResponse<R> {
    // Panel header background
    let header_rect = ui.available_rect_before_wrap();
    let header_rect = Rect::from_min_size(
        header_rect.min,
        Vec2::new(ui.available_width(), 24.0)
    );

    ui.painter().rect_filled(header_rect, Rounding::ZERO, Color32::from_rgb(45, 45, 45));
    ui.painter().hline(
        header_rect.x_range(),
        header_rect.bottom(),
        Stroke::new(1.0, Color32::from_rgb(28, 28, 28))
    );

    let response = egui::CollapsingHeader::new(RichText::new(title).size(11.0).color(Color32::from_rgb(200, 200, 200)).strong())
        .default_open(default_open)
        .show(ui, |ui| {
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(Color32::from_rgb(51, 51, 51))
                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    add_contents(ui)
                }).inner
        });

    // Bottom border
    ui.painter().hline(
        ui.available_rect_before_wrap().x_range(),
        ui.cursor().top(),
        Stroke::new(1.0, Color32::from_rgb(28, 28, 28))
    );

    response
}