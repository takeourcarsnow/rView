use crate::app::ImageViewerApp;
use egui::{self, Vec2};

impl ImageViewerApp {
    pub fn render_go_to_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_go_to_dialog {
            return;
        }

        egui::Window::new("Go to Image")
            .collapsible(false)
            .resizable(false)
            .default_width(250.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Image number:");
                    let response = ui.text_edit_singleline(&mut self.go_to_input);

                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if let Ok(num) = self.go_to_input.parse::<usize>() {
                            if num > 0 && num <= self.filtered_list.len() {
                                self.go_to_index(num - 1);
                                self.show_go_to_dialog = false;
                            }
                        }
                    }

                    response.request_focus();
                });

                ui.label(format!("(1 - {})", self.filtered_list.len()));

                ui.horizontal(|ui| {
                    if ui.button("Go").clicked() {
                        if let Ok(num) = self.go_to_input.parse::<usize>() {
                            if num > 0 && num <= self.filtered_list.len() {
                                self.go_to_index(num - 1);
                                self.show_go_to_dialog = false;
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_go_to_dialog = false;
                    }
                });
            });
    }
}
