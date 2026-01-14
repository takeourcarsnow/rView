use crate::app::ImageViewerApp;
use crate::ui::common;
use egui::{self, RichText};

pub fn render_metadata_info_panel(app: &mut ImageViewerApp, ui: &mut egui::Ui) {
    common::lr_collapsible_panel(ui, "Metadata", true, |ui| {
        if let Some(exif) = &app.current_exif {
            if !exif.has_data() {
                ui.label(
                    RichText::new("No EXIF data")
                        .size(10.0)
                        .color(common::LR_TEXT_SECONDARY),
                );
            } else {
                // Camera info
                common::lr_info_row(ui, "Camera", exif.camera_model.as_deref());
                common::lr_info_row(ui, "Lens", exif.lens.as_deref());

                let fl = exif.focal_length_formatted();
                if !fl.is_empty() {
                    common::lr_info_row(ui, "Focal Length", Some(&fl));
                }

                let ap = exif.aperture_formatted();
                if !ap.is_empty() {
                    common::lr_info_row(ui, "Aperture", Some(&ap));
                }

                common::lr_info_row(ui, "Shutter", exif.shutter_speed.as_deref());
                common::lr_info_row(ui, "ISO", exif.iso.as_deref());
                common::lr_info_row(ui, "Date", exif.date_taken.as_deref());
                common::lr_info_row(ui, "Dimensions", exif.dimensions.as_deref());

                if exif.gps_latitude.is_some() && exif.gps_longitude.is_some() {
                    let gps = format!(
                        "{:.4}, {:.4}",
                        exif.gps_latitude.unwrap_or(0.0),
                        exif.gps_longitude.unwrap_or(0.0)
                    );
                    common::lr_info_row(ui, "GPS", Some(&gps));
                }
            }
        } else {
            ui.label(
                RichText::new("No metadata")
                    .size(10.0)
                    .color(common::LR_TEXT_SECONDARY),
            );
        }
    });
}
