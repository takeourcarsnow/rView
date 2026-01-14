use crate::app::ImageViewerApp;
use egui::{self, Color32, Margin, Stroke};

// Use the modules from the parent ui crate
use crate::ui::{adjustments, folders, histogram, metadata, navigator};

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
            .frame(
                egui::Frame::NONE
                    .fill(LR_BG_DARK)
                    .stroke(Stroke::new(1.0, LR_BORDER))
                    .inner_margin(Margin::same(0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_folders_panel(ui);
                        ui.separator();
                        self.render_navigator_panel(ui);
                    });
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
            .frame(
                egui::Frame::NONE
                    .fill(LR_BG_DARK)
                    .stroke(Stroke::new(1.0, LR_BORDER))
                    .inner_margin(Margin::same(0)),
            )
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

                        // Overlays & Frames
                        if self.settings.show_overlays {
                            self.render_overlays_panel(ui);
                        }

                        // EXIF / Metadata
                        if self.settings.show_exif {
                            self.render_metadata_info_panel(ui);
                        }

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

    fn render_folders_panel(&mut self, ui: &mut egui::Ui) {
        folders::render_folders_panel(self, ui);
    }

    fn render_overlays_panel(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Overlays & Frames", |ui| {
            ui.add_space(4.0);

            // Get available overlays
            let overlay_dir = std::path::Path::new("src/images/overlays");
            let available_overlays: Vec<String> = if overlay_dir.exists() {
                std::fs::read_dir(overlay_dir)
                    .ok()
                    .map(|entries| {
                        entries
                            .filter_map(|entry| entry.ok())
                            .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "webp"))
                            .filter_map(|entry| entry.file_name().to_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            ui.horizontal(|ui| {
                ui.label("Overlay:");
                egui::ComboBox::from_id_salt("sidebar_overlay_select")
                    .selected_text(self.settings.selected_overlay.as_deref().unwrap_or("None"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.settings.selected_overlay, None, "None");
                        for overlay in &available_overlays {
                            let selected = self.settings.selected_overlay.as_ref() == Some(overlay);
                            if ui.selectable_label(selected, overlay).clicked() {
                                self.settings.selected_overlay = Some(overlay.clone());
                                // Reload overlay texture
                                self.load_custom_overlay(ui.ctx());
                            }
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Opacity:");
                if ui.add(egui::Slider::new(&mut self.settings.overlay_opacity, 0.0..=1.0)).changed() {
                    // Opacity changed, no need to reload texture
                }
            });

            ui.add_space(8.0);

            // Get available frames
            let frame_dir = std::path::Path::new("src/images/frames");
            let available_frames: Vec<String> = if frame_dir.exists() {
                std::fs::read_dir(frame_dir)
                    .ok()
                    .map(|entries| {
                        entries
                            .filter_map(|entry| entry.ok())
                            .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "webp"))
                            .filter_map(|entry| entry.file_name().to_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            ui.horizontal(|ui| {
                ui.label("Frame:");
                egui::ComboBox::from_id_salt("sidebar_frame_select")
                    .selected_text(self.settings.selected_frame.as_deref().unwrap_or("None"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.settings.selected_frame, None, "None");
                        for frame in &available_frames {
                            let selected = self.settings.selected_frame.as_ref() == Some(frame);
                            if ui.selectable_label(selected, frame).clicked() {
                                self.settings.selected_frame = Some(frame.clone());
                                // Reload frame texture
                                self.load_frame(ui.ctx());
                            }
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Opacity:");
                if ui.add(egui::Slider::new(&mut self.settings.frame_opacity, 0.0..=1.0)).changed() {
                    // Opacity changed, no need to reload texture
                }
            });
        });
    }
}
