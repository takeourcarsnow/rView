use egui::{Color32, RichText, Vec2, Rounding, Stroke, Rect};

// Lightroom-style collapsible panel
pub fn lr_collapsible_panel<R>(
    ui: &mut egui::Ui,
    title: &str,
    default_open: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::CollapsingResponse<R> {
    // Lightroom-inspired color scheme
    const LR_BG_PANEL: Color32 = Color32::from_rgb(51, 51, 51);
    const LR_BORDER: Color32 = Color32::from_rgb(28, 28, 28);
    const LR_TEXT_PRIMARY: Color32 = Color32::from_rgb(200, 200, 200);
    const LR_HEADER_BG: Color32 = Color32::from_rgb(45, 45, 45);

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
pub fn lr_separator(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        rect.x_range(),
        rect.top(),
        Stroke::new(1.0, Color32::from_rgb(60, 60, 60))
    );
    ui.add_space(1.0);
}

// Lightroom-style slider with label on left, value on right
pub fn lr_slider(ui: &mut egui::Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str, default: f32) -> bool {
    // Lightroom-inspired color scheme
    const LR_TEXT_LABEL: Color32 = Color32::from_rgb(180, 180, 180);
    const LR_TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 140);

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

// Lightroom-style info row (label: value)
pub fn lr_info_row(ui: &mut egui::Ui, label: &str, value: Option<&str>) {
    // Lightroom-inspired color scheme
    const LR_TEXT_PRIMARY: Color32 = Color32::from_rgb(200, 200, 200);
    const LR_TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 140);

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