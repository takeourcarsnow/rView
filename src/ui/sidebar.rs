use crate::app::ImageViewerApp;
use crate::image_loader::{ImageAdjustments, FilmPreset};
use crate::metadata::FileOperation;
use crate::settings::ColorLabel;
use egui::{self, Color32, RichText, Vec2, Rounding, Margin, Stroke, Rect};
use std::path::PathBuf;

// Lightroom-inspired color scheme
const LR_BG_DARK: Color32 = Color32::from_rgb(38, 38, 38);
const LR_BG_PANEL: Color32 = Color32::from_rgb(51, 51, 51);
const LR_BG_INPUT: Color32 = Color32::from_rgb(34, 34, 34);
const LR_BORDER: Color32 = Color32::from_rgb(28, 28, 28);
const LR_TEXT_PRIMARY: Color32 = Color32::from_rgb(200, 200, 200);
const LR_TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 140);
const LR_TEXT_LABEL: Color32 = Color32::from_rgb(180, 180, 180);
const LR_HEADER_BG: Color32 = Color32::from_rgb(45, 45, 45);

impl ImageViewerApp {
    /// Render the navigator panel on the left side of the screen
    pub fn render_navigator_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigator_panel")
            .resizable(true)
            .default_width(200.0)
            .min_width(150.0)
            .max_width(300.0)
            .frame(egui::Frame::none()
                .fill(LR_BG_DARK)
                .stroke(Stroke::new(1.0, LR_BORDER))
                .inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                self.render_navigator_panel(ui);
            });
    }
    
    pub fn render_sidebar(&mut self, ctx: &egui::Context) {
        if !self.settings.show_sidebar {
            return;
        }
        
        egui::SidePanel::right("sidebar")
            .resizable(true)
            .default_width(280.0)
            .min_width(220.0)
            .max_width(400.0)
            .frame(egui::Frame::none()
                .fill(LR_BG_DARK)
                .stroke(Stroke::new(1.0, LR_BORDER))
                .inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Histogram
                        if self.settings.show_histogram {
                            self.render_histogram_panel(ui);
                        }
                        
                        // Quick Develop / Basic adjustments
                        if self.settings.show_adjustments {
                            self.render_basic_panel(ui);
                        }
                        
                        // EXIF / Metadata
                        if self.settings.show_exif {
                            self.render_metadata_info_panel(ui);
                        }
                        
                        // Keywording / Rating & Labels
                        self.render_keywording_panel(ui);
                        
                        // File Browser (Folders panel like Lightroom)
                        self.render_folders_panel(ui);
                        
                        ui.add_space(20.0);
                    });
            });
    }

    fn render_navigator_panel(&mut self, ui: &mut egui::Ui) {
        // Panel header background
        let header_rect = ui.available_rect_before_wrap();
        let header_rect = Rect::from_min_size(
            header_rect.min,
            Vec2::new(ui.available_width(), 24.0)
        );

        ui.painter().rect_filled(header_rect, Rounding::ZERO, LR_HEADER_BG);
        ui.painter().hline(
            header_rect.x_range(),
            header_rect.bottom(),
            Stroke::new(1.0, LR_BORDER)
        );

        // Header text
        ui.painter().text(
            header_rect.left_center() + Vec2::new(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "Navigator",
            egui::FontId::proportional(11.0),
            LR_TEXT_PRIMARY,
        );

        // Contents
        ui.add_space(4.0);
        egui::Frame::none()
            .fill(LR_BG_PANEL)
            .inner_margin(Margin::symmetric(8.0, 6.0))
            .show(ui, |ui| {
                let available_width = ui.available_width();
                let nav_height = 140.0;
                
                // Zoom level buttons (like Lightroom: FIT, FILL, 1:1, custom)
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    
                    let zoom_btn_style = |is_active: bool| {
                        if is_active { LR_TEXT_PRIMARY } else { LR_TEXT_SECONDARY }
                    };
                    
                    let fit_active = (self.zoom - 1.0).abs() < 0.01 && self.pan_offset == Vec2::ZERO;
                    if ui.add(egui::Button::new(RichText::new("FIT").size(10.0).color(zoom_btn_style(fit_active)))
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(28.0, 16.0)))
                        .clicked() {
                        self.fit_to_window();
                    }
                    
                    if ui.add(egui::Button::new(RichText::new("100%").size(10.0).color(zoom_btn_style((self.zoom - 1.0).abs() < 0.01)))
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(32.0, 16.0)))
                        .clicked() {
                        self.zoom = 1.0;
                        self.target_zoom = 1.0;
                    }
                    
                    if ui.add(egui::Button::new(RichText::new("25%").size(10.0).color(LR_TEXT_SECONDARY))
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(28.0, 16.0)))
                        .clicked() {
                        self.zoom = 0.25;
                        self.target_zoom = 0.25;
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(4.0);
                        // Show current zoom percentage
                        ui.label(RichText::new(format!("{:.0}%", self.zoom * 100.0))
                            .size(10.0)
                            .color(LR_TEXT_SECONDARY));
                    });
                });
                
                ui.add_space(4.0);
                
                // Navigator preview with viewport rectangle
                let (response, painter) = ui.allocate_painter(
                    Vec2::new(available_width - 8.0, nav_height),
                    egui::Sense::click_and_drag()
                );
                let nav_rect = response.rect;
                
                // Background
                painter.rect_filled(nav_rect, Rounding::ZERO, LR_BG_INPUT);
                
                // Draw thumbnail preview
                if let Some(tex) = &self.current_texture {
                    let tex_size = tex.size_vec2();
                    
                    // Calculate scaled size to fit in navigator
                    let scale = (nav_rect.width() / tex_size.x).min(nav_rect.height() / tex_size.y);
                    let scaled_size = tex_size * scale;
                    
                    let image_rect = Rect::from_center_size(nav_rect.center(), scaled_size);
                    
                    painter.image(
                        tex.id(),
                        image_rect,
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    
                    // Draw viewport rectangle (showing what's visible in main view)
                    if self.zoom > 1.0 || self.pan_offset != Vec2::ZERO {
                        // Calculate the viewport rectangle
                        let view_size = self.available_view_size;
                        let display_size = tex_size * self.zoom;
                        
                        // What portion of the image is visible
                        let visible_width = (view_size.x / display_size.x).min(1.0);
                        let visible_height = (view_size.y / display_size.y).min(1.0);
                        
                        // Calculate center offset as a fraction of image
                        let pan_fraction_x = -self.pan_offset.x / display_size.x;
                        let pan_fraction_y = -self.pan_offset.y / display_size.y;
                        
                        // Viewport center in normalized coordinates (0 to 1)
                        let center_x = 0.5 + pan_fraction_x;
                        let center_y = 0.5 + pan_fraction_y;
                        
                        // Viewport rect in navigator space
                        let vp_width = scaled_size.x * visible_width;
                        let vp_height = scaled_size.y * visible_height;
                        let vp_center = egui::pos2(
                            image_rect.left() + scaled_size.x * center_x,
                            image_rect.top() + scaled_size.y * center_y
                        );
                        
                        let viewport_rect = Rect::from_center_size(
                            vp_center,
                            Vec2::new(vp_width, vp_height)
                        );
                        
                        // Draw viewport rectangle (white border like Lightroom)
                        painter.rect_stroke(
                            viewport_rect,
                            Rounding::ZERO,
                            Stroke::new(1.5, Color32::WHITE)
                        );
                        
                        // Handle click-drag to pan
                        if response.dragged() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                // Convert click position to image coordinates
                                let rel_x = (pos.x - image_rect.left()) / scaled_size.x;
                                let rel_y = (pos.y - image_rect.top()) / scaled_size.y;
                                
                                // Set pan to center view on clicked position
                                let new_center_x = 0.5 - rel_x;
                                let new_center_y = 0.5 - rel_y;
                                
                                self.pan_offset = Vec2::new(
                                    new_center_x * display_size.x,
                                    new_center_y * display_size.y
                                );
                                self.target_pan = self.pan_offset;
                            }
                        }
                    } else if self.zoom < 1.0 {
                        // When zoomed out, show that entire image is visible
                        // Draw a subtle border around the image area
                        painter.rect_stroke(
                            image_rect,
                            Rounding::ZERO,
                            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 100))
                        );
                    }
                } else {
                    // No image loaded
                    painter.text(
                        nav_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "No image",
                        egui::FontId::proportional(11.0),
                        LR_TEXT_SECONDARY,
                    );
                }
            });

        // Bottom border
        ui.painter().hline(
            header_rect.x_range(),
            ui.available_rect_before_wrap().bottom(),
            Stroke::new(1.0, LR_BORDER)
        );
    }

    fn render_histogram_panel(&self, ui: &mut egui::Ui) {
        lr_collapsible_panel(ui, "Histogram", true, |ui| {
            let height = 80.0;
            let (response, painter) = ui.allocate_painter(
                Vec2::new(ui.available_width() - 8.0, height),
                egui::Sense::hover()
            );
            let rect = response.rect;
            
            // Background
            painter.rect_filled(rect, Rounding::same(2.0), LR_BG_INPUT);
            
            if let Some(histogram) = &self.histogram_data {
                if histogram.len() >= 3 {
                    let w = rect.width() - 4.0;
                    let h = rect.height() - 4.0;
                    let offset = 2.0;
                    
                    // Find max for scaling (use log scale for better visualization)
                    let max_val = histogram[0].iter()
                        .chain(histogram[1].iter())
                        .chain(histogram[2].iter())
                        .max()
                        .copied()
                        .unwrap_or(1) as f32;
                    
                    // Draw filled histograms with transparency
                    let num_bins = 256.min(histogram[0].len());
                    for i in 0..num_bins {
                        let x = rect.left() + offset + (i as f32 / 255.0) * w;
                        let base_y = rect.bottom() - offset;
                        
                        // Red channel
                        let r_h = (histogram[0][i] as f32 / max_val).sqrt() * h;
                        painter.line_segment(
                            [egui::pos2(x, base_y), egui::pos2(x, base_y - r_h)],
                            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 80, 80, 120)),
                        );
                        
                        // Green channel
                        let g_h = (histogram[1][i] as f32 / max_val).sqrt() * h;
                        painter.line_segment(
                            [egui::pos2(x, base_y), egui::pos2(x, base_y - g_h)],
                            Stroke::new(1.0, Color32::from_rgba_unmultiplied(80, 255, 80, 120)),
                        );
                        
                        // Blue channel
                        let b_h = (histogram[2][i] as f32 / max_val).sqrt() * h;
                        painter.line_segment(
                            [egui::pos2(x, base_y), egui::pos2(x, base_y - b_h)],
                            Stroke::new(1.0, Color32::from_rgba_unmultiplied(80, 80, 255, 120)),
                        );
                    }
                }
            }
        });
    }

    fn render_basic_panel(&mut self, ui: &mut egui::Ui) {
        let previous_adjustments = self.adjustments.clone();
        let mut adjustments_changed = false;
        
        lr_collapsible_panel(ui, "Basic", true, |ui| {
            ui.spacing_mut().slider_width = ui.available_width() - 80.0;
            
            // WB: White Balance section
            ui.horizontal(|ui| {
                ui.label(RichText::new("WB:").size(11.0).color(LR_TEXT_LABEL));
                ui.add_space(36.0);
                ui.label(RichText::new("As Shot").size(10.0).color(LR_TEXT_SECONDARY));
            });
            
            ui.add_space(4.0);
            
            // Temperature
            if lr_slider(ui, "Temp", &mut self.adjustments.temperature, -1.0..=1.0, "") {
                adjustments_changed = true;
                if self.should_apply_adjustments() {
                    self.refresh_adjustments();
                }
            }
            
            // Tint
            if lr_slider(ui, "Tint", &mut self.adjustments.tint, -1.0..=1.0, "") {
                adjustments_changed = true;
                if self.should_apply_adjustments() {
                    self.refresh_adjustments();
                }
            }
            
            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);
            
            // Tone section header
            ui.label(RichText::new("Tone").size(11.0).color(LR_TEXT_LABEL));
            ui.add_space(4.0);
            
            // Exposure
            if lr_slider(ui, "Exposure", &mut self.adjustments.exposure, -3.0..=3.0, " EV") {
                adjustments_changed = true;
                if self.should_apply_adjustments() {
                    self.refresh_adjustments();
                }
            }
            
            // Contrast (convert from 0.5-2.0 to -100 to +100 display)
            let mut contrast_display = (self.adjustments.contrast - 1.0) * 100.0;
            if lr_slider(ui, "Contrast", &mut contrast_display, -100.0..=100.0, "") {
                self.adjustments.contrast = 1.0 + contrast_display / 100.0;
                adjustments_changed = true;
                if self.should_apply_adjustments() {
                    self.refresh_adjustments();
                }
            }
            
            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);
            
            // Highlights (convert to -100 to +100)
            let mut highlights_display = self.adjustments.highlights * 100.0;
            if lr_slider(ui, "Highlights", &mut highlights_display, -100.0..=100.0, "") {
                self.adjustments.highlights = highlights_display / 100.0;
                adjustments_changed = true;
                if self.should_apply_adjustments() {
                    self.refresh_adjustments();
                }
            }
            
            // Shadows
            let mut shadows_display = self.adjustments.shadows * 100.0;
            if lr_slider(ui, "Shadows", &mut shadows_display, -100.0..=100.0, "") {
                self.adjustments.shadows = shadows_display / 100.0;
                adjustments_changed = true;
                if self.should_apply_adjustments() {
                    self.refresh_adjustments();
                }
            }
            
            // Whites
            let mut whites_display = self.adjustments.whites * 100.0;
            if lr_slider(ui, "Whites", &mut whites_display, -100.0..=100.0, "") {
                self.adjustments.whites = whites_display / 100.0;
                adjustments_changed = true;
                if self.should_apply_adjustments() {
                    self.refresh_adjustments();
                }
            }
            
            // Blacks
            let mut blacks_display = self.adjustments.blacks * 100.0;
            if lr_slider(ui, "Blacks", &mut blacks_display, -100.0..=100.0, "") {
                self.adjustments.blacks = blacks_display / 100.0;
                adjustments_changed = true;
                if self.should_apply_adjustments() {
                    self.refresh_adjustments();
                }
            }
            
            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);
            
            // Presence section
            ui.label(RichText::new("Presence").size(11.0).color(LR_TEXT_LABEL));
            ui.add_space(4.0);
            
            // Saturation (convert from 0-2 to -100 to +100)
            let mut sat_display = (self.adjustments.saturation - 1.0) * 100.0;
            if lr_slider(ui, "Saturation", &mut sat_display, -100.0..=100.0, "") {
                self.adjustments.saturation = 1.0 + sat_display / 100.0;
                adjustments_changed = true;
                if self.should_apply_adjustments() {
                    self.refresh_adjustments();
                }
            }
            
            // Sharpening (convert to 0-100)
            let mut sharp_display = self.adjustments.sharpening * 50.0;
            if lr_slider(ui, "Sharpening", &mut sharp_display, 0.0..=100.0, "") {
                self.adjustments.sharpening = sharp_display / 50.0;
                adjustments_changed = true;
                if self.should_apply_adjustments() {
                    self.refresh_adjustments();
                }
            }
            
            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);
            
            // Frame section
            ui.label(RichText::new("Frame").size(11.0).color(LR_TEXT_LABEL));
            ui.add_space(4.0);
            
            // Enable frame
            let mut frame_enabled = self.adjustments.frame_enabled;
            if ui.checkbox(&mut frame_enabled, RichText::new("Enable").size(10.0).color(LR_TEXT_LABEL)).changed() {
                self.adjustments.frame_enabled = frame_enabled;
                adjustments_changed = true;
                if self.should_apply_adjustments() {
                    self.refresh_adjustments();
                }
            }
            
            if self.adjustments.frame_enabled {
                // Thickness
                if lr_slider(ui, "Thickness", &mut self.adjustments.frame_thickness, 1.0..=100.0, "px") {
                    adjustments_changed = true;
                    if self.should_apply_adjustments() {
                        self.refresh_adjustments();
                    }
                }
                
                // Color picker
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Color").size(10.0).color(LR_TEXT_LABEL));
                    ui.add_space(8.0);
                    
                    let mut color = self.adjustments.frame_color;
                    
                    if ui.color_edit_button_rgb(&mut color).changed() {
                        self.adjustments.frame_color = color;
                        adjustments_changed = true;
                        if self.should_apply_adjustments() {
                            self.refresh_adjustments();
                        }
                    }
                });
            }
            
            ui.add_space(8.0);
            
            // Reset button
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("Reset").size(10.0).color(LR_TEXT_SECONDARY))
                    .fill(LR_BG_INPUT)
                    .stroke(Stroke::new(1.0, LR_BORDER))
                    .rounding(Rounding::same(2.0)))
                    .clicked() {
                    self.adjustments = ImageAdjustments::default();
                    self.refresh_adjustments();
                    if let Some(path) = self.get_current_path() {
                        self.undo_history.push(FileOperation::Adjust {
                            path,
                            adjustments: self.adjustments.clone(),
                            previous_adjustments: previous_adjustments.clone(),
                        });
                    }
                }
            });
        });
        
        // Film Emulation panel (separate like Lightroom's Detail panel)
        self.render_film_emulation_panel(ui, &previous_adjustments, &mut adjustments_changed);
        
        // Record undo if adjustments changed
        if adjustments_changed && self.adjustments != previous_adjustments {
            if let Some(path) = self.get_current_path() {
                self.undo_history.push(FileOperation::Adjust {
                    path,
                    adjustments: self.adjustments.clone(),
                    previous_adjustments,
                });
            }
        }
    }
    
    fn render_film_emulation_panel(&mut self, ui: &mut egui::Ui, _previous_adjustments: &ImageAdjustments, adjustments_changed: &mut bool) {
        lr_collapsible_panel(ui, "Film Emulation", false, |ui| {
            ui.spacing_mut().slider_width = ui.available_width() - 80.0;
            
            // Enable film emulation
            let mut film_enabled = self.adjustments.film.enabled;
            if ui.checkbox(&mut film_enabled, RichText::new("Enable").size(10.0).color(LR_TEXT_LABEL)).changed() {
                self.adjustments.film.enabled = film_enabled;
                *adjustments_changed = true;
                if self.should_apply_adjustments() {
                    self.refresh_adjustments();
                }
            }
            
            if self.adjustments.film.enabled {
                ui.add_space(4.0);
                lr_separator(ui);
                ui.add_space(4.0);
                
                // Profile/Preset selector (like Lightroom)
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Profile:").size(11.0).color(LR_TEXT_LABEL));
                    ui.add_space(8.0);
                    egui::ComboBox::from_id_salt("film_preset")
                        .width(ui.available_width() - 8.0)
                        .selected_text(self.current_film_preset.name())
                        .show_ui(ui, |ui| {
                            for preset in FilmPreset::all() {
                                let selected = *preset == self.current_film_preset;
                                if ui.selectable_label(selected, preset.name()).clicked() {
                                    self.current_film_preset = *preset;
                                    let prev_adj = self.adjustments.clone();
                                    self.adjustments.apply_preset(*preset);
                                    self.refresh_adjustments();
                                    if let Some(path) = self.get_current_path() {
                                        self.undo_history.push(FileOperation::Adjust {
                                            path,
                                            adjustments: self.adjustments.clone(),
                                            previous_adjustments: prev_adj,
                                        });
                                    }
                                }
                            }
                        });
                });
                
                ui.add_space(4.0);
                lr_separator(ui);
                ui.add_space(4.0);
                
                // Grain
                ui.label(RichText::new("Grain").size(11.0).color(LR_TEXT_LABEL));
                
                let mut grain_display = self.adjustments.film.grain_amount * 100.0;
                if lr_slider(ui, "Amount", &mut grain_display, 0.0..=100.0, "") {
                    self.adjustments.film.grain_amount = grain_display / 100.0;
                    *adjustments_changed = true;
                    if self.should_apply_adjustments() { self.refresh_adjustments(); }
                }
                
                let mut size_display = self.adjustments.film.grain_size * 50.0;
                if lr_slider(ui, "Size", &mut size_display, 25.0..=100.0, "") {
                    self.adjustments.film.grain_size = size_display / 50.0;
                    *adjustments_changed = true;
                    if self.should_apply_adjustments() { self.refresh_adjustments(); }
                }
                
                let mut rough_display = self.adjustments.film.grain_roughness * 100.0;
                if lr_slider(ui, "Roughness", &mut rough_display, 0.0..=100.0, "") {
                    self.adjustments.film.grain_roughness = rough_display / 100.0;
                    *adjustments_changed = true;
                    if self.should_apply_adjustments() { self.refresh_adjustments(); }
                }
                
                ui.add_space(4.0);
                lr_separator(ui);
                ui.add_space(4.0);
                
                // Vignette
                ui.label(RichText::new("Vignette").size(11.0).color(LR_TEXT_LABEL));
                
                let mut vig_display = self.adjustments.film.vignette_amount * 100.0;
                if lr_slider(ui, "Amount", &mut vig_display, 0.0..=100.0, "") {
                    self.adjustments.film.vignette_amount = vig_display / 100.0;
                    *adjustments_changed = true;
                    if self.should_apply_adjustments() { self.refresh_adjustments(); }
                }
                
                let mut soft_display = (self.adjustments.film.vignette_softness - 0.5) / 1.5 * 100.0;
                if lr_slider(ui, "Feather", &mut soft_display, 0.0..=100.0, "") {
                    self.adjustments.film.vignette_softness = 0.5 + soft_display / 100.0 * 1.5;
                    *adjustments_changed = true;
                    if self.should_apply_adjustments() { self.refresh_adjustments(); }
                }
                
                ui.add_space(4.0);
                lr_separator(ui);
                ui.add_space(4.0);
                
                // Halation
                ui.label(RichText::new("Halation").size(11.0).color(LR_TEXT_LABEL));
                
                let mut hal_display = self.adjustments.film.halation_amount * 100.0;
                if lr_slider(ui, "Amount", &mut hal_display, 0.0..=100.0, "") {
                    self.adjustments.film.halation_amount = hal_display / 100.0;
                    *adjustments_changed = true;
                    if self.should_apply_adjustments() { self.refresh_adjustments(); }
                }
                
                let mut rad_display = (self.adjustments.film.halation_radius - 0.5) / 2.5 * 100.0;
                if lr_slider(ui, "Radius", &mut rad_display, 0.0..=100.0, "") {
                    self.adjustments.film.halation_radius = 0.5 + rad_display / 100.0 * 2.5;
                    *adjustments_changed = true;
                    if self.should_apply_adjustments() { self.refresh_adjustments(); }
                }
                
                ui.add_space(4.0);
                lr_separator(ui);
                ui.add_space(4.0);
                
                // Tone Curve
                ui.label(RichText::new("Tone Curve").size(11.0).color(LR_TEXT_LABEL));
                
                let mut shadows_tc = self.adjustments.film.tone_curve_shadows * 100.0;
                if lr_slider(ui, "Shadows", &mut shadows_tc, -100.0..=100.0, "") {
                    self.adjustments.film.tone_curve_shadows = shadows_tc / 100.0;
                    *adjustments_changed = true;
                    if self.should_apply_adjustments() { self.refresh_adjustments(); }
                }
                
                let mut mids_tc = self.adjustments.film.tone_curve_midtones * 100.0;
                if lr_slider(ui, "Midtones", &mut mids_tc, -100.0..=100.0, "") {
                    self.adjustments.film.tone_curve_midtones = mids_tc / 100.0;
                    *adjustments_changed = true;
                    if self.should_apply_adjustments() { self.refresh_adjustments(); }
                }
                
                let mut highs_tc = self.adjustments.film.tone_curve_highlights * 100.0;
                if lr_slider(ui, "Highlights", &mut highs_tc, -100.0..=100.0, "") {
                    self.adjustments.film.tone_curve_highlights = highs_tc / 100.0;
                    *adjustments_changed = true;
                    if self.should_apply_adjustments() { self.refresh_adjustments(); }
                }
                
                let mut scurve = self.adjustments.film.s_curve_strength * 100.0;
                if lr_slider(ui, "S-Curve", &mut scurve, 0.0..=100.0, "") {
                    self.adjustments.film.s_curve_strength = scurve / 100.0;
                    *adjustments_changed = true;
                    if self.should_apply_adjustments() { self.refresh_adjustments(); }
                }
            }
        });
    }
    
    fn render_metadata_info_panel(&mut self, ui: &mut egui::Ui) {
        lr_collapsible_panel(ui, "Metadata", true, |ui| {
            if let Some(exif) = &self.current_exif {
                if !exif.has_data() {
                    ui.label(RichText::new("No EXIF data").size(10.0).color(LR_TEXT_SECONDARY));
                } else {
                    // Camera info
                    lr_info_row(ui, "Camera", exif.camera_model.as_deref());
                    lr_info_row(ui, "Lens", exif.lens.as_deref());
                    
                    let fl = exif.focal_length_formatted();
                    if !fl.is_empty() { lr_info_row(ui, "Focal Length", Some(&fl)); }
                    
                    let ap = exif.aperture_formatted();
                    if !ap.is_empty() { lr_info_row(ui, "Aperture", Some(&ap)); }
                    
                    lr_info_row(ui, "Shutter", exif.shutter_speed.as_deref());
                    lr_info_row(ui, "ISO", exif.iso.as_deref());
                    lr_info_row(ui, "Date", exif.date_taken.as_deref());
                    lr_info_row(ui, "Dimensions", exif.dimensions.as_deref());
                    
                    if exif.gps_latitude.is_some() && exif.gps_longitude.is_some() {
                        let gps = format!("{:.4}, {:.4}", 
                            exif.gps_latitude.unwrap_or(0.0), 
                            exif.gps_longitude.unwrap_or(0.0));
                        lr_info_row(ui, "GPS", Some(&gps));
                    }
                }
            } else {
                ui.label(RichText::new("No metadata").size(10.0).color(LR_TEXT_SECONDARY));
            }
        });
    }
    
    fn render_keywording_panel(&mut self, ui: &mut egui::Ui) {
        lr_collapsible_panel(ui, "Keywording", true, |ui| {
            if let Some(path) = self.get_current_path() {
                let metadata = self.metadata_db.get(&path);
                
                // Star rating (like Lightroom)
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Rating").size(10.0).color(LR_TEXT_LABEL));
                    ui.add_space(16.0);
                    
                    for i in 1..=5 {
                        let star = if i <= metadata.rating { "★" } else { "☆" };
                        let color = if i <= metadata.rating {
                            Color32::from_rgb(230, 180, 50)
                        } else {
                            LR_TEXT_SECONDARY
                        };
                        
                        if ui.add(egui::Button::new(RichText::new(star).size(14.0).color(color))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE)
                            .min_size(Vec2::new(16.0, 16.0)))
                            .clicked() {
                            let new_rating = if metadata.rating == i { 0 } else { i };
                            self.set_rating(new_rating);
                        }
                    }
                });
                
                ui.add_space(4.0);
                
                // Color labels (like Lightroom)
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Label").size(10.0).color(LR_TEXT_LABEL));
                    ui.add_space(24.0);
                    
                    for label in [ColorLabel::None, ColorLabel::Red, ColorLabel::Yellow, 
                                  ColorLabel::Green, ColorLabel::Blue, ColorLabel::Purple] {
                        let is_selected = metadata.color_label == label;
                        let color = label.to_color();
                        
                        let size = 14.0;
                        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click());
                        
                        if label == ColorLabel::None {
                            ui.painter().rect_stroke(rect.shrink(1.0), Rounding::same(2.0),
                                Stroke::new(1.0, LR_TEXT_SECONDARY));
                            if is_selected {
                                // X mark for "none"
                                let c = rect.center();
                                let d = 3.0;
                                ui.painter().line_segment(
                                    [egui::pos2(c.x - d, c.y - d), egui::pos2(c.x + d, c.y + d)],
                                    Stroke::new(1.0, LR_TEXT_SECONDARY),
                                );
                                ui.painter().line_segment(
                                    [egui::pos2(c.x + d, c.y - d), egui::pos2(c.x - d, c.y + d)],
                                    Stroke::new(1.0, LR_TEXT_SECONDARY),
                                );
                            }
                        } else {
                            ui.painter().rect_filled(rect.shrink(1.0), Rounding::same(2.0), color);
                            if is_selected {
                                ui.painter().rect_stroke(rect, Rounding::same(3.0),
                                    Stroke::new(2.0, Color32::WHITE));
                            }
                        }
                        
                        if response.clicked() {
                            self.set_color_label(label);
                        }
                    }
                });
                
                ui.add_space(8.0);
                
                // Keywords/Tags
                ui.label(RichText::new("Keywords").size(10.0).color(LR_TEXT_LABEL));
                ui.add_space(2.0);
                
                if metadata.tags.is_empty() {
                    ui.label(RichText::new("No keywords").size(10.0).color(LR_TEXT_SECONDARY).italics());
                } else {
                    ui.horizontal_wrapped(|ui| {
                        for tag in metadata.tags.clone() {
                            ui.add(egui::Button::new(RichText::new(&tag).size(9.0).color(LR_TEXT_PRIMARY))
                                .fill(LR_BG_INPUT)
                                .stroke(Stroke::new(1.0, LR_BORDER))
                                .rounding(Rounding::same(8.0)));
                        }
                    });
                }
            }
        });
    }
    
    fn render_folders_panel(&mut self, ui: &mut egui::Ui) {
        lr_collapsible_panel(ui, "Folders", true, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("folder_tree")
                .auto_shrink([false, true])
                .max_height(300.0)
                .show(ui, |ui| {
                    #[cfg(windows)]
                    {
                        use std::path::Path;
                        for drive in b'A'..=b'Z' {
                            let drive_str = format!("{}:\\", drive as char);
                            let drive_path = Path::new(&drive_str);
                            if drive_path.exists() {
                                self.render_folder_node(ui, drive_path.to_path_buf(), 0);
                            }
                        }
                    }
                    
                    #[cfg(unix)]
                    {
                        self.render_folder_node(ui, PathBuf::from("/"), 0);
                    }
                    
                    #[cfg(not(any(windows, unix)))]
                    {
                        if let Ok(current) = std::env::current_dir() {
                            if let Some(parent) = current.parent() {
                                self.render_folder_node(ui, parent.to_path_buf(), 0);
                            }
                        }
                    }
                });
        });
    }
    
    fn render_folder_node(&mut self, ui: &mut egui::Ui, path: PathBuf, depth: usize) {
        if depth > 10 {
            return;
        }
        
        let is_expanded = self.expanded_dirs.contains(&path);
        let is_current_folder = self.current_folder.as_ref() == Some(&path);
        
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        
        let indent = depth as f32 * 12.0;
        
        ui.horizontal(|ui| {
            ui.add_space(indent);
            
            // Expand triangle (like Lightroom)
            if path.is_dir() {
                let icon = if is_expanded { "▼" } else { "▶" };
                if ui.add(egui::Button::new(RichText::new(icon).size(8.0).color(LR_TEXT_SECONDARY))
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE)
                    .min_size(Vec2::new(14.0, 14.0)))
                    .clicked() {
                    if is_expanded {
                        self.expanded_dirs.remove(&path);
                    } else {
                        self.expanded_dirs.insert(path.clone());
                    }
                }
            } else {
                ui.add_space(14.0);
            }
            
            // Folder icon and name
            let folder_color = if is_current_folder {
                Color32::from_rgb(200, 170, 100)
            } else {
                LR_TEXT_SECONDARY
            };
            
            let bg_color = if is_current_folder {
                Color32::from_rgba_unmultiplied(200, 170, 100, 30)
            } else {
                Color32::TRANSPARENT
            };
            
            if ui.add(egui::Button::new(RichText::new(format!("📁 {}", name)).size(10.0).color(folder_color))
                .fill(bg_color)
                .stroke(Stroke::NONE)
                .wrap())
                .clicked() {
                if path.is_dir() {
                    self.load_folder(path.clone());
                } else if path.is_file() {
                    self.load_image_file(path.clone());
                }
            }
        });
        
        // Render children if expanded
        if is_expanded && path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                let mut dirs: Vec<PathBuf> = entries
                    .flatten()
                    .filter_map(|entry| {
                        let p = entry.path();
                        if p.is_dir() && !p.file_name()
                            .map(|n| n.to_string_lossy().starts_with('.'))
                            .unwrap_or(false) {
                            Some(p)
                        } else {
                            None
                        }
                    })
                    .collect();
                
                dirs.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
                
                for dir_path in dirs {
                    self.render_folder_node(ui, dir_path, depth + 1);
                }
            }
        }
    }
}

