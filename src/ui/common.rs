use egui::{self, Color32, CornerRadius, Rect, RichText, Stroke, Vec2};

// Common color constants used across UI panels
pub const LR_BG_PANEL: Color32 = Color32::from_rgb(51, 51, 51);
pub const LR_BG_INPUT: Color32 = Color32::from_rgb(34, 34, 34);
pub const LR_BORDER: Color32 = Color32::from_rgb(28, 28, 28);
pub const LR_TEXT_PRIMARY: Color32 = Color32::from_rgb(200, 200, 200);
pub const LR_TEXT_LABEL: Color32 = Color32::from_rgb(180, 180, 180);
pub const LR_TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 140);
pub const LR_HEADER_BG: Color32 = Color32::from_rgb(45, 45, 45);

/// Common collapsible panel widget used across UI panels
pub fn lr_collapsible_panel<R>(
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
        LR_HEADER_BG,
    );
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

/// Common separator widget
pub fn lr_separator(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        rect.x_range(),
        rect.top(),
        Stroke::new(1.0, Color32::from_rgb(60, 60, 60)),
    );
}

/// Common slider widget with label and value display
pub fn lr_slider_ex(
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

/// Simplified slider wrapper that returns only the changed flag
pub fn lr_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
    default: f32,
) -> bool {
    lr_slider_ex(ui, label, value, range, suffix, default).0
}

/// Common info row widget for metadata display
pub fn lr_info_row(ui: &mut egui::Ui, label: &str, value: Option<&str>) {
    if let Some(v) = value {
        if !v.is_empty() && v != "Unknown" {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{}:", label))
                        .size(10.0)
                        .color(LR_TEXT_SECONDARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(v).size(10.0).color(LR_TEXT_PRIMARY));
                });
            });
        }
    }
}