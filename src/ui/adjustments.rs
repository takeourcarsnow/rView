use crate::app::ImageViewerApp;
use crate::image_loader::{FilmPreset, ImageAdjustments};
use crate::metadata::FileOperation;
use egui::{self, Color32, CornerRadius, Rect, RichText, Stroke, Vec2};

const LR_BG_INPUT: Color32 = Color32::from_rgb(34, 34, 34);
const LR_BG_PANEL: Color32 = Color32::from_rgb(51, 51, 51);
const LR_BORDER: Color32 = Color32::from_rgb(28, 28, 28);
const LR_TEXT_LABEL: Color32 = Color32::from_rgb(180, 180, 180);
const LR_TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 140);

pub fn render_basic_panel(app: &mut ImageViewerApp, ui: &mut egui::Ui) {
    let was_dragging = app.slider_dragging;
    let mut adjustments_changed = false;
    let mut any_slider_dragging = false;

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
        let (changed, dragging) = lr_slider_ex(
            ui,
            "Temp",
            &mut app.adjustments.temperature,
            -1.0..=1.0,
            "",
            0.0,
        );
        if changed {
            adjustments_changed = true;
            app.mark_adjustments_dirty();
        }
        any_slider_dragging |= dragging;

        ui.add_space(4.0);
        lr_separator(ui);
        ui.add_space(4.0);

        // Tone section header
        ui.label(RichText::new("Tone").size(11.0).color(LR_TEXT_LABEL));
        ui.add_space(4.0);

        // Exposure
        let (changed, dragging) = lr_slider_ex(
            ui,
            "Exposure",
            &mut app.adjustments.exposure,
            -3.0..=3.0,
            " EV",
            0.0,
        );
        if changed {
            adjustments_changed = true;
            app.mark_adjustments_dirty();
        }
        any_slider_dragging |= dragging;

        ui.add_space(4.0);
        lr_separator(ui);
        ui.add_space(4.0);

        // Presence section
        ui.label(RichText::new("Presence").size(11.0).color(LR_TEXT_LABEL));
        ui.add_space(4.0);

        // Saturation (convert from 0-2 to -100 to +100)
        let mut sat_display = (app.adjustments.saturation - 1.0) * 100.0;
        let (changed, dragging) =
            lr_slider_ex(ui, "Saturation", &mut sat_display, -100.0..=100.0, "", 0.0);
        if changed {
            app.adjustments.saturation = 1.0 + sat_display / 100.0;
            adjustments_changed = true;
            app.mark_adjustments_dirty();
        }
        any_slider_dragging |= dragging;

        ui.add_space(8.0);

        // Reset button
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("Reset").size(10.0).color(LR_TEXT_SECONDARY))
                        .fill(LR_BG_INPUT)
                        .stroke(Stroke::new(1.0, LR_BORDER))
                        .corner_radius(CornerRadius::same(2)),
                )
                .clicked()
            {
                let prev = app
                    .pre_drag_adjustments
                    .take()
                    .unwrap_or_else(|| app.adjustments.clone());
                app.adjustments = ImageAdjustments::default();
                app.current_film_preset = FilmPreset::None;
                app.refresh_adjustments();
                if let Some(path) = app.get_current_path() {
                    app.undo_history.push(FileOperation::Adjust {
                        path: path.clone(),
                        adjustments: app.adjustments.clone(),
                        previous_adjustments: Box::new(prev),
                    });
                    // Save reset adjustments to metadata database
                    app.metadata_db
                        .set_adjustments(path.clone(), &app.adjustments);
                    app.metadata_db.save();
                    // Invalidate thumbnail to regenerate with new adjustments
                    app.thumbnail_textures.remove(&path);
                    app.thumbnail_requests.remove(&path);
                }
            }
        });
    });

    // Update slider dragging state
    app.slider_dragging = any_slider_dragging;

    // Capture pre-drag adjustments when drag starts
    if any_slider_dragging && !was_dragging {
        app.pre_drag_adjustments = Some(app.adjustments.clone());
    }

    render_film_emulation_panel(app, ui, &mut adjustments_changed);

    // When drag ends, finalize: save undo, metadata, and invalidate thumbnail
    if was_dragging && !any_slider_dragging {
        if let Some(pre_drag) = app.pre_drag_adjustments.take() {
            if app.adjustments != pre_drag {
                if let Some(path) = app.get_current_path() {
                    app.undo_history.push(FileOperation::Adjust {
                        path: path.clone(),
                        adjustments: app.adjustments.clone(),
                        previous_adjustments: Box::new(pre_drag),
                    });
                    // Save adjustments to metadata database
                    app.metadata_db
                        .set_adjustments(path.clone(), &app.adjustments);
                    app.metadata_db.save();
                    // Invalidate thumbnail to regenerate with new adjustments
                    app.thumbnail_textures.remove(&path);
                    app.thumbnail_requests.remove(&path);
                    // Do a full refresh now that drag ended (for histogram/overlays)
                    app.refresh_adjustments();

                    // Quick profiler summary after drag completes
                    crate::profiler::with_profiler(|p| {
                        let stats = p.get_stats();
                        let measure = |name: &str| {
                            stats
                                .measurements
                                .get(name)
                                .map(|m| m.average_time.as_millis())
                                .unwrap_or(0)
                        };
                        log::info!(
                            "Perf summary (ms avg): apply_adjustments_fast={}, set_current_image_fast_total={}, refresh_internal_fast={}, refresh_if_dirty={}",
                            measure("apply_adjustments_fast"),
                            measure("set_current_image_fast_total"),
                            measure("refresh_adjustments_internal_fast"),
                            measure("refresh_adjustments_if_dirty")
                        );
                        p.reset();
                    });
                }
            }
        }
    }
}

