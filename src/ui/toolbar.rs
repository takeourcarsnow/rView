use crate::app::{ImageViewerApp, ViewMode};

use egui::{self, Color32, RichText, Vec2, Rounding, Margin, FontFamily, FontId};
use iconflow::{try_icon, Pack, Size, Style};

/// Helper to get a Lucide icon character
fn lucide(name: &str) -> char {
    try_icon(Pack::Lucide, name, Style::Regular, Size::Regular)
        .map(|icon| char::from_u32(icon.codepoint).unwrap_or('?'))
        .unwrap_or('?')
}

/// Helper to get a Lucide icon font family name
fn lucide_font() -> &'static str {
    try_icon(Pack::Lucide, "folder", Style::Regular, Size::Regular)
        .map(|icon| icon.family)
        .unwrap_or("Lucide")
}

impl ImageViewerApp {
    pub fn render_toolbar(&mut self, ctx: &egui::Context) {
        // Collect state needed for decisions
        let current_index = self.current_index;
        let filtered_len = self.filtered_list.len();
        let zoom = self.zoom;
        let view_mode = self.view_mode;
        let _selected_count = self.selected_indices.len();
        let is_fullscreen = self.is_fullscreen;
        let show_focus_peaking = self.settings.show_focus_peaking;
        let show_zebras = self.settings.show_zebras;
        let show_grid_overlay = self.settings.show_grid_overlay;
        let loupe_enabled = self.settings.loupe_enabled;
        let load_raw_full_size = self.settings.load_raw_full_size;

        
        // Collect actions to perform after UI
        let mut open_folder = false;
        let mut open_file = false;
        let mut show_move = false;
        let mut export_image = false;
        let mut go_prev = false;
        let mut go_next = false;
        let mut show_go_to = false;
        let mut zoom_out = false;
        let mut zoom_in = false;
        let mut new_zoom: Option<f32> = None;
        let mut fit_window = false;
        let mut fill_window = false;
        let mut rotate_left = false;
        let mut rotate_right = false;
        let mut set_view_single = false;
        let mut toggle_lightbox = false;
        let mut toggle_focus_peaking = false;
        let mut toggle_zebras = false;
        let mut toggle_grid = false;
        let mut toggle_loupe = false;
    let mut toggle_panels = false;
        let mut toggle_fullscreen = false;
        let mut show_settings = false;
        let mut show_command_palette = false;

        let mut toggle_load_raw = false;
        let mut toggle_before_after = false;
        let mut search_changed = false;
        let mut toggle_search = false;
        
        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(28, 28, 32))
                .inner_margin(Margin::symmetric(8.0, 6.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                    
                    // File operations - using Lucide icons
                    if icon_button(ui, lucide("folder-open"), "Open folder (Ctrl+Shift+O)").clicked() {
                        open_folder = true;
                    }
                    if icon_button(ui, lucide("file"), "Open file (Ctrl+O)").clicked() {
                        open_file = true;
                    }
                    if icon_button(ui, lucide("folder-input"), "Move to folder (M)").clicked() {
                        show_move = true;
                    }
                    if icon_button(ui, lucide("download"), "Export image (Ctrl+S)").clicked() {
                        export_image = true;
                    }
                    
                    ui.add_space(8.0);
                    toolbar_separator(ui);
                    ui.add_space(8.0);
                    
                    // Search button
                    if toggle_button(ui, lucide("search"), "Toggle search (Ctrl+F)", self.search_visible).clicked() {
                        toggle_search = true;
                    }
                    
                    // Search bar (only shown when toggled on)
                    if self.search_visible {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("🔍").size(14.0));
                            let search_response = ui.add(
                                egui::TextEdit::singleline(&mut self.search_query)
                                    .hint_text("Search images...")
                                    .desired_width(150.0)
                            );
                            if search_response.changed() {
                                search_changed = true;
                            }
                            if !self.search_query.is_empty() && ui.add(egui::Button::new("✕").small()).on_hover_text("Clear search").clicked() {
                                self.search_query.clear();
                                search_changed = true;
                            }
                        });
                        
