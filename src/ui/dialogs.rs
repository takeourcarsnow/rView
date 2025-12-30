use crate::app::ImageViewerApp;
use crate::settings::{Theme, BackgroundColor, ThumbnailPosition, FocusPeakingColor, GridType};
use egui::{self, Color32, Vec2, Rounding, Margin, RichText};

impl ImageViewerApp {
    pub fn render_dialogs(&mut self, ctx: &egui::Context) {
        self.render_settings_dialog(ctx);
        self.render_go_to_dialog(ctx);
        self.render_command_palette(ctx);
    }
    
    fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_settings_dialog {
            return;
        }
        
        // Close on escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_settings_dialog = false;
            return;
        }
        
        let screen_rect = ctx.screen_rect();
        let max_height = (screen_rect.height() - 100.0).max(300.0);
        
        egui::Window::new("⚙ Settings")
            .collapsible(false)
            .resizable(true)
            .default_width(480.0)
            .max_height(max_height)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(max_height - 80.0)
                    .show(ui, |ui| {
                        // Theme
                        ui.heading("Appearance");
                        ui.add_space(4.0);
                        
                        ui.horizontal(|ui| {
                            ui.label("Theme:");
                            egui::ComboBox::from_id_salt("theme_combo")
                                .selected_text(format!("{:?}", self.settings.theme))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.settings.theme, Theme::Dark, "Dark");
                                    ui.selectable_value(&mut self.settings.theme, Theme::Light, "Light");
                                    ui.selectable_value(&mut self.settings.theme, Theme::Oled, "OLED Black");
                                    ui.selectable_value(&mut self.settings.theme, Theme::System, "System");
                                });
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Background:");
                            egui::ComboBox::from_id_salt("bg_combo")
                                .selected_text(format!("{:?}", self.settings.background_color))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.settings.background_color, BackgroundColor::Black, "Black");
                                    ui.selectable_value(&mut self.settings.background_color, BackgroundColor::Dark, "Dark");
                                    ui.selectable_value(&mut self.settings.background_color, BackgroundColor::Gray, "Gray");
                                    ui.selectable_value(&mut self.settings.background_color, BackgroundColor::Light, "Light");
                                    ui.selectable_value(&mut self.settings.background_color, BackgroundColor::Checkered, "Checkered");
                                });
                        });
                        
                        ui.add_space(12.0);
                        ui.heading("Thumbnails");
                        ui.add_space(4.0);
                        
                        ui.horizontal(|ui| {
                            ui.label("Position:");
                            egui::ComboBox::from_id_salt("thumb_pos")
                                .selected_text(format!("{:?}", self.settings.thumbnail_position))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.settings.thumbnail_position, ThumbnailPosition::Bottom, "Bottom");
                                    ui.selectable_value(&mut self.settings.thumbnail_position, ThumbnailPosition::Top, "Top");
                                    ui.selectable_value(&mut self.settings.thumbnail_position, ThumbnailPosition::Left, "Left");
                                    ui.selectable_value(&mut self.settings.thumbnail_position, ThumbnailPosition::Right, "Right");
                                });
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Size:");
                            ui.add(egui::Slider::new(&mut self.settings.thumbnail_size, 50.0..=200.0).suffix("px"));
                        });
                        
                        ui.add_space(12.0);
                        ui.heading("View");
                        ui.add_space(4.0);
                        
                        ui.checkbox(&mut self.settings.smooth_zoom, "Smooth zoom animation");
                        ui.checkbox(&mut self.settings.maintain_zoom_on_navigate, "Keep zoom when navigating");
                        ui.checkbox(&mut self.settings.maintain_pan_on_navigate, "Keep pan position when navigating");
                        ui.checkbox(&mut self.settings.auto_rotate_exif, "Auto-rotate based on EXIF");
                        
                        ui.add_space(12.0);
                        ui.heading("Photography Tools");
                        ui.add_space(4.0);
                        
                        ui.horizontal(|ui| {
                            ui.label("Focus peaking color:");
                            egui::ComboBox::from_id_salt("focus_color")
                                .selected_text(format!("{:?}", self.settings.focus_peaking_color))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.settings.focus_peaking_color, FocusPeakingColor::Red, "Red");
                                    ui.selectable_value(&mut self.settings.focus_peaking_color, FocusPeakingColor::Green, "Green");
                                    ui.selectable_value(&mut self.settings.focus_peaking_color, FocusPeakingColor::Blue, "Blue");
                                    ui.selectable_value(&mut self.settings.focus_peaking_color, FocusPeakingColor::Yellow, "Yellow");
                                    ui.selectable_value(&mut self.settings.focus_peaking_color, FocusPeakingColor::White, "White");
                                });
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Focus peaking threshold:");
                            ui.add(egui::Slider::new(&mut self.settings.focus_peaking_threshold, 10.0..=100.0));
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Zebra high threshold:");
                            ui.add(egui::Slider::new(&mut self.settings.zebra_high_threshold, 200..=255));
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Grid overlay:");
                            egui::ComboBox::from_id_salt("grid_type")
                                .selected_text(format!("{:?}", self.settings.grid_type))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.settings.grid_type, GridType::Off, "Off");
                                    ui.selectable_value(&mut self.settings.grid_type, GridType::RuleOfThirds, "Rule of Thirds");
                                    ui.selectable_value(&mut self.settings.grid_type, GridType::GoldenRatio, "Golden Ratio");
                                    ui.selectable_value(&mut self.settings.grid_type, GridType::Diagonal, "Diagonal");
                                    ui.selectable_value(&mut self.settings.grid_type, GridType::Center, "Center");
                                });
                        });
                        
                        ui.add_space(12.0);
                        ui.heading("Slideshow");
                        ui.add_space(4.0);
                        
                        ui.horizontal(|ui| {
                            ui.label("Interval:");
                            ui.add(egui::Slider::new(&mut self.settings.slideshow_interval, 0.5..=30.0).suffix("s"));
                        });
                        
                        ui.checkbox(&mut self.settings.slideshow_loop, "Loop slideshow");
                        ui.checkbox(&mut self.settings.slideshow_random, "Random order");
                        
                        ui.add_space(12.0);
                        ui.heading("Cache");
                        ui.add_space(4.0);
                        
                        ui.horizontal(|ui| {
                            ui.label("Cache size:");
                            ui.add(egui::Slider::new(&mut self.settings.cache_size_mb, 100..=4096).suffix(" MB"));
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Preload ahead:");
                            ui.add(egui::Slider::new(&mut self.settings.preload_adjacent, 0..=10).suffix(" images"));
                        });
                        
                        // Cache stats
                        let stats = self.image_cache.get_stats();
                        ui.label(format!("Cache: {} images ({:.1} MB)", 
                            stats.image_count, 
                            stats.image_size_bytes as f64 / 1_048_576.0));
                        
                        if ui.button("Clear Cache").clicked() {
                            self.image_cache.clear();
                        }
                        
                        ui.add_space(12.0);
                        ui.heading("Performance & Diagnostics");
                        ui.add_space(4.0);
                        
                        ui.checkbox(&mut self.profiler_enabled, "Enable performance profiling");
                        
                        if self.profiler_enabled {
                            ui.add_space(8.0);
                            
                            // Cache statistics
                            ui.label(RichText::new("Cache Statistics").strong());
                            ui.add_space(2.0);
                            
                            let cache_stats = &self.cache_stats;
                            ui.label(format!("Images cached: {} / {} ({:.1}%)", 
                                cache_stats.cached_images, 
                                cache_stats.total_images,
                                if cache_stats.total_images > 0 {
                                    (cache_stats.cached_images as f64 / cache_stats.total_images as f64) * 100.0
                                } else { 0.0 }));
                            ui.label(format!("Cache hit rate: {:.1}%", cache_stats.hit_rate() * 100.0));
                            ui.label(format!("Memory usage: {:.1} MB", cache_stats.memory_usage_mb()));
                            ui.label(format!("Evictions: {}", cache_stats.eviction_count));
                            
                            ui.add_space(8.0);
                            
                            // Loading diagnostics
                            ui.label(RichText::new("Loading Performance").strong());
                            ui.add_space(2.0);
                            
                            let diag = &self.loading_diagnostics;
                            ui.label(format!("Total load time: {:.2}s", diag.total_load_time.as_secs_f64()));
                            ui.label(format!("Average load time: {:.2}s", diag.average_load_time().as_secs_f64()));
                            ui.label(format!("Images loaded: {}", diag.images_loaded));
                            ui.label(format!("Thumbnails generated: {}", diag.thumbnails_generated));
                            ui.label(format!("Errors encountered: {}", diag.errors_encountered));
                            
                            if !diag.bottlenecks.is_empty() {
                                ui.add_space(4.0);
                                ui.label(RichText::new("Bottlenecks:").color(Color32::YELLOW));
                                for bottleneck in &diag.bottlenecks {
                                    ui.label(format!("• {}", bottleneck));
                                }
                            }
                            
                            ui.add_space(8.0);
                            
                            // Profiler stats
                            ui.label(RichText::new("Performance Timers").strong());
                            ui.add_space(2.0);
                            
                            let profiler_stats = crate::profiler::with_profiler(|p| p.get_stats());
                            for (name, stats) in &profiler_stats.measurements {
                                ui.label(format!("{}: {:.2}ms avg ({} samples)", 
                                    name, 
                                    stats.average_time.as_millis(),
                                    stats.count));
                            }
                            
                            for (name, count) in &profiler_stats.counters {
                                ui.label(format!("{}: {} times", name, count));
                            }
                            
                            if ui.button("Reset Profiler").clicked() {
                                crate::profiler::with_profiler(|p| p.reset());
                            }
                        }
                    });
                
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                
                ui.horizontal(|ui| {
                    if ui.add_sized(Vec2::new(80.0, 28.0), egui::Button::new("✓ Close")).clicked() {
                        self.show_settings_dialog = false;
                    }
                    ui.add_space(8.0);
                    if ui.add_sized(Vec2::new(120.0, 28.0), egui::Button::new("↺ Reset Defaults")).clicked() {
                        self.settings = crate::settings::Settings::default();
                    }
                });
            });
    }
    
    fn render_go_to_dialog(&mut self, ctx: &egui::Context) {
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
    
    fn render_command_palette(&mut self, ctx: &egui::Context) {
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
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(35, 35, 40))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(12.0)))
            .show(ctx, |ui| {
                // Search input
                let response = ui.add_sized(
                    Vec2::new(ui.available_width(), 32.0),
                    egui::TextEdit::singleline(&mut self.command_palette_query)
                        .hint_text("Type a command...")
                        .font(egui::TextStyle::Heading)
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
                                egui::Button::new(
                                    egui::RichText::new(&name).size(13.0)
                                ).fill(Color32::TRANSPARENT)
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
            ("Toggle Zebras", "Ctrl+Z", "zebras"),
            ("Toggle Grid Overlay", "Ctrl+G", "grid"),
            ("Toggle Loupe", "Ctrl+L", "loupe"),
            ("Toggle Sidebar", "S", "sidebar"),
            ("Toggle Thumbnails", "T", "thumbnails"),
            ("Toggle EXIF Info", "I", "exif"),
            ("Toggle Histogram", "H", "histogram"),
            ("Compare Mode", "C", "compare"),
            ("Grid View", "G", "lightbox"),
            ("Delete Image", "Del", "delete"),
            ("Set as Wallpaper", "", "wallpaper"),
            ("Settings", "", "settings"),
        ];
        
        let query = self.command_palette_query.to_lowercase();
        
        all_commands.into_iter()
            .filter(|(name, _, _)| query.is_empty() || name.to_lowercase().contains(&query))
            .map(|(n, s, a)| (n.to_string(), s.to_string(), a.to_string()))
            .collect()
    }
    
    fn execute_command(&mut self, action: &str) {
        match action {
            "open_file" => self.open_file_dialog(),
            "open_folder" => self.open_folder_dialog(),
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
            "grid" => self.settings.show_grid_overlay = !self.settings.show_grid_overlay,
            "loupe" => self.settings.loupe_enabled = !self.settings.loupe_enabled,
            "sidebar" => self.settings.show_sidebar = !self.settings.show_sidebar,
            "thumbnails" => self.settings.show_thumbnails = !self.settings.show_thumbnails,
            "exif" => self.settings.show_exif = !self.settings.show_exif,
            "histogram" => self.settings.show_histogram = !self.settings.show_histogram,
            "compare" => self.toggle_compare_mode(),
            "lightbox" => self.toggle_lightbox_mode(),
            "delete" => self.delete_current_image(),
            "wallpaper" => self.set_as_wallpaper(),
            "settings" => self.show_settings_dialog = true,
            _ => {}
        }
    }
}
