use crate::app::ImageViewerApp;
use egui::{self, Vec2};

impl ImageViewerApp {
    pub fn render_move_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_move_dialog {
            return;
        }

        egui::Window::new("Move Image to Folder")
            .collapsible(false)
            .resizable(false)
            .default_width(400.0)
            .default_height(300.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Choose a folder to move the current image to:");
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Press M again to move to most recent folder")
                        .small()
                        .weak(),
                );

                ui.add_space(8.0);

                // Show recent/quick move folders
                ui.label("Recent folders:");
                ui.separator();

                let quick_folders = self.settings.quick_move_folders.clone();
                if quick_folders.is_empty() {
                    ui.label("(No recent folders)");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(150.0)
                        .show(ui, |ui| {
                            for (index, folder) in quick_folders.iter().enumerate() {
                                if let Some(folder_name) =
                                    folder.file_name().and_then(|n| n.to_str())
                                {
                                    let folder_path = folder.display().to_string();
                                    let button_text = if index == 0 {
                                        format!("📁 {} (press M)", folder_name)
                                    } else {
                                        format!("📁 {}", folder_name)
                                    };
                                    if ui.button(button_text).on_hover_text(&folder_path).clicked()
                                    {
                                        self.move_to_folder(folder.clone());
                                        self.show_move_dialog = false;
                                    }
                                }
                            }
                        });
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("Choose Folder...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            self.move_to_folder(folder);
                            self.show_move_dialog = false;
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_move_dialog = false;
                    }
                });
            });
    }
}
