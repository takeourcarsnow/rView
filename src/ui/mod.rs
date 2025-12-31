use crate::app::{ImageViewerApp, ViewMode, LoaderMessage};
use crate::settings::{Theme, ColorLabel};
use egui::{self, Color32, RichText, Vec2, Rounding, Margin, Rect};

mod toolbar;
mod sidebar;
mod thumbnails;
mod dialogs;
mod image_view;

impl eframe::App for ImageViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::profiler::with_profiler(|p| p.start_timer("ui_update"));
        
        self.ctx = Some(ctx.clone());
        
        // Process async messages
        self.process_loader_messages(ctx);
        
        // Handle keyboard input
        self.handle_keyboard(ctx);
        
        // Update slideshow
        self.update_slideshow(ctx);
        
        // Animate zoom/pan
        self.animate_view(ctx);
        
        // Apply theme
        apply_theme(ctx, &self.settings);
        
        // Handle dropped files
        self.handle_dropped_files(ctx);
        
        // Render dialogs
        self.render_dialogs(ctx);
        
        // Tabs disabled — do not render tab bar
        
        // Render UI based on view mode
        match self.view_mode {
            ViewMode::Single => {
                if self.settings.show_toolbar {
                    self.render_toolbar(ctx);
                }
                if !self.panels_hidden {
                    self.render_sidebar(ctx);
                    self.render_thumbnail_bar(ctx);
                }
                if self.settings.show_statusbar {
                    self.render_statusbar(ctx);
                }
                self.render_main_view(ctx);
            }
            ViewMode::Lightbox => {
                if self.settings.show_toolbar {
                    self.render_toolbar(ctx);
                }
                self.render_lightbox(ctx);
            }
            ViewMode::Compare => {
                if self.settings.show_toolbar {
                    self.render_toolbar(ctx);
                }
                if !self.panels_hidden {
                    self.render_sidebar(ctx);
                }
                if self.settings.show_statusbar {
                    self.render_statusbar(ctx);
                }
                // Call the public wrapper
                self.render_compare_view_public(ctx);
            }
        }
        
        // Process pending navigation actions (deferred to avoid UI blocking)
        if self.pending_navigate_prev { self.previous_image(); }
        if self.pending_navigate_next { self.next_image(); }
        if self.pending_navigate_first { self.go_to_first(); }
        if self.pending_navigate_last { self.go_to_last(); }
        if self.pending_navigate_page_up { for _ in 0..10 { self.previous_image(); } }
        if self.pending_navigate_page_down { for _ in 0..10 { self.next_image(); } }
        if self.pending_fit_to_window { self.fit_to_window_internal(); }
        
        // Reset pending flags
        self.pending_navigate_prev = false;
        self.pending_navigate_next = false;
        self.pending_navigate_first = false;
        self.pending_navigate_last = false;
        self.pending_navigate_page_up = false;
        self.pending_navigate_page_down = false;
        self.pending_fit_to_window = false;
        
        crate::profiler::with_profiler(|p| {
            p.end_timer("ui_update");
            p.increment_counter("ui_updates");
        });
    }
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings.save();
        self.metadata_db.save();
    }
}

