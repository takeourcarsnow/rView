use crate::app::ImageViewerApp;
use egui::{self, Color32, Margin, Stroke};

// Use the modules from the parent ui crate
use crate::ui::{navigator, histogram, adjustments, metadata, keywording, folders};

// Lightroom-inspired color scheme
const LR_BG_DARK: Color32 = Color32::from_rgb(38, 38, 38);
const LR_BORDER: Color32 = Color32::from_rgb(28, 28, 28);

impl ImageViewerApp {
    /// Render the navigator panel on the left side of the screen
    pub fn render_navigator_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigator_panel")
            .resizable(true)
            .default_width(200.0)
            .min_width(150.0)
            .max_width(300.0)
            .frame(egui::Frame::none()
                .fill(LR_BG_DARK)
                .stroke(Stroke::new(1.0, LR_BORDER))
                .inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                // If catalog is enabled, show catalog panel and folders, otherwise show navigator
                if self.catalog_enabled && self.catalog_db.is_some() {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            self.render_catalog_panel(ui);
                            ui.separator();
                            self.render_folders_panel(ui);
                        });
                } else {
                    self.render_navigator_panel(ui);
                }
            });
    }

    pub fn render_sidebar(&mut self, ctx: &egui::Context) {
        if !self.settings.show_sidebar {
            return;
        }

        egui::SidePanel::right("sidebar")
            .resizable(true)
            .default_width(280.0)
            .min_width(220.0)
            .max_width(400.0)
            .frame(egui::Frame::none()
                .fill(LR_BG_DARK)
                .stroke(Stroke::new(1.0, LR_BORDER))
                .inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Histogram
                        if self.settings.show_histogram {
                            self.render_histogram_panel(ui);
                        }

                        // Quick Develop / Basic adjustments
                        if self.settings.show_adjustments {
                            self.render_basic_panel(ui);
                        }

                        // EXIF / Metadata
                        if self.settings.show_exif {
                            self.render_metadata_info_panel(ui);
                        }

                        // Keywording / Rating & Labels
                        self.render_keywording_panel(ui);

                        ui.add_space(20.0);
                    });
            });
    }

    fn render_navigator_panel(&mut self, ui: &mut egui::Ui) {
        navigator::render_navigator_panel(self, ui);
    }

    fn render_histogram_panel(&self, ui: &mut egui::Ui) {
        histogram::render_histogram_panel(self, ui);
    }

    fn render_basic_panel(&mut self, ui: &mut egui::Ui) {
        adjustments::render_basic_panel(self, ui);
    }

    fn render_metadata_info_panel(&mut self, ui: &mut egui::Ui) {
        metadata::render_metadata_info_panel(self, ui);
    }

    fn render_keywording_panel(&mut self, ui: &mut egui::Ui) {
        keywording::render_keywording_panel(self, ui);
    }

    fn render_folders_panel(&mut self, ui: &mut egui::Ui) {
        folders::render_folders_panel(self, ui);
    }
}
