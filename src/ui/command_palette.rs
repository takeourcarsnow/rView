use crate::app::ImageViewerApp;
use egui::{self, Color32, Margin, Rounding, Vec2};

impl ImageViewerApp {
    pub fn render_command_palette(&mut self, ctx: &egui::Context) {
        if !self.command_palette_open {
            return;
        }

        // Close on escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.command_palette_open = false;
            return;
        }

        egui::Window::new("Command Palette")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(500.0, 400.0))
            .anchor(egui::Align2::CENTER_TOP, Vec2::new(0.0, 100.0))
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(35, 35, 40))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(Margin::same(12.0)),
            )
            .show(ctx, |ui| {
                // Search input
                let response = ui.add_sized(
                    Vec2::new(ui.available_width(), 32.0),
                    egui::TextEdit::singleline(&mut self.command_palette_query)
                        .hint_text("Type a command...")
                        .font(egui::TextStyle::Heading),
                );
                response.request_focus();

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Commands list
                let commands = self.get_filtered_commands();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for (name, shortcut, action) in commands {
                            let response = ui.add_sized(
                                Vec2::new(ui.available_width(), 28.0),
                                egui::Button::new(egui::RichText::new(&name).size(13.0))
                                    .fill(Color32::TRANSPARENT),
                            );

                            // Show shortcut
                            if !shortcut.is_empty() {
                                ui.painter().text(
                                    response.rect.right_center() - Vec2::new(10.0, 0.0),
                                    egui::Align2::RIGHT_CENTER,
                                    &shortcut,
                                    egui::FontId::monospace(10.0),
                                    Color32::GRAY,
                                );
                            }

                            if response.clicked() {
                                self.execute_command(&action);
                                self.command_palette_open = false;
                            }
                        }
                    });
            });
    }

    fn get_filtered_commands(&self) -> Vec<(String, String, String)> {
        let all_commands = vec![
            ("Open File", "Ctrl+O", "open_file"),
            ("Open Folder", "Ctrl+Shift+O", "open_folder"),
            ("Move to Folder", "M", "move"),
            ("Next Image", "→", "next"),
            ("Previous Image", "←", "previous"),
            ("First Image", "Home", "first"),
            ("Last Image", "End", "last"),
            ("Zoom In", "+", "zoom_in"),
            ("Zoom Out", "-", "zoom_out"),
            ("Actual Size (100%)", "1", "actual_size"),
            ("Rotate Left", "L", "rotate_left"),
            ("Rotate Right", "R", "rotate_right"),
            ("Toggle Fullscreen", "F11", "fullscreen"),
            ("Start Slideshow", "Space", "slideshow"),
            ("Toggle Focus Peaking", "Ctrl+F", "focus_peaking"),
            ("Toggle Zebras", "Alt+Z", "zebras"),
            ("Undo", "Ctrl+Z", "undo"),
            ("Redo", "Ctrl+Y", "redo"),
            ("Toggle Grid Overlay", "Ctrl+G", "grid"),
            ("Toggle Loupe", "Ctrl+L", "loupe"),
            ("Toggle Sidebar", "S", "sidebar"),
            ("Toggle Thumbnails", "T", "thumbnails"),
            ("Toggle EXIF Info", "I", "exif"),
            ("Toggle Histogram", "H", "histogram"),
            ("Grid View", "G", "lightbox"),
            ("Delete Image", "Del", "delete"),
            ("Set as Wallpaper", "", "wallpaper"),
            ("Settings", "", "settings"),
            ("Import to Catalog", "", "catalog_import"),
            ("View All Photos", "", "catalog_all"),
            ("New Collection", "", "catalog_collection"),
        ];

        let query = self.command_palette_query.to_lowercase();

        all_commands
            .into_iter()
            .filter(|(name, _, _)| query.is_empty() || name.to_lowercase().contains(&query))
            .map(|(n, s, a)| (n.to_string(), s.to_string(), a.to_string()))
            .collect()
    }

    fn execute_command(&mut self, action: &str) {
        match action {
            "open_file" => self.open_file_dialog(),
            "open_folder" => self.open_folder_dialog(),
            "move" => self.show_move_dialog = true,
            "next" => self.next_image(),
            "previous" => self.previous_image(),
            "first" => self.go_to_first(),
            "last" => self.go_to_last(),
            "zoom_in" => self.zoom_in(),
            "zoom_out" => self.zoom_out(),
            "actual_size" => self.zoom_to(1.0),
            "rotate_left" => self.rotate_left(),
            "rotate_right" => self.rotate_right(),
            "fullscreen" => self.is_fullscreen = !self.is_fullscreen,
            "slideshow" => self.toggle_slideshow(),
            "focus_peaking" => self.settings.show_focus_peaking = !self.settings.show_focus_peaking,
            "zebras" => self.settings.show_zebras = !self.settings.show_zebras,
            "undo" => self.undo_last_operation(),
            "redo" => self.redo_last_operation(),
            "grid" => self.settings.show_grid_overlay = !self.settings.show_grid_overlay,
            "loupe" => self.settings.loupe_enabled = !self.settings.loupe_enabled,
            "sidebar" => self.settings.show_sidebar = !self.settings.show_sidebar,
            "thumbnails" => self.settings.show_thumbnails = !self.settings.show_thumbnails,
            "exif" => self.settings.show_exif = !self.settings.show_exif,
            "histogram" => self.settings.show_histogram = !self.settings.show_histogram,

            "lightbox" => self.toggle_lightbox_mode(),
            "delete" => self.delete_current_image(),
            "wallpaper" => self.set_as_wallpaper(),
            "settings" => self.show_settings_dialog = true,
            "catalog_import" => {
                if self.catalog_db.is_some() {
                    self.catalog_show_import_dialog = true;
                }
            }
            "catalog_all" => {
                self.catalog_view_active = true;
                self.load_catalog_all_photos();
            }
            "catalog_collection" => {
                if self.catalog_db.is_some() {
                    self.catalog_show_new_collection_dialog = true;
                }
            }
            _ => {}
        }
    }
}
