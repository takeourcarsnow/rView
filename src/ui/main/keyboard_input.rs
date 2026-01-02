use crate::app::ImageViewerApp;
use crate::settings::ColorLabel;

impl ImageViewerApp {
    pub fn handle_keyboard(&mut self, ctx: &egui::Context) {
        self.handle_escape_key(ctx);
        
        let dialogs_open = self.show_settings_dialog || self.show_go_to_dialog || self.command_palette_open;
        
        ctx.input(|i| {
            // Navigation keys work even when dialogs are open
            self.handle_navigation_keys(i);
            
            // Handle M key specially for move dialog
            if i.key_pressed(egui::Key::M) && !i.modifiers.ctrl {
                self.handle_move_key();
                return;
            }
            
            // Handle Ctrl+F for search toggle
            if i.key_pressed(egui::Key::F) && i.modifiers.ctrl {
                self.search_visible = !self.search_visible;
                return;
            }
            
            // Other keys only work when no dialogs are open
            if !dialogs_open && !self.show_move_dialog {
                self.handle_zoom_keys(i);
                self.handle_toggle_keys(i, ctx);
                self.handle_action_keys(i);
                self.handle_modifier_keys(i);
            }
        });
    }
    
    fn handle_escape_key(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return;
        }
        
        let dialogs = [
            (&mut self.command_palette_open, "command_palette"),
            (&mut self.show_settings_dialog, "settings"),
            (&mut self.show_go_to_dialog, "go_to"),
            (&mut self.show_move_dialog, "move"),
            (&mut self.batch_rename_state.show_dialog, "batch_rename"),
        ];
        
        for (flag, _) in dialogs {
            if *flag {
                *flag = false;
                return;
            }
        }
        
        if self.slideshow_active {
            self.slideshow_active = false;
            return;
        }
        
        if self.search_visible {
            self.search_visible = false;
            return;
        }
        
        if self.is_fullscreen {
            self.is_fullscreen = false;
        }
    }
    
    fn handle_navigation_keys(&mut self, i: &egui::InputState) {
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
    }
    
    fn handle_move_key(&mut self) {
        if self.show_move_dialog {
            if let Some(recent_folder) = self.settings.quick_move_folders.first() {
                self.move_to_folder(recent_folder.clone());
                self.show_move_dialog = false;
            }
        } else {
            self.show_move_dialog = true;
        }
    }
    
    fn handle_zoom_keys(&mut self, i: &egui::InputState) {
        let ctrl = i.modifiers.ctrl;
        let alt = i.modifiers.alt;
        
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
    }
    
    fn handle_toggle_keys(&mut self, i: &egui::InputState, ctx: &egui::Context) {
        let ctrl = i.modifiers.ctrl;
        let alt = i.modifiers.alt;
        let shift = i.modifiers.shift;
        
        if i.key_pressed(egui::Key::E) {
            self.settings.show_exif_overlay = !self.settings.show_exif_overlay;
        }
        if i.key_pressed(egui::Key::F11) || (i.key_pressed(egui::Key::F) && !ctrl) {
            self.is_fullscreen = !self.is_fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
        }
        if i.key_pressed(egui::Key::C) && !ctrl {
            self.view_mode = match self.view_mode {
                crate::app::ViewMode::Compare => crate::app::ViewMode::Single,
                _ => crate::app::ViewMode::Compare,
            };
        }
        if i.key_pressed(egui::Key::T) && !ctrl {
            self.settings.show_thumbnails = !self.settings.show_thumbnails;
        }
        if i.key_pressed(egui::Key::S) && !ctrl {
            self.settings.show_sidebar = !self.settings.show_sidebar;
        }
        if i.key_pressed(egui::Key::H) && !ctrl {
            self.settings.show_histogram = !self.settings.show_histogram;
        }
        if i.key_pressed(egui::Key::A) && !ctrl && !alt {
            self.settings.show_adjustments = !self.settings.show_adjustments;
        }
        if i.key_pressed(egui::Key::P) && !ctrl && !alt {
            self.toggle_panels();
        }
        if ctrl && i.key_pressed(egui::Key::F) {
            self.settings.show_focus_peaking = !self.settings.show_focus_peaking;
        }
        if ctrl && i.key_pressed(egui::Key::Z) && !shift {
            self.settings.show_zebras = !self.settings.show_zebras;
        }
        if ctrl && i.key_pressed(egui::Key::Z) && shift {
            self.undo_last_operation();
        }
        if i.key_pressed(egui::Key::G) && !ctrl {
            self.toggle_lightbox_mode();
        }
        if ctrl && i.key_pressed(egui::Key::G) {
            self.settings.show_grid_overlay = !self.settings.show_grid_overlay;
        }
        if ctrl && i.key_pressed(egui::Key::L) {
            self.settings.loupe_enabled = !self.settings.loupe_enabled;
        }
        if i.key_pressed(egui::Key::Backslash) {
            self.show_original = !self.show_original;
            self.refresh_adjustments();
        }
    }
    
    fn handle_action_keys(&mut self, i: &egui::InputState) {
        let ctrl = i.modifiers.ctrl;
        let shift = i.modifiers.shift;
        
        if i.key_pressed(egui::Key::Delete) {
            self.delete_current_image();
        }
        if i.key_pressed(egui::Key::F2) {
            self.handle_batch_rename_key();
        }
        if ctrl && i.key_pressed(egui::Key::P) {
            self.command_palette_open = true;
            self.command_palette_query.clear();
        }
        if ctrl && i.key_pressed(egui::Key::G) {
            self.show_go_to_dialog = true;
            self.go_to_input.clear();
        }
        if ctrl && i.key_pressed(egui::Key::O) {
            if shift {
                self.open_folder_dialog();
            } else {
                self.open_file_dialog();
            }
        }
        if ctrl && i.key_pressed(egui::Key::C) {
            self.copy_to_clipboard();
        }
        if ctrl && i.key_pressed(egui::Key::A) {
            // Select all images
            self.selected_indices.clear();
            for i in 0..self.filtered_list.len() {
                self.selected_indices.insert(i);
            }
        }
    }
    
    fn handle_modifier_keys(&mut self, i: &egui::InputState) {
        let ctrl = i.modifiers.ctrl;
        let alt = i.modifiers.alt;
        
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
    }
}