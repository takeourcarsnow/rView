use crate::app::ImageViewerApp;
use crate::settings::ColorLabel;
use egui::{self, Color32, CornerRadius, Rect, RichText, Stroke, StrokeKind, Vec2};

// Lightroom-inspired color scheme
const LR_BG_PANEL: Color32 = Color32::from_rgb(51, 51, 51);
const LR_BORDER: Color32 = Color32::from_rgb(28, 28, 28);
const LR_TEXT_LABEL: Color32 = Color32::from_rgb(180, 180, 180);
const LR_TEXT_PRIMARY: Color32 = Color32::from_rgb(200, 200, 200);
const LR_TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 140);
const LR_HEADER_BG: Color32 = Color32::from_rgb(45, 45, 45);

pub fn render_keywording_panel(app: &mut ImageViewerApp, ui: &mut egui::Ui) {
    lr_collapsible_panel(ui, "Keywording", true, |ui| {
        if let Some(path) = app.get_current_path() {
            let metadata = app.metadata_db.get(&path);

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

                    if ui
                        .add(
                            egui::Button::new(RichText::new(star).size(14.0).color(color))
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .min_size(Vec2::new(16.0, 16.0)),
                        )
                        .clicked()
                    {
                        let new_rating = if metadata.rating == i { 0 } else { i };
                        app.set_rating(new_rating);
                    }
                }
            });

            ui.add_space(4.0);

            // Color labels (like Lightroom)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Label").size(10.0).color(LR_TEXT_LABEL));
                ui.add_space(24.0);

                for label in [
                    ColorLabel::None,
                    ColorLabel::Red,
                    ColorLabel::Yellow,
                    ColorLabel::Green,
                    ColorLabel::Blue,
                    ColorLabel::Purple,
                ] {
                    let is_selected = metadata.color_label == label;
                    let color = label.to_color();

                    let size = 14.0;
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click());

                    if label == ColorLabel::None {
                        ui.painter().rect_stroke(
                            rect.shrink(1.0),
                            CornerRadius::same(2),
                            Stroke::new(1.0, LR_TEXT_SECONDARY),
                            StrokeKind::Inside,
                        );
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
                        ui.painter()
                            .rect_filled(rect.shrink(1.0), CornerRadius::same(2), color);
                        if is_selected {
                            ui.painter().rect_stroke(
                                rect,
                                CornerRadius::same(3),
                                Stroke::new(2.0, Color32::WHITE),
                                StrokeKind::Inside,
                            );
                        }
                    }

                    if response.clicked() {
                        app.set_color_label(label);
                    }
                }
            });

            ui.add_space(8.0);

            // Keywords/Tags
            ui.label(RichText::new("Keywords").size(10.0).color(LR_TEXT_LABEL));
            ui.add_space(2.0);

            if metadata.tags.is_empty() {
                ui.label(
                    RichText::new("No keywords")
                        .size(10.0)
                        .color(LR_TEXT_SECONDARY)
                        .italics(),
                );
            } else {
                ui.horizontal_wrapped(|ui| {
                    for tag in &metadata.tags {
                        ui.add(
                            egui::Button::new(
                                RichText::new(tag.as_str()).size(9.0).color(LR_TEXT_PRIMARY),
                            )
                            .fill(Color32::from_rgb(34, 34, 34))
                            .stroke(Stroke::new(1.0, LR_BORDER))
                            .corner_radius(CornerRadius::same(8)),
                        );
                    }
                });
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
    let header_rect = Rect::from_min_size(header_rect.min, Vec2::new(ui.available_width(), 24.0));

    ui.painter()
        .rect_filled(header_rect, CornerRadius::ZERO, LR_HEADER_BG);
    ui.painter().hline(
        header_rect.x_range(),
        header_rect.bottom(),
        Stroke::new(1.0, LR_BORDER),
    );

    let response = egui::CollapsingHeader::new(
        RichText::new(title)
            .size(11.0)
            .color(LR_TEXT_PRIMARY)
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