impl ImageViewerApp {
    pub fn process_loader_messages(&mut self, ctx: &egui::Context) {
        // Limit the number of messages processed per frame to prevent UI blocking
        let max_messages_per_frame = 10;
        let mut messages_processed = 0;
        
        while messages_processed < max_messages_per_frame {
            match self.loader_rx.try_recv() {
                Ok(msg) => {
                    match msg {
                        LoaderMessage::ImageLoaded(path, image) => {
                            crate::profiler::with_profiler(|p| p.increment_counter("images_loaded"));
                            if self.get_current_path().as_ref() == Some(&path) {
                                self.showing_preview = false;
                                self.set_current_image(&path, image.clone());
                                // Default to fitting the image to the view when it is loaded
                                self.pending_fit_to_window = true;
                            } else {
                                self.image_cache.insert(path.clone(), image.clone());
                            }

                        }
                        LoaderMessage::PreviewLoaded(path, preview) => {
                            crate::profiler::with_profiler(|p| p.increment_counter("previews_loaded"));
                            // Only use preview if we're still waiting for this image and don't have the full one yet
                            if self.get_current_path().as_ref() == Some(&path) && self.is_loading {
                                self.showing_preview = true;
                                self.set_current_image(&path, preview);
                                // If this is a RAW file and the user disabled full-size RAW decoding, stop the loading indicator
                                if crate::image_loader::is_raw_file(&path) && !self.settings.load_raw_full_size {
                                    self.is_loading = false;
                                } else {
                                    self.is_loading = true; // Keep loading indicator for full image
                                }
                            }
                        }
                        LoaderMessage::ProgressiveLoaded(path, progressive) => {
                            crate::profiler::with_profiler(|p| p.increment_counter("progressive_loaded"));
                            // Use progressive image if we're still waiting for the full image
                            if self.get_current_path().as_ref() == Some(&path) && self.is_loading {
                                self.showing_preview = true;
                                self.set_current_image(&path, progressive);
                                // Keep loading indicator for full image
                            }
                        }
                        LoaderMessage::ThumbnailLoaded(path, thumb) => {
                            crate::profiler::with_profiler(|p| p.increment_counter("thumbnails_loaded"));
                            let size = [thumb.width() as usize, thumb.height() as usize];
                            let rgba = thumb.to_rgba8();
                            let pixels = rgba.as_flat_samples();
                            
                            let texture = ctx.load_texture(
                                format!("thumb_{}", path.display()),
                                egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
                                egui::TextureOptions::LINEAR,
                            );
                            
                            self.thumbnail_textures.insert(path, texture);
                        }
                        LoaderMessage::LoadError(path, error) => {
                            crate::profiler::with_profiler(|p| p.increment_counter("load_errors"));
                            log::error!("Failed to load {}: {}", path.display(), error);
                            if self.get_current_path().as_ref() == Some(&path) {
                                self.is_loading = false;
                                self.load_error = Some(error);
                            }
                        }
                        LoaderMessage::ExifLoaded(path, exif) => {
                            crate::profiler::with_profiler(|p| p.increment_counter("exif_loaded"));
                            // Clone the exif info to avoid moving it twice
                            let exif_val = (*exif).clone();
                            self.compare_exifs.insert(path.clone(), exif_val.clone());
                            if self.get_current_path().as_ref() == Some(&path) {
                                self.current_exif = Some(exif_val);
                            }
                        }
                    }
                    messages_processed += 1;
                }
                Err(_) => break, // No more messages
            }
        }
    }
    
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
                          self.command_palette_open;
        
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
                        ViewMode::Compare => ViewMode::Single,
                        _ => ViewMode::Compare,
                    };
                }
                
                // Delete
                if i.key_pressed(egui::Key::Delete) {
                    self.delete_current_image();
                }
                
                // Move to selected folder
                if i.key_pressed(egui::Key::M) && !ctrl {
                    self.move_to_selected_folder();
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
    
    pub fn update_slideshow(&mut self, ctx: &egui::Context) {
        if self.slideshow_active {
            self.slideshow_timer += ctx.input(|i| i.stable_dt);
            
            if self.slideshow_timer >= self.settings.slideshow_interval {
                self.slideshow_timer = 0.0;
                
                if self.settings.slideshow_loop || self.current_index < self.filtered_list.len() - 1 {
                    self.next_image();
                } else {
                    self.slideshow_active = false;
                }
            }
            
            ctx.request_repaint();
        }
    }
    
    pub fn animate_view(&mut self, ctx: &egui::Context) {
        if self.settings.smooth_zoom {
            let dt = ctx.input(|i| i.stable_dt);
            let speed = self.settings.zoom_animation_speed;
            
            // Smooth zoom
            if (self.zoom - self.target_zoom).abs() > 0.001 {
                self.zoom += (self.target_zoom - self.zoom) * speed * dt;
                ctx.request_repaint();
            } else {
                self.zoom = self.target_zoom;
            }
            
            // Smooth pan
            let pan_diff = self.target_pan - self.pan_offset;
            if pan_diff.length() > 0.1 {
                self.pan_offset += pan_diff * speed * dt;
                ctx.request_repaint();
            } else {
                self.pan_offset = self.target_pan;
            }
        }
    }
    
    fn render_statusbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("statusbar")
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(25, 25, 28))
                .inner_margin(Margin::symmetric(12.0, 4.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Image info
                    if let Some(path) = self.get_current_path() {
                        let filename = path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        ui.label(RichText::new(&filename).color(Color32::WHITE).size(12.0));
                        
                        // Dimensions
                        if let Some(tex) = &self.current_texture {
                            let size = tex.size_vec2();
                            ui.label(RichText::new(format!("{}×{}", size.x as u32, size.y as u32))
                                .color(Color32::GRAY).size(11.0));
                        }
                        
                        // File size from EXIF
                        if let Some(exif) = &self.current_exif {
                            if let Some(ref size) = exif.file_size {
                                ui.label(RichText::new(size).color(Color32::GRAY).size(11.0));
                            }
                        }
                        
                        // Preview indicator
                        if self.showing_preview {
                            ui.label(RichText::new("[Preview]")
                                .color(Color32::from_rgb(255, 200, 100)).size(11.0));
                        }
                        
                        // Rating
                        let metadata = self.metadata_db.get(&path);
                        if metadata.rating > 0 {
                            ui.label(RichText::new("★".repeat(metadata.rating as usize))
                                .color(Color32::from_rgb(255, 200, 50)).size(11.0));
                        }
                        
                        // Color label
                        if metadata.color_label != ColorLabel::None {
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 5.0, metadata.color_label.to_color());
                        }
                    }
                    
                    // Spacer
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Status message
                        if let Some((msg, time)) = &self.status_message {
                            if time.elapsed().as_secs() < 3 {
                                ui.label(RichText::new(msg).color(Color32::from_rgb(100, 200, 100)).size(11.0));
                            }
                        }
                        
                        // Zoom level
                        ui.label(RichText::new(format!("{:.0}%", self.zoom * 100.0))
                            .color(Color32::GRAY).size(11.0));
                        
                        // Image counter
                        if !self.filtered_list.is_empty() {
                            ui.label(RichText::new(format!("{} / {}", self.current_index + 1, self.filtered_list.len()))
                                .color(Color32::GRAY).size(11.0));
                        }
                    });
                });
            });
    }
    
    fn render_lightbox(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(self.settings.background_color.to_color()))
            .show(ctx, |ui| {
                let _available = ui.available_size();
                let thumb_size = 150.0;
                let padding = 8.0;

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::splat(padding);
                            
                            // Collect data first to avoid borrow conflicts
                            let items: Vec<(usize, usize, std::path::PathBuf)> = self.filtered_list.iter()
                                .enumerate()
                                .filter_map(|(display_idx, &real_idx)| {
                                    self.image_list.get(real_idx).map(|p| (display_idx, real_idx, p.clone()))
                                })
                                .collect();
                            
                            let mut thumbnails_needed: Vec<std::path::PathBuf> = Vec::new();
                            let mut clicked_index: Option<(usize, bool)> = None; // (index, ctrl held)
                            let mut double_clicked_index: Option<usize> = None;
                            
                            for (display_idx, _real_idx, path) in &items {
                                let is_selected = self.selected_indices.contains(display_idx) || *display_idx == self.current_index;
                                
                                let (response, painter) = ui.allocate_painter(
                                    Vec2::splat(thumb_size),
                                    egui::Sense::click()
                                );
                                
                                let rect = response.rect;
                                
                                // Background
                                let bg_color = if is_selected {
                                    Color32::from_rgb(70, 130, 255)
                                } else if response.hovered() {
                                    Color32::from_rgb(50, 50, 55)
                                } else {
                                    Color32::from_rgb(35, 35, 40)
                                };
                                
                                painter.rect_filled(rect, Rounding::same(6.0), bg_color);
                                
                                // Thumbnail (preserve aspect ratio)
                                if let Some(handle) = self.thumbnail_textures.get(path) {
                                    let inner_rect = rect.shrink(4.0);
                                    // Compute texture size and scale to fit while preserving aspect
                                    let tex_size = self.texture_size_from_id(handle.id());
                                    let scale = (inner_rect.width() / tex_size.x).min(inner_rect.height() / tex_size.y);
                                    let display_size = tex_size * scale;
                                    let image_rect = Rect::from_center_size(inner_rect.center(), display_size);
                                    painter.image(
                                        handle.id(),
                                        image_rect,
                                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        Color32::WHITE,
                                    );
                                } else {
                                    thumbnails_needed.push(path.clone());

                                    // Show spinner placeholder while thumbnail is loading
                                    let spinner = self.spinner_char(ui);
                                    painter.text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        spinner,
                                        egui::FontId::proportional(18.0),
                                        Color32::from_rgb(100, 100, 100),
                                    );
                                    ui.ctx().request_repaint();
                                }
                                
                                // Rating stars
                                let metadata = self.metadata_db.get(path);
                                if metadata.rating > 0 {
                                    painter.text(
                                        rect.left_bottom() + Vec2::new(4.0, -4.0),
                                        egui::Align2::LEFT_BOTTOM,
                                        "★".repeat(metadata.rating as usize),
                                        egui::FontId::proportional(10.0),
                                        Color32::from_rgb(255, 200, 50),
                                    );
                                }
                                
                                // Color label
                                if metadata.color_label != ColorLabel::None {
                                    painter.circle_filled(
                                        rect.right_top() + Vec2::new(-8.0, 8.0),
                                        5.0,
                                        metadata.color_label.to_color(),
                                    );
                                }
                                
                                // Click handling
                                if response.clicked() {
                                    let ctrl = ui.input(|i| i.modifiers.ctrl);
                                    clicked_index = Some((*display_idx, ctrl));
                                }
                                
                                // Double click to view
                                if response.double_clicked() {
                                    double_clicked_index = Some(*display_idx);
                                }
                            }
                            
                            // Now apply state changes after the UI loop
                            for path in thumbnails_needed {
                                self.ensure_thumbnail_requested(&path, ctx);
                            }
                            
                            if let Some((idx, ctrl)) = clicked_index {
                                if ctrl {
                                    if self.selected_indices.contains(&idx) {
                                        self.selected_indices.remove(&idx);
                                    } else {
                                        self.selected_indices.insert(idx);
                                    }
                                } else {
                                    self.selected_indices.clear();
                                    self.current_index = idx;
                                    self.load_current_image();
                                }
                            }
                            
                            if let Some(idx) = double_clicked_index {
                                self.current_index = idx;
                                self.load_current_image();
                                self.view_mode = ViewMode::Single;
                            }
                        });
                    });
            });
    }
}


