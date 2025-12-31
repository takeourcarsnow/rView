use crate::app::ImageViewerApp;
use crate::settings::ColorLabel;

impl ImageViewerApp {
    pub fn handle_keyboard(&mut self, ctx: &egui::Context) {
        // Handle escape key globally to close dialogs
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.command_palette_open {
                self.command_palette_open = false;
                return;
            }
            if self.show_settings_dialog {
                self.show_settings_dialog = false;
                return;
            }
            if self.show_go_to_dialog {
                self.show_go_to_dialog = false;
                return;
            }
            if self.show_move_dialog {
                self.show_move_dialog = false;
                return;
            }
            if self.slideshow_active {
                self.slideshow_active = false;
                return;
            }
            if self.is_fullscreen {
                self.is_fullscreen = false;
                return;
            }
        }

        // Don't handle other keys if a dialog is open or text input is focused
        let dialogs_open = self.show_settings_dialog || self.show_go_to_dialog ||
                          self.show_move_dialog || self.command_palette_open;

        ctx.input(|i| {
            let ctrl = i.modifiers.ctrl;
            let alt = i.modifiers.alt;
            let shift = i.modifiers.shift;

            // Navigation keys work even when dialogs are open
            if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::A) {
                self.pending_navigate_prev = true;
            }
            if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::D) {
                self.pending_navigate_next = true;
            }
            if i.key_pressed(egui::Key::Home) {
                self.pending_navigate_first = true;
            }
            if i.key_pressed(egui::Key::End) {
                self.pending_navigate_last = true;
            }
            if i.key_pressed(egui::Key::PageUp) {
                self.pending_navigate_page_up = true;
            }
            if i.key_pressed(egui::Key::PageDown) {
                self.pending_navigate_page_down = true;
            }

            // Other keys only work when no dialogs are open
            if !dialogs_open {
                // Zoom
                if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                    self.zoom_in();
                }
                if i.key_pressed(egui::Key::Minus) {
                    self.zoom_out();
                }
                if i.key_pressed(egui::Key::Num0) && !ctrl && !alt {
                    self.reset_view();
                }
                if i.key_pressed(egui::Key::Num1) && !ctrl && !alt {
                    self.zoom_to(1.0);
                }
                if i.key_pressed(egui::Key::Num2) && !ctrl && !alt {
                    self.zoom_to(2.0);
                }

                // (Removed slideshow hotkey - slideshow feature intentionally hidden)


                // Toggle EXIF overlay
                // Toggle EXIF overlay only (E)
                if i.key_pressed(egui::Key::E) {
                    self.settings.show_exif_overlay = !self.settings.show_exif_overlay;
                }
                if i.key_pressed(egui::Key::F11) || (i.key_pressed(egui::Key::F) && !ctrl) {
                    self.is_fullscreen = !self.is_fullscreen;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
                }

                // Compare mode toggle
                if i.key_pressed(egui::Key::C) {
                    self.view_mode = match self.view_mode {
                        crate::app::ViewMode::Compare => crate::app::ViewMode::Single,
                        _ => crate::app::ViewMode::Compare,
                    };
                }

                // Delete
                if i.key_pressed(egui::Key::Delete) {
                    self.delete_current_image();
                }

                // Move to selected folder
                if i.key_pressed(egui::Key::M) && !ctrl {
                    self.show_move_dialog = true;
                }

                // Ratings (with Alt modifier)
                if alt {
                    if i.key_pressed(egui::Key::Num0) { self.set_rating(0); }
                    if i.key_pressed(egui::Key::Num1) { self.set_rating(1); }
                    if i.key_pressed(egui::Key::Num2) { self.set_rating(2); }
                    if i.key_pressed(egui::Key::Num3) { self.set_rating(3); }
                    if i.key_pressed(egui::Key::Num4) { self.set_rating(4); }
                    if i.key_pressed(egui::Key::Num5) { self.set_rating(5); }
                }

                // Color labels (with Ctrl modifier)
                if ctrl {
                    if i.key_pressed(egui::Key::Num1) { self.set_color_label(ColorLabel::Red); }
                    if i.key_pressed(egui::Key::Num2) { self.set_color_label(ColorLabel::Yellow); }
                    if i.key_pressed(egui::Key::Num3) { self.set_color_label(ColorLabel::Green); }
                    if i.key_pressed(egui::Key::Num4) { self.set_color_label(ColorLabel::Blue); }
                    if i.key_pressed(egui::Key::Num5) { self.set_color_label(ColorLabel::Purple); }
                    if i.key_pressed(egui::Key::Num0) { self.set_color_label(ColorLabel::None); }
                }

                // Toggle panels
                if i.key_pressed(egui::Key::T) && !ctrl {
                    self.settings.show_thumbnails = !self.settings.show_thumbnails;
                }
                if i.key_pressed(egui::Key::S) && !ctrl {
                    self.settings.show_sidebar = !self.settings.show_sidebar;
                }
                if i.key_pressed(egui::Key::H) && !ctrl {
                    self.settings.show_histogram = !self.settings.show_histogram;
                }
                if i.key_pressed(egui::Key::P) && !ctrl && !alt {
                    self.toggle_panels();
                }

                // Focus peaking / Zebras
                if ctrl && i.key_pressed(egui::Key::F) {
                    self.settings.show_focus_peaking = !self.settings.show_focus_peaking;
                }
                if ctrl && i.key_pressed(egui::Key::Z) && !shift {
                    self.settings.show_zebras = !self.settings.show_zebras;
                }

                // Undo
                if ctrl && i.key_pressed(egui::Key::Z) && shift {
                    self.undo_last_operation();
                }



                // Lightbox mode
                if i.key_pressed(egui::Key::G) {
                    self.toggle_lightbox_mode();
                }

                // Grid overlay
                if ctrl && i.key_pressed(egui::Key::G) {
                    self.settings.show_grid_overlay = !self.settings.show_grid_overlay;
                }

                // Loupe
                if ctrl && i.key_pressed(egui::Key::L) {
                    self.settings.loupe_enabled = !self.settings.loupe_enabled;
                }

                // Before/After toggle
                if i.key_pressed(egui::Key::Backslash) {
                    self.show_original = !self.show_original;
                    self.refresh_adjustments();
                }

                // Command palette
                if ctrl && i.key_pressed(egui::Key::P) {
                    self.command_palette_open = true;
                    self.command_palette_query.clear();
                }

                // Go to image
                if ctrl && i.key_pressed(egui::Key::G) {
                    self.show_go_to_dialog = true;
                    self.go_to_input.clear();
                }

                // Open file/folder
                if ctrl && i.key_pressed(egui::Key::O) {
                    if shift {
                        self.open_folder_dialog();
                    } else {
                        self.open_file_dialog();
                    }
                }

                // Copy path
                if ctrl && i.key_pressed(egui::Key::C) {
                    self.copy_to_clipboard();
                }
            }
        });
    }
}