use crate::app::{ImageViewerApp, ViewMode};
use crate::settings::{SortMode, SortOrder};
use egui::{self, Color32, RichText, Vec2, Rounding, Margin};

impl ImageViewerApp {
    pub fn render_toolbar(&mut self, ctx: &egui::Context) {
        // Collect state needed for decisions
        let current_index = self.current_index;
        let filtered_len = self.filtered_list.len();
        let zoom = self.zoom;
        let view_mode = self.view_mode;
        let is_fullscreen = self.is_fullscreen;
        let slideshow_active = self.slideshow_active;
        let show_focus_peaking = self.settings.show_focus_peaking;
        let show_zebras = self.settings.show_zebras;
        let show_grid_overlay = self.settings.show_grid_overlay;
        let loupe_enabled = self.settings.loupe_enabled;
        let show_exif = self.settings.show_exif;
        let show_sidebar = self.settings.show_sidebar;
        let sort_mode = self.settings.sort_mode;
        let sort_order = self.settings.sort_order;
        
        // Collect actions to perform after UI
        let mut open_folder = false;
        let mut open_file = false;
        let mut go_first = false;
        let mut go_prev = false;
        let mut go_next = false;
        let mut go_last = false;
        let mut show_go_to = false;
        let mut zoom_out = false;
        let mut zoom_in = false;
        let mut new_zoom: Option<f32> = None;
        let mut fit_window = false;
        let mut fill_window = false;
        let mut rotate_left = false;
        let mut rotate_right = false;
        let mut set_view_single = false;
        let mut toggle_compare = false;
        let mut toggle_lightbox = false;
        let mut toggle_focus_peaking = false;
        let mut toggle_zebras = false;
        let mut toggle_grid = false;
        let mut toggle_loupe = false;
    let mut toggle_exif = false;
    let mut toggle_sidebar = false;
        let mut toggle_panels = false;
        let mut toggle_fullscreen = false;
        let mut toggle_slideshow = false;
        let mut show_settings = false;
        let mut show_command_palette = false;
        let mut new_sort_mode: Option<SortMode> = None;
        let mut toggle_sort_order = false;
        
        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::none()
                .fill(Color32::from_rgb(28, 28, 32))
                .inner_margin(Margin::symmetric(8.0, 6.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                    
                    // File operations
                    if icon_button(ui, "\u{1F4C2}", "Open folder (Ctrl+Shift+O)").clicked() {
                        open_folder = true;
                    }
                    if icon_button(ui, "\u{1F4C4}", "Open file (Ctrl+O)").clicked() {
                        open_file = true;
                    }
                    
                    ui.add_space(8.0);
                    toolbar_separator(ui);
                    ui.add_space(8.0);
                    
                    // Navigation
                    if icon_button(ui, "<<", "First image (Home)").clicked() {
                        go_first = true;
                    }
                    if icon_button(ui, "<", "Previous image (←)").clicked() {
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
                    
                    if icon_button(ui, ">", "Next image (→)").clicked() {
                        go_next = true;
                    }
                    if icon_button(ui, ">>", "Last image (End)").clicked() {
                        go_last = true;
                    }
                    
                    ui.add_space(8.0);
                    toolbar_separator(ui);
                    ui.add_space(8.0);
                    
                    // Zoom controls
                    if icon_button(ui, "−", "Zoom out (-)").clicked() {
                        zoom_out = true;
                    }
                    
                    // Zoom slider
                    let mut zoom_pct = (zoom * 100.0) as i32;
                    let slider = egui::Slider::new(&mut zoom_pct, 10..=800)
                        .show_value(false);
                    if ui.add_sized(Vec2::new(80.0, 20.0), slider).changed() {
                        new_zoom = Some(zoom_pct as f32 / 100.0);
                    }
                    
                    if icon_button(ui, "+", "Zoom in (+)").clicked() {
                        zoom_in = true;
                    }
                    
                    // Zoom presets
                    egui::ComboBox::from_id_salt("zoom_preset")
                        .selected_text(format!("{:.0}%", zoom * 100.0))
                        .width(60.0)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(false, "Fit").clicked() { fit_window = true; }
                            if ui.selectable_label(false, "Fill").clicked() { fill_window = true; }
                            ui.separator();
                            for pct in [25, 50, 100, 150, 200, 400] {
                                if ui.selectable_label(false, format!("{}%", pct)).clicked() {
                                    new_zoom = Some(pct as f32 / 100.0);
                                }
                            }
                        });
                    
                    ui.add_space(8.0);
                    toolbar_separator(ui);
                    ui.add_space(8.0);
                    
                    // Rotation
                    if icon_button(ui, "⟲", "Rotate left (L)").clicked() {
                        rotate_left = true;
                    }
                    if icon_button(ui, "⟳", "Rotate right (R)").clicked() {
                        rotate_right = true;
                    }
                    
                    ui.add_space(8.0);
                    toolbar_separator(ui);
                    ui.add_space(8.0);
                    
                    // View modes
                    if toggle_button(ui, "1", "Single view", view_mode == ViewMode::Single).clicked() {
                        set_view_single = true;
                    }
                    if toggle_button(ui, "2", "Compare view (C)", view_mode == ViewMode::Compare).clicked() {
                        toggle_compare = true;
                    }
                    if toggle_button(ui, "G", "Grid view (G)", view_mode == ViewMode::Lightbox).clicked() {
                        toggle_lightbox = true;
                    }
                    
                    ui.add_space(8.0);
                    toolbar_separator(ui);
                    ui.add_space(8.0);
                    
                    // Photography tools
                    if toggle_button(ui, "FP", "Focus peaking (Ctrl+F)", show_focus_peaking).clicked() {
                        toggle_focus_peaking = true;
                    }
                    if toggle_button(ui, "ZB", "Zebras (Ctrl+Z)", show_zebras).clicked() {
                        toggle_zebras = true;
                    }
                    if toggle_button(ui, "GR", "Grid overlay", show_grid_overlay).clicked() {
                        toggle_grid = true;
                    }
                    if toggle_button(ui, "LP", "Loupe (Ctrl+L)", loupe_enabled).clicked() {
                        toggle_loupe = true;
                    }
                    
                    ui.add_space(8.0);
                    toolbar_separator(ui);
                    ui.add_space(8.0);
                    
                    // Panel toggles
                    if toggle_button(ui, "ⓘ", "EXIF Info (I)", show_exif).clicked() {
                        toggle_exif = true;
                    }
                    if toggle_button(ui, "☰", "Sidebar (S)", show_sidebar).clicked() {
                        toggle_sidebar = true;
                    }
                    if toggle_button(ui, "⊞", "Toggle all panels (P)", self.panels_hidden).clicked() {
                        toggle_panels = true;
                    }
                    
                    // Right side
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Settings
                        if icon_button(ui, "⚙", "Settings").clicked() {
                            show_settings = true;
                        }
                        
                        if icon_button(ui, "⌘", "Command palette (Ctrl+P)").clicked() {
                            show_command_palette = true;
                        }
                        
                        // Fullscreen
                        if toggle_button(ui, "⛶", "Fullscreen (F11)", is_fullscreen).clicked() {
                            toggle_fullscreen = true;
                        }
                        
                        // Slideshow
                        let ss_icon = if slideshow_active { "⏸" } else { "▷" };
                        if toggle_button(ui, ss_icon, "Slideshow (Space)", slideshow_active).clicked() {
                            toggle_slideshow = true;
                        }
                        
                        toolbar_separator(ui);
                        
                        // Sort options
                        egui::ComboBox::from_id_salt("sort_mode")
                            .selected_text(format!("{:?}", sort_mode))
                            .width(80.0)
                            .show_ui(ui, |ui| {
                                for mode in [SortMode::Name, SortMode::Date, SortMode::Size, SortMode::Type, SortMode::Random] {
                                    if ui.selectable_label(sort_mode == mode, format!("{:?}", mode)).clicked() {
                                        new_sort_mode = Some(mode);
                                    }
                                }
                            });
                        
                        let order_icon = match sort_order {
                            SortOrder::Ascending => "↑",
                            SortOrder::Descending => "↓",
                        };
                        if icon_button(ui, order_icon, "Toggle sort order").clicked() {
                            toggle_sort_order = true;
                        }
                    });
                });
            });
        });
        
        // Apply actions after UI
        if open_folder { self.open_folder_dialog(); }
        if open_file { self.open_file_dialog(); }
        if go_first { self.go_to_first(); }
        if go_prev { self.previous_image(); }
        if go_next { self.next_image(); }
        if go_last { self.go_to_last(); }
        if show_go_to { self.show_go_to_dialog = true; }
        if zoom_out { self.zoom_out(); }
        if zoom_in { self.zoom_in(); }
        if let Some(z) = new_zoom { self.zoom_to(z); }
        if fit_window { self.fit_to_window(); }
        if fill_window { self.fill_window(); }
        if rotate_left { self.rotate_left(); }
        if rotate_right { self.rotate_right(); }
        if set_view_single { self.view_mode = ViewMode::Single; }
        if toggle_compare { self.toggle_compare_mode(); }
        if toggle_lightbox { self.toggle_lightbox_mode(); }
        if toggle_focus_peaking {
            self.settings.show_focus_peaking = !self.settings.show_focus_peaking;
            if self.settings.show_focus_peaking {
                if let Some(img) = self.current_image.clone() {
                    self.generate_focus_peaking_overlay(&img, ctx);
                }
            }
        }
        if toggle_zebras {
            self.settings.show_zebras = !self.settings.show_zebras;
            if self.settings.show_zebras {
                if let Some(img) = self.current_image.clone() {
                    self.generate_zebra_overlay(&img, ctx);
                }
            }
        }
        if toggle_grid { self.settings.show_grid_overlay = !self.settings.show_grid_overlay; }
        if toggle_loupe { self.settings.loupe_enabled = !self.settings.loupe_enabled; }
        if toggle_exif { self.settings.show_exif = !self.settings.show_exif; }
        if toggle_sidebar { self.settings.show_sidebar = !self.settings.show_sidebar; }
        if toggle_panels { self.toggle_panels(); }
        if toggle_fullscreen { self.is_fullscreen = !self.is_fullscreen; }
        if toggle_slideshow { self.toggle_slideshow(); }
        if show_settings { self.show_settings_dialog = true; }
        if show_command_palette { self.command_palette_open = true; }
        if let Some(mode) = new_sort_mode {
            self.settings.sort_mode = mode;
            self.sort_file_list();
        }
        if toggle_sort_order {
            self.settings.sort_order = match self.settings.sort_order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
            self.sort_file_list();
        }
    }
}

fn icon_button(ui: &mut egui::Ui, icon: &str, tooltip: &str) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(icon).size(16.0))
        .fill(Color32::TRANSPARENT)
        .rounding(Rounding::same(4.0))
        .min_size(Vec2::new(28.0, 28.0)))
        .on_hover_text(tooltip)
}

fn toggle_button(ui: &mut egui::Ui, icon: &str, tooltip: &str, active: bool) -> egui::Response {
    let bg = if active {
        Color32::from_rgb(70, 130, 255)
    } else {
        Color32::TRANSPARENT
    };
    
    ui.add(egui::Button::new(RichText::new(icon).size(16.0))
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