fn apply_theme(ctx: &egui::Context, settings: &crate::settings::Settings) {
    let mut visuals = match settings.theme {
        Theme::Dark => egui::Visuals::dark(),
        Theme::Light => egui::Visuals::light(),
        Theme::Oled => {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::BLACK;
            visuals.window_fill = Color32::BLACK;
            visuals.extreme_bg_color = Color32::BLACK;
            visuals
        }
        Theme::System => egui::Visuals::dark(),
        Theme::SolarizedDark => {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(0, 43, 54);
            visuals.window_fill = Color32::from_rgb(7, 54, 66);
            visuals.widgets.inactive.bg_fill = Color32::from_rgb(7, 54, 66);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(88, 110, 117);
            visuals.widgets.active.bg_fill = Color32::from_rgb(38, 139, 210);
            visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(131, 148, 150);
            visuals
        }
        Theme::SolarizedLight => {
            let mut visuals = egui::Visuals::light();
            visuals.panel_fill = Color32::from_rgb(238, 232, 213);
            visuals.window_fill = Color32::from_rgb(253, 246, 227);
            visuals.widgets.inactive.bg_fill = Color32::from_rgb(253, 246, 227);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(238, 232, 213);
            visuals.widgets.active.bg_fill = Color32::from_rgb(133, 153, 0);
            visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(101, 123, 131);
            visuals
        }
        Theme::HighContrast => {
            let mut visuals = egui::Visuals::dark();
            visuals.widgets.inactive.fg_stroke.color = Color32::WHITE;
            visuals.widgets.hovered.fg_stroke.color = Color32::YELLOW;
            visuals.widgets.active.fg_stroke.color = Color32::from_rgb(255, 255, 0);
            visuals.widgets.inactive.bg_stroke.color = Color32::WHITE;
            visuals.widgets.hovered.bg_stroke.color = Color32::YELLOW;
            visuals.widgets.active.bg_stroke.color = Color32::from_rgb(255, 255, 0);
            visuals.widgets.inactive.bg_stroke.width = 2.0;
            visuals.widgets.hovered.bg_stroke.width = 3.0;
            visuals.widgets.active.bg_stroke.width = 4.0;
            visuals
        }
        Theme::Blue => {
            let mut visuals = egui::Visuals::dark();
            visuals.widgets.active.bg_fill = Color32::from_rgb(70, 130, 255);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(100, 150, 255);
            visuals.widgets.active.fg_stroke.color = Color32::WHITE;
            visuals
        }
        Theme::Purple => {
            let mut visuals = egui::Visuals::dark();
            visuals.widgets.active.bg_fill = Color32::from_rgb(160, 90, 255);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(180, 110, 255);
            visuals.widgets.active.fg_stroke.color = Color32::WHITE;
            visuals
        }
        Theme::Green => {
            let mut visuals = egui::Visuals::dark();
            visuals.widgets.active.bg_fill = Color32::from_rgb(50, 205, 100);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(70, 225, 120);
            visuals.widgets.active.fg_stroke.color = Color32::WHITE;
            visuals
        }
        Theme::Warm => {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(30, 25, 20);
            visuals.widgets.active.bg_fill = Color32::from_rgb(255, 150, 50);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(255, 170, 70);
            visuals.widgets.active.fg_stroke.color = Color32::from_rgb(30, 25, 20);
            visuals
        }
        Theme::Cool => {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(20, 25, 35);
            visuals.widgets.active.bg_fill = Color32::from_rgb(50, 200, 220);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(70, 220, 240);
            visuals.widgets.active.fg_stroke.color = Color32::from_rgb(20, 25, 35);
            visuals
        }
    };

    // Apply accent color to active elements
    visuals.widgets.active.bg_fill = settings.accent_color.to_color();
    visuals.selection.bg_fill = settings.accent_color.to_color().linear_multiply(0.5);

    ctx.set_visuals(visuals);
}