pub fn render_film_emulation_panel(
    app: &mut ImageViewerApp,
    ui: &mut egui::Ui,
    adjustments_changed: &mut bool,
) {
    lr_collapsible_panel(ui, "Film Emulation", false, |ui| {
        ui.spacing_mut().slider_width = ui.available_width() - 80.0;

        ui.horizontal(|ui| {
            ui.label(RichText::new("Profile:").size(11.0).color(LR_TEXT_LABEL));
            ui.add_space(8.0);
            egui::ComboBox::from_id_salt("film_preset")
                .width(ui.available_width() - 8.0)
                .selected_text(app.current_film_preset.name())
                .show_ui(ui, |ui| {
                    for preset in FilmPreset::all() {
                        let selected = *preset == app.current_film_preset;
                        let response = ui
                            .selectable_label(selected, preset.name())
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
                                app.metadata_db
                                    .set_adjustments(path.clone(), &app.adjustments);
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

            let mut grain_display = app.adjustments.film.grain.amount * 100.0;
            if lr_slider(ui, "Amount", &mut grain_display, 0.0..=100.0, "", 0.0) {
                app.adjustments.film.grain.amount = grain_display / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut size_display = app.adjustments.film.grain.size * 50.0;
            if lr_slider(ui, "Size", &mut size_display, 25.0..=100.0, "", 50.0) {
                app.adjustments.film.grain.size = size_display / 50.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut rough_display = app.adjustments.film.grain.roughness * 100.0;
            if lr_slider(ui, "Roughness", &mut rough_display, 0.0..=100.0, "", 50.0) {
                app.adjustments.film.grain.roughness = rough_display / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);

            // Vignette
            ui.label(RichText::new("Vignette").size(11.0).color(LR_TEXT_LABEL));

            let mut vig_display = app.adjustments.film.vignette.amount * 100.0;
            if lr_slider(ui, "Amount", &mut vig_display, 0.0..=100.0, "", 0.0) {
                app.adjustments.film.vignette.amount = vig_display / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut soft_display = (app.adjustments.film.vignette.softness - 0.5) / 1.5 * 100.0;
            if lr_slider(
                ui,
                "Feather",
                &mut soft_display,
                0.0..=100.0,
                "",
                33.333_332,
            ) {
                app.adjustments.film.vignette.softness = 0.5 + soft_display / 100.0 * 1.5;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);

            // Halation
            ui.label(RichText::new("Halation").size(11.0).color(LR_TEXT_LABEL));

            let mut hal_display = app.adjustments.film.halation.amount * 100.0;
            if lr_slider(ui, "Amount", &mut hal_display, 0.0..=100.0, "", 0.0) {
                app.adjustments.film.halation.amount = hal_display / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut rad_display = (app.adjustments.film.halation.radius - 0.5) / 2.5 * 100.0;
            if lr_slider(ui, "Radius", &mut rad_display, 0.0..=100.0, "", 20.0) {
                app.adjustments.film.halation.radius = 0.5 + rad_display / 100.0 * 2.5;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);

            // Tone Curve
            ui.label(RichText::new("Tone Curve").size(11.0).color(LR_TEXT_LABEL));

            let mut shadows_tc = app.adjustments.film.tone.shadows * 100.0;
            if lr_slider(ui, "Shadows", &mut shadows_tc, -100.0..=100.0, "", 0.0) {
                app.adjustments.film.tone.shadows = shadows_tc / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut mids_tc = app.adjustments.film.tone.midtones * 100.0;
            if lr_slider(ui, "Midtones", &mut mids_tc, -100.0..=100.0, "", 0.0) {
                app.adjustments.film.tone.midtones = mids_tc / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut highs_tc = app.adjustments.film.tone.highlights * 100.0;
            if lr_slider(ui, "Highlights", &mut highs_tc, -100.0..=100.0, "", 0.0) {
                app.adjustments.film.tone.highlights = highs_tc / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut scurve = app.adjustments.film.tone.s_curve_strength * 100.0;
            if lr_slider(ui, "S-Curve", &mut scurve, 0.0..=100.0, "", 0.0) {
                app.adjustments.film.tone.s_curve_strength = scurve / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);

            // Color Science (Advanced)
            ui.label(RichText::new("Color Science").size(11.0).color(LR_TEXT_LABEL));

            // Color Crossover
            ui.label(RichText::new("Crossover").size(10.0).color(LR_TEXT_SECONDARY));
            ui.add_space(2.0);

            let mut rig = app.adjustments.film.color_crossover.red_in_green * 100.0;
            if lr_slider(ui, "R→G", &mut rig, -20.0..=20.0, "", 0.0) {
                app.adjustments.film.color_crossover.red_in_green = rig / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut rib = app.adjustments.film.color_crossover.red_in_blue * 100.0;
            if lr_slider(ui, "R→B", &mut rib, -20.0..=20.0, "", 0.0) {
                app.adjustments.film.color_crossover.red_in_blue = rib / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut gir = app.adjustments.film.color_crossover.green_in_red * 100.0;
            if lr_slider(ui, "G→R", &mut gir, -20.0..=20.0, "", 0.0) {
                app.adjustments.film.color_crossover.green_in_red = gir / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut gib = app.adjustments.film.color_crossover.green_in_blue * 100.0;
            if lr_slider(ui, "G→B", &mut gib, -20.0..=20.0, "", 0.0) {
                app.adjustments.film.color_crossover.green_in_blue = gib / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut bir = app.adjustments.film.color_crossover.blue_in_red * 100.0;
            if lr_slider(ui, "B→R", &mut bir, -20.0..=20.0, "", 0.0) {
                app.adjustments.film.color_crossover.blue_in_red = bir / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut big = app.adjustments.film.color_crossover.blue_in_green * 100.0;
            if lr_slider(ui, "B→G", &mut big, -20.0..=20.0, "", 0.0) {
                app.adjustments.film.color_crossover.blue_in_green = big / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            // Color Gamma
            ui.add_space(4.0);
            ui.label(RichText::new("Gamma").size(10.0).color(LR_TEXT_SECONDARY));
            ui.add_space(2.0);

            let mut r_gamma = (app.adjustments.film.color_gamma.red - 0.8) / 0.4 * 100.0;
            if lr_slider(ui, "R Gamma", &mut r_gamma, 0.0..=100.0, "", 50.0) {
                app.adjustments.film.color_gamma.red = 0.8 + r_gamma / 100.0 * 0.4;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut g_gamma = (app.adjustments.film.color_gamma.green - 0.8) / 0.4 * 100.0;
            if lr_slider(ui, "G Gamma", &mut g_gamma, 0.0..=100.0, "", 50.0) {
                app.adjustments.film.color_gamma.green = 0.8 + g_gamma / 100.0 * 0.4;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut b_gamma = (app.adjustments.film.color_gamma.blue - 0.8) / 0.4 * 100.0;
            if lr_slider(ui, "B Gamma", &mut b_gamma, 0.0..=100.0, "", 50.0) {
                app.adjustments.film.color_gamma.blue = 0.8 + b_gamma / 100.0 * 0.4;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);

            // Tone & Dynamic Range
            ui.label(RichText::new("Tone & Dynamic Range").size(11.0).color(LR_TEXT_LABEL));

            let mut bp_display = app.adjustments.film.black_point * 1000.0;
            if lr_slider(ui, "Black Point", &mut bp_display, 0.0..=100.0, "", 0.0) {
                app.adjustments.film.black_point = bp_display / 1000.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut wp_display = (app.adjustments.film.white_point - 0.9) / 0.1 * 100.0;
            if lr_slider(ui, "White Point", &mut wp_display, 0.0..=100.0, "", 100.0) {
                app.adjustments.film.white_point = 0.9 + wp_display / 100.0 * 0.1;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut lat_display = app.adjustments.film.latitude * 100.0;
            if lr_slider(ui, "Latitude", &mut lat_display, 0.0..=100.0, "", 0.0) {
                app.adjustments.film.latitude = lat_display / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);

            // Color Grading
            ui.label(RichText::new("Color Grading").size(11.0).color(LR_TEXT_LABEL));

            // Shadow Tint
            ui.label(RichText::new("Shadow Tint").size(10.0).color(LR_TEXT_SECONDARY));
            ui.add_space(2.0);

            let mut shadow_r = app.adjustments.film.shadow_tint[0] * 100.0;
            if lr_slider(ui, "Shadow R", &mut shadow_r, -100.0..=100.0, "", 0.0) {
                app.adjustments.film.shadow_tint[0] = shadow_r / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut shadow_g = app.adjustments.film.shadow_tint[1] * 100.0;
            if lr_slider(ui, "Shadow G", &mut shadow_g, -100.0..=100.0, "", 0.0) {
                app.adjustments.film.shadow_tint[1] = shadow_g / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut shadow_b = app.adjustments.film.shadow_tint[2] * 100.0;
            if lr_slider(ui, "Shadow B", &mut shadow_b, -100.0..=100.0, "", 0.0) {
                app.adjustments.film.shadow_tint[2] = shadow_b / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            // Highlight Tint
            ui.add_space(4.0);
            ui.label(RichText::new("Highlight Tint").size(10.0).color(LR_TEXT_SECONDARY));
            ui.add_space(2.0);

            let mut highlight_r = app.adjustments.film.highlight_tint[0] * 100.0;
            if lr_slider(ui, "High R", &mut highlight_r, -100.0..=100.0, "", 0.0) {
                app.adjustments.film.highlight_tint[0] = highlight_r / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut highlight_g = app.adjustments.film.highlight_tint[1] * 100.0;
            if lr_slider(ui, "High G", &mut highlight_g, -100.0..=100.0, "", 0.0) {
                app.adjustments.film.highlight_tint[1] = highlight_g / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            let mut highlight_b = app.adjustments.film.highlight_tint[2] * 100.0;
            if lr_slider(ui, "High B", &mut highlight_b, -100.0..=100.0, "", 0.0) {
                app.adjustments.film.highlight_tint[2] = highlight_b / 100.0;
                *adjustments_changed = true;
                app.mark_adjustments_dirty();
            }

            ui.add_space(4.0);
            lr_separator(ui);
            ui.add_space(4.0);

            // Processing
            ui.label(RichText::new("Processing").size(11.0).color(LR_TEXT_LABEL));

            ui.horizontal(|ui| {
                ui.label(RichText::new("B&W Film").size(10.0).color(LR_TEXT_LABEL));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.checkbox(&mut app.adjustments.film.is_bw, "").changed() {
                        *adjustments_changed = true;
                        app.mark_adjustments_dirty();
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new("Enable Frame").size(10.0).color(LR_TEXT_LABEL));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.checkbox(&mut app.adjustments.frame_enabled, "").changed() {
                        *adjustments_changed = true;
                        app.mark_adjustments_dirty();
                    }
                });
            });

            if app.adjustments.frame_enabled {
                // Frame Thickness
                let mut thickness_display = app.adjustments.frame_thickness;
                if lr_slider(ui, "Frame Thickness", &mut thickness_display, 0.0..=200.0, "", 80.0) {
                    app.adjustments.frame_thickness = thickness_display;
                    *adjustments_changed = true;
                    app.mark_adjustments_dirty();
                }

                // Frame Color (RGB sliders)
                ui.label(RichText::new("Frame Color").size(10.0).color(LR_TEXT_SECONDARY));
                ui.add_space(2.0);

                let mut r = (app.adjustments.frame_color[0] * 255.0) as u8;
                let mut g = (app.adjustments.frame_color[1] * 255.0) as u8;
                let mut b = (app.adjustments.frame_color[2] * 255.0) as u8;

                ui.horizontal(|ui| {
                    ui.label(RichText::new("R").size(10.0).color(LR_TEXT_LABEL));
                    if ui.add(egui::Slider::new(&mut r, 0..=255).show_value(false)).changed() {
                        app.adjustments.frame_color[0] = r as f32 / 255.0;
                        *adjustments_changed = true;
                        app.mark_adjustments_dirty();
                    }
                    ui.label(RichText::new("G").size(10.0).color(LR_TEXT_LABEL));
                    if ui.add(egui::Slider::new(&mut g, 0..=255).show_value(false)).changed() {
                        app.adjustments.frame_color[1] = g as f32 / 255.0;
                        *adjustments_changed = true;
                        app.mark_adjustments_dirty();
                    }
                    ui.label(RichText::new("B").size(10.0).color(LR_TEXT_LABEL));
                    if ui.add(egui::Slider::new(&mut b, 0..=255).show_value(false)).changed() {
                        app.adjustments.frame_color[2] = b as f32 / 255.0;
                        *adjustments_changed = true;
                        app.mark_adjustments_dirty();
                    }
                });
            }
        }
    });
}

fn lr_collapsible_panel<R>(
    ui: &mut egui::Ui,
    title: &str,
    default_open: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::CollapsingResponse<R> {
    // Panel header background
    let header_rect = ui.available_rect_before_wrap();
    let header_rect = Rect::from_min_size(header_rect.min, Vec2::new(ui.available_width(), 24.0));

    ui.painter().rect_filled(
        header_rect,
        CornerRadius::ZERO,
        Color32::from_rgb(45, 45, 45),
    );
    ui.painter().hline(
        header_rect.x_range(),
        header_rect.bottom(),
        Stroke::new(1.0, LR_BORDER),
    );

    let response = egui::CollapsingHeader::new(
        RichText::new(title)
            .size(11.0)
            .color(Color32::from_rgb(200, 200, 200))
            .strong(),
    )
    .default_open(default_open)
    .show(ui, |ui| {
        ui.add_space(4.0);
        egui::Frame::NONE
            .fill(LR_BG_PANEL)
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| add_contents(ui))
            .inner
    });

    // Bottom border
    ui.painter().hline(
        ui.available_rect_before_wrap().x_range(),
        ui.cursor().top(),
        Stroke::new(1.0, LR_BORDER),
    );

    response
}

fn lr_separator(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        rect.x_range(),
        rect.top(),
        Stroke::new(1.0, Color32::from_rgb(60, 60, 60)),
    );
    ui.add_space(1.0);
}

fn lr_slider_ex(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
    default: f32,
) -> (bool, bool) {
    let mut changed = false;
    let mut is_dragging = false;

    ui.horizontal(|ui| {
        // Fixed-width label column
        ui.allocate_ui_with_layout(
            Vec2::new(70.0, 18.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.label(RichText::new(label).size(10.0).color(LR_TEXT_LABEL));
            },
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
        // Track if user is actively dragging
        is_dragging = response.dragged();

        // Double-click on slider area resets to default value
        // Check for double-click via secondary sense since slider may consume it
        let double_click = response.double_clicked()
            || (ui.input(|i| {
                i.pointer
                    .button_double_clicked(egui::PointerButton::Primary)
            }) && response.hovered());
        if double_click {
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
            ui.label(
                RichText::new(display_val)
                    .size(10.0)
                    .color(LR_TEXT_SECONDARY)
                    .monospace(),
            );
        });
    });

    (changed, is_dragging)
}

// Wrapper that maintains backward compatibility
fn lr_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
    default: f32,
) -> bool {
    lr_slider_ex(ui, label, value, range, suffix, default).0
}
