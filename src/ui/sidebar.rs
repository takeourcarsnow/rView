use crate::app::ImageViewerApp;
use crate::image_loader::{ImageAdjustments, FilmPreset};
use crate::metadata::FileOperation;
use crate::settings::ColorLabel;
use egui::{self, Color32, RichText, Vec2, Rounding, Margin, Stroke, Rect};
use std::path::PathBuf;

// Use the modules from the parent ui crate
use crate::ui::{navigator, histogram, adjustments, metadata, keywording, folders, sidebar_utils};

// Lightroom-inspired color scheme
const LR_BG_DARK: Color32 = Color32::from_rgb(38, 38, 38);
const LR_BG_PANEL: Color32 = Color32::from_rgb(51, 51, 51);
const LR_BG_INPUT: Color32 = Color32::from_rgb(34, 34, 34);
const LR_BORDER: Color32 = Color32::from_rgb(28, 28, 28);
const LR_TEXT_PRIMARY: Color32 = Color32::from_rgb(200, 200, 200);
const LR_TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 140);
const LR_TEXT_LABEL: Color32 = Color32::from_rgb(180, 180, 180);
const LR_HEADER_BG: Color32 = Color32::from_rgb(45, 45, 45);

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
                self.render_navigator_panel(ui);
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

                        // File Browser (Folders panel like Lightroom)
                        self.render_folders_panel(ui);

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

    fn render_film_emulation_panel(&mut self, ui: &mut egui::Ui, _previous_adjustments: &ImageAdjustments, adjustments_changed: &mut bool) {
        adjustments::render_film_emulation_panel(self, ui, _previous_adjustments, adjustments_changed);
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

    fn render_folder_node(&mut self, ui: &mut egui::Ui, path: PathBuf, depth: usize) {
        folders::render_folder_node(self, ui, path, depth);
    }
}