// Lightroom-style collapsible panel
fn lr_collapsible_panel<R>(
    ui: &mut egui::Ui,
    title: &str,
    default_open: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::CollapsingResponse<R> {
    // Panel header background
    let header_rect = ui.available_rect_before_wrap();
    let header_rect = Rect::from_min_size(
        header_rect.min,
        Vec2::new(ui.available_width(), 24.0)
    );
    
    ui.painter().rect_filled(header_rect, Rounding::ZERO, LR_HEADER_BG);
    ui.painter().hline(
        header_rect.x_range(),
        header_rect.bottom(),
        Stroke::new(1.0, LR_BORDER)
    );
    
    let response = egui::CollapsingHeader::new(RichText::new(title).size(11.0).color(LR_TEXT_PRIMARY).strong())
        .default_open(default_open)
        .show(ui, |ui| {
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(LR_BG_PANEL)
                .inner_margin(Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    add_contents(ui)
                }).inner
        });
    
    // Bottom border
    ui.painter().hline(
        ui.available_rect_before_wrap().x_range(),
        ui.cursor().top(),
        Stroke::new(1.0, LR_BORDER)
    );
    
    response
}

// Lightroom-style separator (subtle line)
fn lr_separator(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        rect.x_range(),
        rect.top(),
        Stroke::new(1.0, Color32::from_rgb(60, 60, 60))
    );
    ui.add_space(1.0);
}