                        ui.add_space(8.0);
                        toolbar_separator(ui);
                        ui.add_space(8.0);
                    }
                    
                    // Navigation (previous / next)
                    if icon_button(ui, lucide("chevron-left"), "Previous image (←)").clicked() {
                        go_prev = true;
                    }
                    
                    // Image counter
                    if filtered_len > 0 {
                        let counter = format!("{}/{}", current_index + 1, filtered_len);
                        if ui.add(egui::Button::new(
                            RichText::new(&counter).size(12.0).color(Color32::from_rgb(180, 180, 180))
                        ).fill(Color32::from_rgb(40, 40, 45))
                         .rounding(Rounding::same(4.0))
                         .min_size(Vec2::new(60.0, 24.0)))
                         .on_hover_text("Go to image (Ctrl+G)")
                         .clicked() {
                            show_go_to = true;
                        }
                    }
                    
                    if icon_button(ui, lucide("chevron-right"), "Next image (→)").clicked() {
                        go_next = true;
                    }
                    
                    ui.add_space(8.0);
                    toolbar_separator(ui);
                    ui.add_space(8.0);
                    
                    // Zoom controls
                    if icon_button(ui, lucide("zoom-out"), "Zoom out (-)").clicked() {
                        zoom_out = true;
                    }
                    
                    // Zoom slider
                    let mut zoom_pct = (zoom * 100.0) as i32;
                    let slider = egui::Slider::new(&mut zoom_pct, 10..=800)
                        .show_value(false);
                    if ui.add_sized(Vec2::new(80.0, 20.0), slider).changed() {
                        new_zoom = Some(zoom_pct as f32 / 100.0);
                    }
                    
                    if icon_button(ui, lucide("zoom-in"), "Zoom in (+)").clicked() {
                        zoom_in = true;
                    }
                    
                    // Zoom presets as buttons (Fit / Fill / 100%)
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("Fit").min_size(Vec2::new(48.0, 22.0))).clicked() { fit_window = true; }
                        if ui.add(egui::Button::new("Fill").min_size(Vec2::new(48.0, 22.0))).clicked() { fill_window = true; }
                        if ui.add(egui::Button::new("100%").min_size(Vec2::new(48.0, 22.0))).clicked() { new_zoom = Some(1.0); }
                    });
                    
                    ui.add_space(8.0);
                    toolbar_separator(ui);
                    ui.add_space(8.0);
                    
                    // Rotation
                    if icon_button(ui, lucide("rotate-ccw"), "Rotate left (L)").clicked() {
                        rotate_left = true;
                    }
                    if icon_button(ui, lucide("rotate-cw"), "Rotate right (R)").clicked() {
                        rotate_right = true;
                    }
                    
                    ui.add_space(8.0);
                    toolbar_separator(ui);
                    ui.add_space(8.0);
                    
                    // View modes
                    if toggle_button(ui, lucide("image"), "Single view", view_mode == ViewMode::Single).clicked() {
                        set_view_single = true;
                    }

                    if toggle_button(ui, lucide("layout-grid"), "Grid view (G)", view_mode == ViewMode::Lightbox).clicked() {
                        toggle_lightbox = true;
                    }
                    
                    ui.add_space(8.0);
                    toolbar_separator(ui);
                    ui.add_space(8.0);
                    
                    // Photography tools
                    if toggle_button(ui, lucide("focus"), "Focus peaking (Ctrl+F)", show_focus_peaking).clicked() {
                        toggle_focus_peaking = true;
                    }
                    if toggle_button(ui, lucide("zap"), "Zebras (Ctrl+Z)", show_zebras).clicked() {
                        toggle_zebras = true;
                    }
                    if toggle_button(ui, lucide("grid-3x3"), "Grid overlay", show_grid_overlay).clicked() {
                        toggle_grid = true;
                    }
                    if toggle_button(ui, lucide("search"), "Loupe (Ctrl+L)", loupe_enabled).clicked() {
                        toggle_loupe = true;
                    }

                    // Toggle whether to decode full-size RAW files or only use embedded JPEG previews
                    if toggle_button(ui, "RAW", "Load full-size RAW files (toggle). When off, only embedded JPEG previews are used.", load_raw_full_size).clicked() {
                        toggle_load_raw = true;
                    }

                    // EXIF overlay toggle (only controls overlay, not sidebar panel)
                    if toggle_button(ui, lucide("info"), "Toggle EXIF overlay (E)", self.settings.show_exif_overlay).clicked() {
                        self.settings.show_exif_overlay = !self.settings.show_exif_overlay;
                    }

                    // Before/After toggle (only enabled when adjustments are applied)
                    if !self.adjustments.is_default() && toggle_button(ui, lucide("arrow-left-right"), "Toggle before/after view (\\)", self.show_original).clicked() {
                        toggle_before_after = true;
                    }
                    
                    ui.add_space(8.0);
                    toolbar_separator(ui);
                    ui.add_space(8.0);
                    
                    // Panel toggles
                    if toggle_button(ui, lucide("panel-left"), "Toggle all panels (P)", self.panels_hidden).clicked() {
                        toggle_panels = true;
                    }
                    
                    // Right side
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Help button
                        if icon_button(ui, lucide("help-circle"), "Keyboard shortcuts:\n←/→: Navigate\nSpace: Next\nBackspace: Previous\n1/0: 100%/Fit\nH: Toggle panels\nG: Grid view\nCtrl+F: Search\nF11: Fullscreen\nEsc: Close dialogs").clicked() {
                            // Just show tooltip, no action needed
                        }
                        
                        // Settings (toggle)
                        if icon_button(ui, lucide("settings"), "Settings").clicked() {
                            show_settings = true;
                        }
                        
                        if icon_button(ui, lucide("command"), "Command palette (Ctrl+P)").clicked() {
                            show_command_palette = true;
                        }
                        
                        // Fullscreen
                        if toggle_button(ui, lucide("maximize"), "Fullscreen (F11)", is_fullscreen).clicked() {
                            toggle_fullscreen = true;
                        }
                        
                        // (Slideshow removed per user preference)
                        toolbar_separator(ui);
                        

                    });
                });
            });
        });
        
        // Apply actions after UI
        if open_folder { self.open_folder_dialog(); }
        if open_file { self.open_file_dialog(); }
        if show_move { self.show_move_dialog = true; }
        if export_image { self.export_image(); }
        if go_prev { self.previous_image(); }
        if go_next { self.next_image(); }
        if show_go_to { self.show_go_to_dialog = true; }
        if show_settings { self.show_settings_dialog = !self.show_settings_dialog; }
        if zoom_out { self.zoom_out(); }
        if zoom_in { self.zoom_in(); }
        if let Some(z) = new_zoom { self.zoom_to(z); }
        if fit_window { self.fit_to_window(); }
        if fill_window { self.fill_window(); }
        if rotate_left { self.rotate_left(); }
        if rotate_right { self.rotate_right(); }
        if set_view_single { self.view_mode = ViewMode::Single; }

        if toggle_lightbox { self.toggle_lightbox_mode(); }
        if toggle_focus_peaking {
            self.settings.show_focus_peaking = !self.settings.show_focus_peaking;
            if self.settings.show_focus_peaking {
                if let Some(img) = self.current_image.as_ref().cloned() {
                    self.generate_focus_peaking_overlay(&img, ctx);
                }
            }
        }
        if toggle_zebras {
            self.settings.show_zebras = !self.settings.show_zebras;
            if self.settings.show_zebras {
                if let Some(img) = self.current_image.as_ref().cloned() {
                    self.generate_zebra_overlay(&img, ctx);
                }
            }
        }
        if toggle_grid { self.settings.show_grid_overlay = !self.settings.show_grid_overlay; }
        if toggle_loupe { self.settings.loupe_enabled = !self.settings.loupe_enabled; }
        if toggle_panels { self.toggle_panels(); }
        if toggle_fullscreen { 
            self.is_fullscreen = !self.is_fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
        }
        if toggle_before_after {
            self.show_original = !self.show_original;
            self.refresh_adjustments();
        }

        // Handle RAW load toggle change: flip the setting and, if enabling full RAW and current item is a RAW with only a preview, start full load
        if toggle_load_raw {
            self.settings.load_raw_full_size = !self.settings.load_raw_full_size;
            if self.settings.load_raw_full_size {
                if let Some(path) = self.get_current_path() {
                    if crate::image_loader::is_raw_file(&path) && self.showing_preview {
                        // Start loading the full image now
                        self.is_loading = true;
                        let path_clone = path.clone();
                        self.spawn_loader(move || {
                            Some(match crate::image_loader::load_image(&path_clone) {
                                Ok(image) => super::LoaderMessage::ImageLoaded(path_clone, image),
                                Err(e) => super::LoaderMessage::LoadError(path_clone, e.to_string()),
                            })
                        });
                    }
                }
            }
        }

        if show_command_palette { self.command_palette_open = true; }

        if toggle_search {
            self.search_visible = !self.search_visible;
        }
        if search_changed {
            self.apply_filter();
        }
    }
}

fn icon_button<T: ToString>(ui: &mut egui::Ui, icon: T, tooltip: &str) -> egui::Response {
    let font_id = FontId::new(16.0, FontFamily::Name(lucide_font().into()));
    ui.add(egui::Button::new(RichText::new(icon.to_string()).font(font_id))
        .fill(Color32::TRANSPARENT)
        .rounding(Rounding::same(4.0))
        .min_size(Vec2::new(28.0, 28.0)))
        .on_hover_text(tooltip)
}

fn toggle_button<T: ToString>(ui: &mut egui::Ui, icon: T, tooltip: &str, active: bool) -> egui::Response {
    let bg = if active {
        Color32::from_rgb(70, 130, 255)
    } else {
        Color32::TRANSPARENT
    };
    
    let font_id = FontId::new(16.0, FontFamily::Name(lucide_font().into()));
    ui.add(egui::Button::new(RichText::new(icon.to_string()).font(font_id))
        .fill(bg)
        .rounding(Rounding::same(4.0))
        .min_size(Vec2::new(28.0, 28.0)))
        .on_hover_text(tooltip)
}



fn toolbar_separator(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 20.0), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.left_top(), rect.left_bottom()],
        egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 65)),
    );
}
