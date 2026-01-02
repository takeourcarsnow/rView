use crate::app::ImageViewerApp;
use crate::image_loader::{ImageAdjustments, FilmPreset};
use crate::metadata::FileOperation;
use egui::{self, Color32, RichText, Vec2, Rounding, Stroke, Rect};

// Lightroom-inspired color scheme
const LR_BG_INPUT: Color32 = Color32::from_rgb(34, 34, 34);
const LR_BG_PANEL: Color32 = Color32::from_rgb(51, 51, 51);
const LR_BORDER: Color32 = Color32::from_rgb(28, 28, 28);
const LR_TEXT_LABEL: Color32 = Color32::from_rgb(180, 180, 180);
const LR_TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 140);

pub fn render_basic_panel(app: &mut ImageViewerApp, ui: &mut egui::Ui) {
    let previous_adjustments = app.adjustments.clone();
    let mut adjustments_changed = false;

    lr_collapsible_panel(ui, "Basic", true, |ui| {
        ui.spacing_mut().slider_width = ui.available_width() - 80.0;

        // WB: White Balance section
        ui.horizontal(|ui| {
            ui.label(RichText::new("WB:").size(11.0).color(LR_TEXT_LABEL));
            ui.add_space(36.0);
            // 'As Shot' removed — no preset label shown here
        });

        ui.add_space(4.0);

        // Temperature
        if lr_slider(ui, "Temp", &mut app.adjustments.temperature, -1.0..=1.0, "", 0.0) {
            adjustments_changed = true;
            if app.should_apply_adjustments() {
                app.refresh_adjustments();
            }
        }

        // Tint
        if lr_slider(ui, "Tint", &mut app.adjustments.tint, -1.0..=1.0, "", 0.0) {
            adjustments_changed = true;
            if app.should_apply_adjustments() {
                app.refresh_adjustments();
            }
        }

        ui.add_space(4.0);
        lr_separator(ui);
        ui.add_space(4.0);

        // Tone section header
        ui.label(RichText::new("Tone").size(11.0).color(LR_TEXT_LABEL));
        ui.add_space(4.0);

        // Exposure
        if lr_slider(ui, "Exposure", &mut app.adjustments.exposure, -3.0..=3.0, " EV", 0.0) {
            adjustments_changed = true;
            if app.should_apply_adjustments() {
                app.refresh_adjustments();
            }
        }

        // Contrast (convert from 0.5-2.0 to -100 to +100 display)
        let mut contrast_display = (app.adjustments.contrast - 1.0) * 100.0;
        if lr_slider(ui, "Contrast", &mut contrast_display, -100.0..=100.0, "", 0.0) {
            app.adjustments.contrast = 1.0 + contrast_display / 100.0;
            adjustments_changed = true;
            if app.should_apply_adjustments() {
                app.refresh_adjustments();
            }
        }

        ui.add_space(4.0);
        lr_separator(ui);
        ui.add_space(4.0);

        // Highlights (convert to -100 to +100)
        let mut highlights_display = app.adjustments.highlights * 100.0;
        if lr_slider(ui, "Highlights", &mut highlights_display, -100.0..=100.0, "", 0.0) {
            app.adjustments.highlights = highlights_display / 100.0;
            adjustments_changed = true;
            if app.should_apply_adjustments() {
                app.refresh_adjustments();
            }
        }

        // Shadows
        let mut shadows_display = app.adjustments.shadows * 100.0;
        if lr_slider(ui, "Shadows", &mut shadows_display, -100.0..=100.0, "", 0.0) {
            app.adjustments.shadows = shadows_display / 100.0;
            adjustments_changed = true;
            if app.should_apply_adjustments() {
                app.refresh_adjustments();
            }
        }

        // Whites
        let mut whites_display = app.adjustments.whites * 100.0;
        if lr_slider(ui, "Whites", &mut whites_display, -100.0..=100.0, "", 0.0) {
            app.adjustments.whites = whites_display / 100.0;
            adjustments_changed = true;
            if app.should_apply_adjustments() {
                app.refresh_adjustments();
            }
        }

        // Blacks
        let mut blacks_display = app.adjustments.blacks * 100.0;
        if lr_slider(ui, "Blacks", &mut blacks_display, -100.0..=100.0, "", 0.0) {
            app.adjustments.blacks = blacks_display / 100.0;
            adjustments_changed = true;
            if app.should_apply_adjustments() {
                app.refresh_adjustments();
            }
        }

        ui.add_space(4.0);
        lr_separator(ui);
        ui.add_space(4.0);

        // Presence section
        ui.label(RichText::new("Presence").size(11.0).color(LR_TEXT_LABEL));
        ui.add_space(4.0);

        // Saturation (convert from 0-2 to -100 to +100)
        let mut sat_display = (app.adjustments.saturation - 1.0) * 100.0;
        if lr_slider(ui, "Saturation", &mut sat_display, -100.0..=100.0, "", 0.0) {
            app.adjustments.saturation = 1.0 + sat_display / 100.0;
            adjustments_changed = true;
            if app.should_apply_adjustments() {
                app.refresh_adjustments();
            }
        }

        // Sharpening (convert to 0-100)
        let mut sharp_display = app.adjustments.sharpening * 50.0;
        if lr_slider(ui, "Sharpening", &mut sharp_display, 0.0..=100.0, "", 0.0) {
            app.adjustments.sharpening = sharp_display / 50.0;
            adjustments_changed = true;
            if app.should_apply_adjustments() {
                app.refresh_adjustments();
            }
        }

        ui.add_space(4.0);
        lr_separator(ui);
        ui.add_space(4.0);

        // Frame section
        ui.label(RichText::new("Frame").size(11.0).color(LR_TEXT_LABEL));
        ui.add_space(4.0);

        // Enable frame
        let mut frame_enabled = app.adjustments.frame_enabled;
        if ui.checkbox(&mut frame_enabled, RichText::new("Enable").size(10.0).color(LR_TEXT_LABEL)).changed() {
            app.adjustments.frame_enabled = frame_enabled;
            adjustments_changed = true;
            if app.should_apply_adjustments() {
                app.refresh_adjustments();
            }
        }

        if app.adjustments.frame_enabled {
            // Thickness
            if lr_slider(ui, "Thickness", &mut app.adjustments.frame_thickness, 1.0..=100.0, "px", 10.0) {
                adjustments_changed = true;
                if app.should_apply_adjustments() {
                    app.refresh_adjustments();
                }
            }

            // Color picker
            ui.horizontal(|ui| {
                ui.label(RichText::new("Color").size(10.0).color(LR_TEXT_LABEL));
                ui.add_space(8.0);

                let mut color = app.adjustments.frame_color;

                if ui.color_edit_button_rgb(&mut color).changed() {
                    app.adjustments.frame_color = color;
                    adjustments_changed = true;
                    if app.should_apply_adjustments() {
                        app.refresh_adjustments();
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
                app.adjustments = ImageAdjustments::default();
                app.current_film_preset = FilmPreset::None;
                app.refresh_adjustments();
                if let Some(path) = app.get_current_path() {
                    app.undo_history.push(FileOperation::Adjust {
                        path: path.clone(),
                        adjustments: app.adjustments.clone(),
                        previous_adjustments: Box::new(previous_adjustments.clone()),
                    });
                    // Save reset adjustments to metadata database
                    app.metadata_db.set_adjustments(path.clone(), &app.adjustments);
                    app.metadata_db.save();
                    // Invalidate thumbnail to regenerate with new adjustments
                    app.thumbnail_textures.remove(&path);
                    app.thumbnail_requests.remove(&path);
                }
            }
        });
    });

    // Film Emulation panel (separate like Lightroom's Detail panel)
    render_film_emulation_panel(app, ui, &previous_adjustments, &mut adjustments_changed);

    // Record undo if adjustments changed and save to metadata
    if adjustments_changed && app.adjustments != previous_adjustments {
        if let Some(path) = app.get_current_path() {
            app.undo_history.push(FileOperation::Adjust {
                path: path.clone(),
                adjustments: app.adjustments.clone(),
                previous_adjustments: Box::new(previous_adjustments.clone()),
            });
            // Save adjustments to metadata database
            app.metadata_db.set_adjustments(path.clone(), &app.adjustments);
            app.metadata_db.save();
            // Invalidate thumbnail to regenerate with new adjustments
            app.thumbnail_textures.remove(&path);
            app.thumbnail_requests.remove(&path);
        }
    }
}

pub fn render_film_emulation_panel(app: &mut ImageViewerApp, ui: &mut egui::Ui, _previous_adjustments: &ImageAdjustments, adjustments_changed: &mut bool) {
    lr_collapsible_panel(ui, "Film Emulation", false, |ui| {
        ui.spacing_mut().slider_width = ui.available_width() - 80.0;

        // Profile/Preset selector (like Lightroom) - always visible
        ui.horizontal(|ui| {
            ui.label(RichText::new("Profile:").size(11.0).color(LR_TEXT_LABEL));
            ui.add_space(8.0);
            egui::ComboBox::from_id_salt("film_preset")
                .width(ui.available_width() - 8.0)
                .selected_text(app.current_film_preset.name())
                .show_ui(ui, |ui| {
                    for preset in FilmPreset::all() {
                        let selected = *preset == app.current_film_preset;
                        let response = ui.selectable_label(selected, preset.name())
                            .on_hover_text(preset.description());
                        if response.clicked() {
                            app.current_film_preset = *preset;
                            let prev_adj = app.adjustments.clone();
                            app.adjustments.apply_preset(*preset);
                            app.refresh_adjustments();
                            if let Some(path) = app.get_current_path() {
                                app.undo_history.push(FileOperation::Adjust {
                                    path: path.clone(),
                                    adjustments: app.adjustments.clone(),
                                    previous_adjustments: Box::new(prev_adj),
                                });
                                // Save adjustments to metadata database
                                app.metadata_db.set_adjustments(path.clone(), &app.adjustments);
                                app.metadata_db.save();
                                // Invalidate thumbnail to regenerate with new adjustments
                                app.thumbnail_textures.remove(&path);
                                app.thumbnail_requests.remove(&path);
                            }
                        }
                    }
                });
        });

        if app.adjustments.film.enabled {

            // Grain
            ui.label(RichText::new("Grain").size(11.0).color(LR_TEXT_LABEL));

            let mut grain_display = app.adjustments.film.grain_amount * 100.0;
            if lr_slider(ui, "Amount", &mut grain_display, 0.0..=100.0, "", 0.0) {
                app.adjustments.film.grain_amount = grain_display / 100.0;
                *adjustments_changed = true;
                if app.should_apply_adjustments() { app.refresh_adjustments(); }
            }

            let mut size_display = app.adjustments.film.grain_size * 50.0;
            if lr_slider(ui, "Size", &mut size_display, 25.0..=100.0, "", 50.0) {
                app.adjustments.film.grain_size = size_display / 50.0;
                *adjustments_changed = true;
                if app.should_apply_adjustments() { app.refresh_adjustments(); }
            }

            let mut rough_display = app.adjustments.film.grain_roughness * 100.0;
            if lr_slider(ui, "Roughness", &mut rough_display, 0.0..=100.0, "", 50.0) {
                app.adjustments.film.grain_roughness = rough_display / 100.0;
                *adjustments_changed = true;
                if app.should_apply_adjustments() { app.refresh_adjustments(); }
            }

            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);

            // Vignette
            ui.label(RichText::new("Vignette").size(11.0).color(LR_TEXT_LABEL));

            let mut vig_display = app.adjustments.film.vignette_amount * 100.0;
            if lr_slider(ui, "Amount", &mut vig_display, 0.0..=100.0, "", 0.0) {
                app.adjustments.film.vignette_amount = vig_display / 100.0;
                *adjustments_changed = true;
                if app.should_apply_adjustments() { app.refresh_adjustments(); }
            }

            let mut soft_display = (app.adjustments.film.vignette_softness - 0.5) / 1.5 * 100.0;
            if lr_slider(ui, "Feather", &mut soft_display, 0.0..=100.0, "", 33.33333333333333) {
                app.adjustments.film.vignette_softness = 0.5 + soft_display / 100.0 * 1.5;
                *adjustments_changed = true;
                if app.should_apply_adjustments() { app.refresh_adjustments(); }
            }

            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);

            // Halation
            ui.label(RichText::new("Halation").size(11.0).color(LR_TEXT_LABEL));

            let mut hal_display = app.adjustments.film.halation_amount * 100.0;
            if lr_slider(ui, "Amount", &mut hal_display, 0.0..=100.0, "", 0.0) {
                app.adjustments.film.halation_amount = hal_display / 100.0;
                *adjustments_changed = true;
                if app.should_apply_adjustments() { app.refresh_adjustments(); }
            }

            let mut rad_display = (app.adjustments.film.halation_radius - 0.5) / 2.5 * 100.0;
            if lr_slider(ui, "Radius", &mut rad_display, 0.0..=100.0, "", 20.0) {
                app.adjustments.film.halation_radius = 0.5 + rad_display / 100.0 * 2.5;
                *adjustments_changed = true;
                if app.should_apply_adjustments() { app.refresh_adjustments(); }
            }

            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);

            // Tone Curve
            ui.label(RichText::new("Tone Curve").size(11.0).color(LR_TEXT_LABEL));

            let mut shadows_tc = app.adjustments.film.tone_curve_shadows * 100.0;
            if lr_slider(ui, "Shadows", &mut shadows_tc, -100.0..=100.0, "", 0.0) {
                app.adjustments.film.tone_curve_shadows = shadows_tc / 100.0;
                *adjustments_changed = true;
                if app.should_apply_adjustments() { app.refresh_adjustments(); }
            }

            let mut mids_tc = app.adjustments.film.tone_curve_midtones * 100.0;
            if lr_slider(ui, "Midtones", &mut mids_tc, -100.0..=100.0, "", 0.0) {
                app.adjustments.film.tone_curve_midtones = mids_tc / 100.0;
                *adjustments_changed = true;
                if app.should_apply_adjustments() { app.refresh_adjustments(); }
            }

            let mut highs_tc = app.adjustments.film.tone_curve_highlights * 100.0;
            if lr_slider(ui, "Highlights", &mut highs_tc, -100.0..=100.0, "", 0.0) {
                app.adjustments.film.tone_curve_highlights = highs_tc / 100.0;
                *adjustments_changed = true;
                if app.should_apply_adjustments() { app.refresh_adjustments(); }
            }

            let mut scurve = app.adjustments.film.s_curve_strength * 100.0;
            if lr_slider(ui, "S-Curve", &mut scurve, 0.0..=100.0, "", 0.0) {
                app.adjustments.film.s_curve_strength = scurve / 100.0;
                *adjustments_changed = true;
                if app.should_apply_adjustments() { app.refresh_adjustments(); }
            }
        }
    });
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

    ui.painter().rect_filled(header_rect, Rounding::ZERO, Color32::from_rgb(45, 45, 45));
    ui.painter().hline(
        header_rect.x_range(),
        header_rect.bottom(),
        Stroke::new(1.0, LR_BORDER)
    );

    let response = egui::CollapsingHeader::new(RichText::new(title).size(11.0).color(Color32::from_rgb(200, 200, 200)).strong())
        .default_open(default_open)
        .show(ui, |ui| {
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(LR_BG_PANEL)
                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
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
fn lr_slider(ui: &mut egui::Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str, default: f32) -> bool {
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

        let response = ui.add(slider);
        if response.changed() {
            changed = true;
        }
        // Double-click resets to provided default value
        if response.double_clicked() {
            *value = default;
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