// Lightroom-style slider with label on left, value on right
fn lr_slider(ui: &mut egui::Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str) -> bool {
    let mut changed = false;
    
    ui.horizontal(|ui| {
        // Fixed-width label column
        ui.allocate_ui_with_layout(
            Vec2::new(70.0, 18.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.label(RichText::new(label).size(10.0).color(LR_TEXT_LABEL));
            }
        );
        
        // Slider
        let slider_width = ui.available_width() - 45.0;
        ui.spacing_mut().slider_width = slider_width;
        
        let slider = egui::Slider::new(value, range.clone())
            .show_value(false)
            .trailing_fill(true);
        
        if ui.add(slider).changed() {
            changed = true;
        }
        
        // Value display
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let display_val = if (*range.end() - *range.start()) > 10.0 {
                format!("{:.0}{}", *value, suffix)
            } else {
                format!("{:.2}{}", *value, suffix)
            };
            ui.label(RichText::new(display_val).size(10.0).color(LR_TEXT_SECONDARY).monospace());
        });
    });
    
    changed
}

// Lightroom-style info row (label: value)
fn lr_info_row(ui: &mut egui::Ui, label: &str, value: Option<&str>) {
    if let Some(v) = value {
        if !v.is_empty() && v != "Unknown" {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{}:", label)).size(10.0).color(LR_TEXT_SECONDARY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(v).size(10.0).color(LR_TEXT_PRIMARY));
                });
            });
        }
    }
}
