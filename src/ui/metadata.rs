use crate::app::ImageViewerApp;
use egui::{self, Color32, RichText, Vec2, Rounding, Stroke, Rect};

// Lightroom-inspired color scheme
const LR_BG_PANEL: Color32 = Color32::from_rgb(51, 51, 51);
const LR_BORDER: Color32 = Color32::from_rgb(28, 28, 28);
const LR_TEXT_PRIMARY: Color32 = Color32::from_rgb(200, 200, 200);
const LR_TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 140);
const LR_HEADER_BG: Color32 = Color32::from_rgb(45, 45, 45);

pub fn render_metadata_info_panel(app: &mut ImageViewerApp, ui: &mut egui::Ui) {
    lr_collapsible_panel(ui, "Metadata", true, |ui| {
        if let Some(exif) = &app.current_exif {
            if !exif.has_data() {
                ui.label(RichText::new("No EXIF data").size(10.0).color(LR_TEXT_SECONDARY));
            } else {
                // Camera info
                lr_info_row(ui, "Camera", exif.camera_model.as_deref());
                lr_info_row(ui, "Lens", exif.lens.as_deref());

                let fl = exif.focal_length_formatted();
                if !fl.is_empty() { lr_info_row(ui, "Focal Length", Some(&fl)); }

                let ap = exif.aperture_formatted();
                if !ap.is_empty() { lr_info_row(ui, "Aperture", Some(&ap)); }

                lr_info_row(ui, "Shutter", exif.shutter_speed.as_deref());
                lr_info_row(ui, "ISO", exif.iso.as_deref());
                lr_info_row(ui, "Date", exif.date_taken.as_deref());
                lr_info_row(ui, "Dimensions", exif.dimensions.as_deref());

                if exif.gps_latitude.is_some() && exif.gps_longitude.is_some() {
                    let gps = format!("{:.4}, {:.4}",
                        exif.gps_latitude.unwrap_or(0.0),
                        exif.gps_longitude.unwrap_or(0.0));
                    lr_info_row(ui, "GPS", Some(&gps));
                }
            }
        } else {
            ui.label(RichText::new("No metadata").size(10.0).color(LR_TEXT_SECONDARY));
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

    ui.painter().rect_filled(header_rect, Rounding::ZERO, LR_HEADER_BG);
    ui.painter().hline(
        header_rect.x_range(),
        header_rect.bottom(),
        Stroke::new(1.0, LR_BORDER)
    );

    let response = egui::CollapsingHeader::new(RichText::new(title).size(11.0).color(LR_TEXT_PRIMARY).strong())
        .default_open(default_open)
        .show(ui, |ui| {
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(LR_BG_PANEL)
                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                add_contents(ui)
            }).inner
        });

    // Bottom border
    ui.painter().hline(
        ui.available_rect_before_wrap().x_range(),
        ui.cursor().top(),
        Stroke::new(1.0, LR_BORDER)
    );

    response
}

// Lightroom-style info row (label: value)
fn lr_info_row(ui: &mut egui::Ui, label: &str, value: Option<&str>) {
    if let Some(v) = value {
        if !v.is_empty() && v != "Unknown" {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{}:", label)).size(10.0).color(LR_TEXT_SECONDARY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(v).size(10.0).color(LR_TEXT_PRIMARY));
                });
            });
        }
    }
}