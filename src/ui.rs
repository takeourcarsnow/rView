use crate::app::ImageViewerApp;
use crate::settings::{BackgroundColor, FitMode, SortMode, Theme};
use egui::{self, Color32, RichText, Stroke, Vec2, Rounding, Margin};

impl ImageViewerApp {
    pub fn render_top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(28, 28, 32))
                .inner_margin(Margin::symmetric(16.0, 8.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    
                    // File menu
                    ui.menu_button(RichText::new("📁 File").color(Color32::WHITE), |ui| {
                        if ui.button("📂 Open File...").clicked() {
                            self.open_file_dialog();
                            ui.close_menu();
                        }
                        if ui.button("📁 Open Folder...").clicked() {
                            self.open_folder_dialog();
                            ui.close_menu();
                        }
                        ui.separator();
                        if !self.settings.recent_folders.is_empty() {
                            ui.label(RichText::new("Recent Folders").small().color(Color32::GRAY));
                            for folder in self.settings.recent_folders.clone().iter().take(5) {
                                let name = folder.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| folder.display().to_string());
                                if ui.button(&name).clicked() {
                                    self.load_folder(folder.clone());
                                    ui.close_menu();
                                }
                            }
                            ui.separator();
                        }
                        if ui.button("❌ Exit").clicked() {
                            std::process::exit(0);
                        }
                    });
                    
                    // View menu
                    ui.menu_button(RichText::new("👁 View").color(Color32::WHITE), |ui| {
                        ui.checkbox(&mut self.settings.show_sidebar, "📋 Sidebar");
                        ui.checkbox(&mut self.settings.show_thumbnails, "🖼 Thumbnails");
                        ui.checkbox(&mut self.settings.show_exif, "📊 EXIF Info");
                        ui.checkbox(&mut self.settings.show_histogram, "📈 Histogram");
                        ui.separator();
                        
                        ui.label(RichText::new("Fit Mode").small().color(Color32::GRAY));
                        if ui.radio_value(&mut self.settings.fit_mode, FitMode::Fit, "Fit to Window").clicked() {
                            self.reset_view();
                        }
                        if ui.radio_value(&mut self.settings.fit_mode, FitMode::Fill, "Fill Window").clicked() {
                            self.reset_view();
                        }
                        if ui.radio_value(&mut self.settings.fit_mode, FitMode::OneToOne, "100% (1:1)").clicked() {
                            self.zoom = 1.0;
                        }
                        ui.radio_value(&mut self.settings.fit_mode, FitMode::FitWidth, "Fit Width");
                        ui.radio_value(&mut self.settings.fit_mode, FitMode::FitHeight, "Fit Height");
                    });
                    
                    // Settings menu
                    ui.menu_button(RichText::new("⚙ Settings").color(Color32::WHITE), |ui| {
                        ui.label(RichText::new("Theme").small().color(Color32::GRAY));
                        ui.radio_value(&mut self.settings.theme, Theme::Dark, "🌙 Dark");
                        ui.radio_value(&mut self.settings.theme, Theme::Light, "☀ Light");
                        
                        ui.separator();
                        ui.label(RichText::new("Background").small().color(Color32::GRAY));
                        ui.radio_value(&mut self.settings.background_color, BackgroundColor::Dark, "Dark");
                        ui.radio_value(&mut self.settings.background_color, BackgroundColor::Light, "Light");
                        ui.radio_value(&mut self.settings.background_color, BackgroundColor::Gray, "Gray");
                        ui.radio_value(&mut self.settings.background_color, BackgroundColor::Checkered, "Checkered");
                        
                        ui.separator();
                        ui.label(RichText::new("Navigation").small().color(Color32::GRAY));
                        ui.checkbox(&mut self.settings.maintain_zoom_on_navigate, "🔍 Keep zoom on navigate");
                        ui.checkbox(&mut self.settings.maintain_pan_on_navigate, "✋ Keep pan position on navigate");
                        
                        ui.separator();
                        ui.add(egui::Slider::new(&mut self.settings.slideshow_interval, 1.0..=30.0)
                            .text("Slideshow interval (s)"));
                    });
                    
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);
                    
                    // Navigation and zoom controls
                    if ui.button(RichText::new("⏮").size(18.0).color(Color32::WHITE)).on_hover_text("First (Home)").clicked() {
                        self.go_to_first();
                    }
                    if ui.button(RichText::new("◀").size(18.0).color(Color32::WHITE)).on_hover_text("Previous (←)").clicked() {
                        self.previous_image();
                    }
                    
                    // Image counter
                    if !self.image_list.is_empty() {
                        let text = format!("{} / {}", self.current_index + 1, self.image_list.len());
                        ui.label(RichText::new(text).color(Color32::WHITE).size(14.0));
                    }
                    
                    if ui.button(RichText::new("▶").size(18.0).color(Color32::WHITE)).on_hover_text("Next (→)").clicked() {
                        self.next_image();
                    }
                    if ui.button(RichText::new("⏭").size(18.0).color(Color32::WHITE)).on_hover_text("Last (End)").clicked() {
                        self.go_to_last();
                    }
                    
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);
                    
                    // Zoom controls
                    if ui.button(RichText::new("➖").size(16.0).color(Color32::WHITE)).on_hover_text("Zoom Out (-)").clicked() {
                        self.zoom_out();
                    }
                    
                    let zoom_text = format!("{:.0}%", self.zoom * 100.0);
                    if ui.button(RichText::new(&zoom_text).color(Color32::WHITE)).on_hover_text("Reset Zoom").clicked() {
                        self.reset_view();
                    }
                    
                    if ui.button(RichText::new("➕").size(16.0).color(Color32::WHITE)).on_hover_text("Zoom In (+)").clicked() {
                        self.zoom_in();
                    }
                    
                    if ui.button(RichText::new("🔍").size(16.0).color(Color32::WHITE)).on_hover_text("100% Zoom (1)").clicked() {
                        self.zoom = 1.0;
                    }
                    
                    if ui.button(RichText::new("⊡").size(16.0).color(Color32::WHITE)).on_hover_text("Fit to Window (F)").clicked() {
                        self.reset_view();
                    }
                    
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);
                    
                    // Rotation controls
                    if ui.button(RichText::new("↺").size(16.0).color(Color32::WHITE)).on_hover_text("Rotate Left (L)").clicked() {
                        self.rotate_left();
                    }
                    if ui.button(RichText::new("↻").size(16.0).color(Color32::WHITE)).on_hover_text("Rotate Right (R)").clicked() {
                        self.rotate_right();
                    }
                    
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);
                    
                    // Slideshow toggle
                    let slideshow_text = if self.slideshow_active { "⏸" } else { "▶" };
                    let slideshow_hint = if self.slideshow_active { "Stop Slideshow" } else { "Start Slideshow (Space)" };
                    if ui.button(RichText::new(slideshow_text).size(16.0).color(Color32::WHITE)).on_hover_text(slideshow_hint).clicked() {
                        self.toggle_slideshow();
                    }
                    
                    // Spacer
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Current filename
                        if let Some(path) = self.image_list.get(self.current_index) {
                            let filename = path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            ui.label(RichText::new(filename).color(Color32::GRAY).size(12.0));
                        }
                    });
                });
            });
    }
    
    pub fn render_sidebar(&mut self, ctx: &egui::Context) {
        if !self.settings.show_sidebar {
            return;
        }
        
        egui::SidePanel::right("sidebar")
            .default_width(280.0)
            .min_width(200.0)
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(24, 24, 28))
                .inner_margin(Margin::same(12.0)))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // File info section
                    if let Some(path) = self.image_list.get(self.current_index) {
                        // Filename
                        let filename = path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        ui.heading(RichText::new(&filename).color(Color32::WHITE).size(16.0));
                        ui.add_space(4.0);
                        
                        // Path
                        if let Some(parent) = path.parent() {
                            ui.label(RichText::new(parent.display().to_string())
                                .color(Color32::GRAY)
                                .size(11.0));
                        }
                        
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(12.0);
                        
                        // Image dimensions
                        if let Some(ref texture) = self.current_texture {
                            let size = texture.size_vec2();
                            self.render_info_row(ui, "Dimensions", &format!("{} × {}", size.x as u32, size.y as u32));
                        }
                        
                        // Zoom level
                        self.render_info_row(ui, "Zoom", &format!("{:.1}%", self.zoom * 100.0));
                        
                        // Rotation
                        if self.rotation != 0.0 {
                            self.render_info_row(ui, "Rotation", &format!("{}°", self.rotation as i32));
                        }
                        
                        ui.add_space(16.0);
                    }
                    
                    // EXIF section
                    if self.settings.show_exif {
                        self.render_exif_panel(ui);
                    }
                    
                    // Histogram section
                    if self.settings.show_histogram {
                        ui.add_space(16.0);
                        self.render_histogram(ui);
                    }
                    
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(12.0);
                    
                    // Sorting options
                    ui.label(RichText::new("Sort By").color(Color32::WHITE).size(13.0));
                    ui.add_space(4.0);
                    
                    ui.horizontal(|ui| {
                        if ui.selectable_label(self.settings.sort_mode == SortMode::Name, "Name").clicked() {
                            self.settings.sort_mode = SortMode::Name;
                            self.sort_images();
                        }
                        if ui.selectable_label(self.settings.sort_mode == SortMode::Date, "Date").clicked() {
                            self.settings.sort_mode = SortMode::Date;
                            self.sort_images();
                        }
                        if ui.selectable_label(self.settings.sort_mode == SortMode::Size, "Size").clicked() {
                            self.settings.sort_mode = SortMode::Size;
                            self.sort_images();
                        }
                    });
                    
                    ui.horizontal(|ui| {
                        if ui.selectable_label(self.settings.sort_ascending, "↑ Ascending").clicked() {
                            self.settings.sort_ascending = true;
                            self.sort_images();
                        }
                        if ui.selectable_label(!self.settings.sort_ascending, "↓ Descending").clicked() {
                            self.settings.sort_ascending = false;
                            self.sort_images();
                        }
                    });
                    
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(12.0);
                    
                    // Quick actions
                    ui.label(RichText::new("Quick Actions").color(Color32::WHITE).size(13.0));
                    ui.add_space(8.0);
                    
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("📋 Copy Path").clicked() {
                            if let Some(path) = self.image_list.get(self.current_index) {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    let _ = clipboard.set_text(path.display().to_string());
                                }
                            }
                        }
                        
                        if ui.button("📁 Show in Explorer").clicked() {
                            self.open_in_file_manager();
                        }
                        
                        if ui.button("🗑 Delete").clicked() {
                            self.delete_current_image();
                        }
                    });
                    
                    // Keyboard shortcuts help
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(12.0);
                    
                    egui::CollapsingHeader::new(RichText::new("⌨ Keyboard Shortcuts").color(Color32::GRAY).size(12.0))
                        .default_open(false)
                        .show(ui, |ui| {
                            self.render_shortcut(ui, "← →", "Previous/Next");
                            self.render_shortcut(ui, "Home/End", "First/Last");
                            self.render_shortcut(ui, "+/-", "Zoom In/Out");
                            self.render_shortcut(ui, "0", "Fit to Window");
                            self.render_shortcut(ui, "1", "100% Zoom");
                            self.render_shortcut(ui, "L/R", "Rotate Left/Right");
                            self.render_shortcut(ui, "Space", "Toggle Slideshow");
                            self.render_shortcut(ui, "F11", "Toggle Fullscreen");
                            self.render_shortcut(ui, "Esc", "Exit Fullscreen");
                            self.render_shortcut(ui, "Del", "Delete Image");
                        });
                });
            });
    }
    
    fn render_info_row(&self, ui: &mut egui::Ui, label: &str, value: &str) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).color(Color32::GRAY).size(12.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(value).color(Color32::WHITE).size(12.0));
            });
        });
    }
    
    fn render_shortcut(&self, ui: &mut egui::Ui, key: &str, action: &str) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(key).color(Color32::from_rgb(100, 180, 255)).size(11.0).monospace());
            ui.label(RichText::new(action).color(Color32::GRAY).size(11.0));
        });
    }
    
    fn render_exif_panel(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.add_space(12.0);
        ui.label(RichText::new("📷 Camera Info").color(Color32::WHITE).size(13.0));
        ui.add_space(8.0);
        
        if let Some(ref exif) = self.current_exif {
            if let Some(ref make) = exif.camera_make {
                self.render_info_row(ui, "Make", make);
            }
            if let Some(ref model) = exif.camera_model {
                self.render_info_row(ui, "Model", model);
            }
            if let Some(ref lens) = exif.lens {
                self.render_info_row(ui, "Lens", lens);
            }
            if let Some(ref focal) = exif.focal_length {
                self.render_info_row(ui, "Focal Length", focal);
            }
            if let Some(ref aperture) = exif.aperture {
                self.render_info_row(ui, "Aperture", aperture);
            }
            if let Some(ref shutter) = exif.shutter_speed {
                self.render_info_row(ui, "Shutter", shutter);
            }
            if let Some(ref iso) = exif.iso {
                self.render_info_row(ui, "ISO", iso);
            }
            if let Some(ref date) = exif.date_taken {
                self.render_info_row(ui, "Date", date);
            }
            if let Some(ref size) = exif.file_size {
                self.render_info_row(ui, "File Size", size);
            }
            
            if !exif.has_data() {
                ui.label(RichText::new("No EXIF data available").color(Color32::GRAY).size(11.0).italics());
            }
        } else {
            ui.label(RichText::new("Loading EXIF data...").color(Color32::GRAY).size(11.0).italics());
        }
    }
    
    fn render_histogram(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.add_space(12.0);
        ui.label(RichText::new("📊 Histogram").color(Color32::WHITE).size(13.0));
        ui.add_space(8.0);
        
        // Draw histogram
        let available_width = ui.available_width();
        let hist_height = 80.0;
        
        let (response, painter) = ui.allocate_painter(Vec2::new(available_width, hist_height), egui::Sense::hover());
        let rect = response.rect;
        
        // Background
        painter.rect_filled(rect, Rounding::same(4.0), Color32::from_rgb(15, 15, 18));
        
        if let Some(ref histogram) = self.histogram_data {
            let max_val = histogram.iter().map(|c| c.iter().max().unwrap_or(&0)).max().unwrap_or(&1);
            let max_val = (*max_val).max(1) as f32;
            
            let bar_width = rect.width() / 256.0;
            
            // Draw RGB channels
            for x in 0..256 {
                let x_pos = rect.left() + x as f32 * bar_width;
                
                // Red channel
                let r_height = (histogram[0][x] as f32 / max_val) * rect.height() * 0.8;
                painter.line_segment(
                    [egui::pos2(x_pos, rect.bottom()), egui::pos2(x_pos, rect.bottom() - r_height)],
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 80, 80, 100)),
                );
                
                // Green channel
                let g_height = (histogram[1][x] as f32 / max_val) * rect.height() * 0.8;
                painter.line_segment(
                    [egui::pos2(x_pos, rect.bottom()), egui::pos2(x_pos, rect.bottom() - g_height)],
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(80, 255, 80, 100)),
                );
                
                // Blue channel
                let b_height = (histogram[2][x] as f32 / max_val) * rect.height() * 0.8;
                painter.line_segment(
                    [egui::pos2(x_pos, rect.bottom()), egui::pos2(x_pos, rect.bottom() - b_height)],
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(80, 80, 255, 100)),
                );
            }
        } else {
            // No histogram data
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No data",
                egui::FontId::default(),
                Color32::GRAY,
            );
        }
    }
    
    pub fn render_thumbnail_bar(&mut self, ctx: &egui::Context) {
        if !self.settings.show_thumbnails || self.image_list.is_empty() {
            return;
        }
        
        // Collect thumbnail data first to avoid borrow issues
        let thumb_data: Vec<_> = self.image_list.iter().enumerate().map(|(idx, path)| {
            let is_selected = idx == self.current_index;
            let tex_id = self.thumbnail_textures.get(path).copied();
            let path_clone = path.clone();
            (idx, path_clone, is_selected, tex_id)
        }).collect();
        
        let thumb_size_val = self.settings.thumbnail_size;
        let mut thumbnail_requests: Vec<std::path::PathBuf> = Vec::new();
        let mut clicked_index: Option<usize> = None;
        
        egui::TopBottomPanel::bottom("thumbnails")
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(20, 20, 24))
                .inner_margin(Margin::symmetric(8.0, 8.0)))
            .exact_height(thumb_size_val + 24.0)
            .show(ctx, |ui| {
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            
                            for (idx, path, is_selected, tex_id) in &thumb_data {
                                let thumb_size = Vec2::splat(thumb_size_val);
                                
                                let (response, painter) = ui.allocate_painter(
                                    thumb_size + Vec2::new(4.0, 4.0),
                                    egui::Sense::click()
                                );
                                
                                let rect = response.rect;
                                
                                // Selection highlight
                                if *is_selected {
                                    painter.rect_filled(
                                        rect.expand(2.0),
                                        Rounding::same(6.0),
                                        Color32::from_rgb(70, 130, 255),
                                    );
                                } else if response.hovered() {
                                    painter.rect_filled(
                                        rect,
                                        Rounding::same(4.0),
                                        Color32::from_rgb(50, 50, 55),
                                    );
                                }
                                
                                // Thumbnail background
                                painter.rect_filled(
                                    rect.shrink(2.0),
                                    Rounding::same(4.0),
                                    Color32::from_rgb(35, 35, 40),
                                );
                                
                                // Try to get cached thumbnail texture
                                if let Some(tex_id) = tex_id {
                                    let inner_rect = rect.shrink(4.0);
                                    painter.image(
                                        *tex_id,
                                        inner_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        Color32::WHITE,
                                    );
                                } else {
                                    // Request thumbnail loading
                                    thumbnail_requests.push(path.clone());
                                    
                                    // Show placeholder
                                    painter.text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "...",
                                        egui::FontId::default(),
                                        Color32::GRAY,
                                    );
                                }
                                
                                if response.clicked() {
                                    clicked_index = Some(*idx);
                                }
                            }
                        });
                    });
            });
        
        // Process thumbnail requests after the borrow ends
        for path in thumbnail_requests {
            self.request_thumbnail(path, ctx.clone());
        }
        
        // Handle click after the borrow ends
        if let Some(idx) = clicked_index {
            self.go_to_index(idx);
        }
    }
    
    pub fn render_main_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(self.settings.background_color.to_color()))
            .show(ctx, |ui| {
                // Handle drag and drop
                self.handle_dropped_files(ctx);
                
                if self.image_list.is_empty() {
                    // Welcome screen
                    self.render_welcome_screen(ui);
                } else if let Some(texture) = &self.current_texture {
                    // Image display
                    self.render_image(ui, texture.clone());
                } else if self.is_loading {
                    // Loading indicator
                    ui.centered_and_justified(|ui| {
                        ui.spinner();
                    });
                } else {
                    // Error state
                    ui.centered_and_justified(|ui| {
                        ui.label(RichText::new("Failed to load image").color(Color32::RED).size(18.0));
                    });
                }
            });
    }
    
    fn render_welcome_screen(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 3.0);
            
            ui.label(RichText::new("🖼").size(64.0));
            ui.add_space(16.0);
            ui.label(RichText::new("Image Viewer").color(Color32::WHITE).size(28.0));
            ui.add_space(8.0);
            ui.label(RichText::new("Drop images here or use File → Open").color(Color32::GRAY).size(14.0));
            
            ui.add_space(32.0);
            
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() / 2.0 - 150.0);
                
                ui.vertical(|ui| {
                    ui.label(RichText::new("Supported formats:").color(Color32::GRAY).size(12.0));
                    ui.add_space(4.0);
                    ui.label(RichText::new("JPG, PNG, GIF, BMP, TIFF, WebP").color(Color32::from_rgb(100, 180, 255)).size(11.0));
                    ui.label(RichText::new("CR2, CR3, NEF, ARW, ORF, DNG, RAF...").color(Color32::from_rgb(100, 180, 255)).size(11.0));
                });
            });
        });
    }
    
    fn render_image(&mut self, ui: &mut egui::Ui, texture: egui::TextureHandle) {
        let available = ui.available_size();
        let image_size = texture.size_vec2();
        
        // Calculate display size based on zoom
        let display_size = image_size * self.zoom;
        
        // Create scrollable area for panning
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                let (response, painter) = ui.allocate_painter(
                    display_size.max(available),
                    egui::Sense::click_and_drag()
                );
                
                // Calculate image position (centered when smaller than view)
                let offset = if display_size.x < available.x || display_size.y < available.y {
                    Vec2::new(
                        ((available.x - display_size.x) / 2.0).max(0.0),
                        ((available.y - display_size.y) / 2.0).max(0.0),
                    )
                } else {
                    Vec2::ZERO
                };
                
                let image_rect = egui::Rect::from_min_size(
                    response.rect.min + offset + self.pan_offset,
                    display_size
                );
                
                // Draw checkered background for transparency
                if self.settings.background_color == BackgroundColor::Checkered {
                    self.draw_checkered_background(&painter, image_rect);
                }
                
                // Apply rotation
                if self.rotation != 0.0 {
                    // For rotated images, we use a mesh with transformed UVs
                    let center = image_rect.center();
                    let angle = self.rotation.to_radians();
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();
                    
                    let rotate_point = |p: egui::Pos2| -> egui::Pos2 {
                        let dx = p.x - center.x;
                        let dy = p.y - center.y;
                        egui::pos2(
                            center.x + dx * cos_a - dy * sin_a,
                            center.y + dx * sin_a + dy * cos_a,
                        )
                    };
                    
                    let mut mesh = egui::Mesh::with_texture(texture.id());
                    
                    let corners = [
                        image_rect.left_top(),
                        image_rect.right_top(),
                        image_rect.right_bottom(),
                        image_rect.left_bottom(),
                    ];
                    
                    let uvs = [
                        egui::pos2(0.0, 0.0),
                        egui::pos2(1.0, 0.0),
                        egui::pos2(1.0, 1.0),
                        egui::pos2(0.0, 1.0),
                    ];
                    
                    for i in 0..4 {
                        mesh.vertices.push(egui::epaint::Vertex {
                            pos: rotate_point(corners[i]),
                            uv: uvs[i],
                            color: Color32::WHITE,
                        });
                    }
                    
                    mesh.indices = vec![0, 1, 2, 0, 2, 3];
                    painter.add(egui::Shape::mesh(mesh));
                } else {
                    painter.image(
                        texture.id(),
                        image_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                
                // Handle panning with drag
                if response.dragged() {
                    self.pan_offset += response.drag_delta();
                }
                
                // Handle zoom with scroll wheel
                let scroll = ui.input(|i| i.raw_scroll_delta.y);
                if scroll != 0.0 {
                    let zoom_factor = if scroll > 0.0 { 1.1 } else { 0.9 };
                    let old_zoom = self.zoom;
                    self.zoom = (self.zoom * zoom_factor).clamp(0.1, 20.0);
                    
                    // Zoom towards mouse position
                    if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                        let zoom_change = self.zoom / old_zoom;
                        let mouse_rel = mouse_pos - response.rect.min - self.pan_offset;
                        self.pan_offset -= mouse_rel * (zoom_change - 1.0);
                    }
                }
                
                // Double-click to toggle fit/100%
                if response.double_clicked() {
                    if (self.zoom - 1.0).abs() < 0.01 {
                        self.reset_view();
                    } else {
                        self.zoom = 1.0;
                        self.pan_offset = Vec2::ZERO;
                    }
                }
            });
    }
    
    fn draw_checkered_background(&self, painter: &egui::Painter, rect: egui::Rect) {
        let check_size = 10.0;
        let light = Color32::from_rgb(60, 60, 65);
        let dark = Color32::from_rgb(40, 40, 45);
        
        let start_x = (rect.left() / check_size).floor() as i32;
        let end_x = (rect.right() / check_size).ceil() as i32;
        let start_y = (rect.top() / check_size).floor() as i32;
        let end_y = (rect.bottom() / check_size).ceil() as i32;
        
        for y in start_y..end_y {
            for x in start_x..end_x {
                let color = if (x + y) % 2 == 0 { light } else { dark };
                let check_rect = egui::Rect::from_min_size(
                    egui::pos2(x as f32 * check_size, y as f32 * check_size),
                    Vec2::splat(check_size),
                ).intersect(rect);
                
                if check_rect.is_positive() {
                    painter.rect_filled(check_rect, Rounding::ZERO, color);
                }
            }
        }
    }
}
