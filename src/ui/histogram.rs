use crate::app::ImageViewerApp;
use crate::ui::common;
use egui::{self, Color32, CornerRadius, Vec2};

const LR_BG_INPUT: Color32 = Color32::from_rgb(34, 34, 34);

pub fn render_histogram_panel(app: &ImageViewerApp, ui: &mut egui::Ui) {
    common::lr_collapsible_panel(ui, "Histogram", true, |ui| {
        let height = 80.0;
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width() - 8.0, height),
            egui::Sense::hover(),
        );
        let rect = response.rect;

        // Background
        painter.rect_filled(rect, CornerRadius::same(2), LR_BG_INPUT);

        if let Some(histogram) = &app.histogram_data {
            if histogram.len() >= 3 {
                let w = rect.width() - 4.0;
                let h = rect.height() - 4.0;
                let offset = 2.0;

                // Find max for scaling (use log scale for better visualization)
                let max_val = histogram[0]
                    .iter()
                    .chain(histogram[1].iter())
                    .chain(histogram[2].iter())
                    .max()
                    .copied()
                    .unwrap_or(1) as f32;

                // Draw filled histograms with transparency
                let num_bins = 256.min(histogram[0].len());
                for (i, (&r_val, (&g_val, &b_val))) in histogram[0]
                    .iter()
                    .zip(histogram[1].iter().zip(histogram[2].iter()))
                    .enumerate()
                    .take(num_bins)
                {
                    let x = rect.left() + offset + (i as f32 / 255.0) * w;
                    let base_y = rect.bottom() - offset;

                    // Red channel
                    let r_h = (r_val as f32 / max_val).sqrt() * h;
                    painter.line_segment(
                        [egui::pos2(x, base_y), egui::pos2(x, base_y - r_h)],
                        egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 80, 80, 120)),
                    );

                    // Green channel
                    let g_h = (g_val as f32 / max_val).sqrt() * h;
                    painter.line_segment(
                        [egui::pos2(x, base_y), egui::pos2(x, base_y - g_h)],
                        egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(80, 255, 80, 120)),
                    );

                    // Blue channel
                    let b_h = (b_val as f32 / max_val).sqrt() * h;
                    painter.line_segment(
                        [egui::pos2(x, base_y), egui::pos2(x, base_y - b_h)],
                        egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(80, 80, 255, 120)),
                    );
                }
            }
        }
    });
}
