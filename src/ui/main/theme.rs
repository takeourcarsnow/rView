use egui::Color32;

pub fn apply_theme(ctx: &egui::Context, settings: &crate::settings::Settings) {
    let mut visuals = match settings.theme {
        crate::settings::Theme::Dark => egui::Visuals::dark(),
        crate::settings::Theme::Light => egui::Visuals::light(),
        crate::settings::Theme::Oled => {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::BLACK;
            visuals.window_fill = Color32::BLACK;
            visuals.extreme_bg_color = Color32::BLACK;
            visuals
        }
        crate::settings::Theme::System => egui::Visuals::dark(),
        crate::settings::Theme::SolarizedDark => {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(0, 43, 54);
            visuals.window_fill = Color32::from_rgb(7, 54, 66);
            visuals.widgets.inactive.bg_fill = Color32::from_rgb(7, 54, 66);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(88, 110, 117);
            visuals.widgets.active.bg_fill = Color32::from_rgb(38, 139, 210);
            visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(131, 148, 150);
            visuals
        }
        crate::settings::Theme::SolarizedLight => {
            let mut visuals = egui::Visuals::light();
            visuals.panel_fill = Color32::from_rgb(238, 232, 213);
            visuals.window_fill = Color32::from_rgb(253, 246, 227);
            visuals.widgets.inactive.bg_fill = Color32::from_rgb(253, 246, 227);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(238, 232, 213);
            visuals.widgets.active.bg_fill = Color32::from_rgb(133, 153, 0);
            visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(101, 123, 131);
            visuals
        }
        crate::settings::Theme::HighContrast => {
            let mut visuals = egui::Visuals::dark();
            visuals.widgets.inactive.fg_stroke.color = Color32::WHITE;
            visuals.widgets.hovered.fg_stroke.color = Color32::YELLOW;
            visuals.widgets.active.fg_stroke.color = Color32::from_rgb(255, 255, 0);
            visuals.widgets.inactive.bg_stroke.color = Color32::WHITE;
            visuals.widgets.hovered.bg_stroke.color = Color32::YELLOW;
            visuals.widgets.active.bg_stroke.color = Color32::from_rgb(255, 255, 0);
            visuals.widgets.inactive.bg_stroke.width = 2.0;
            visuals.widgets.hovered.bg_stroke.width = 3.0;
            visuals.widgets.active.bg_stroke.width = 4.0;
            visuals
        }
        crate::settings::Theme::Blue => {
            let mut visuals = egui::Visuals::dark();
            visuals.widgets.active.bg_fill = Color32::from_rgb(70, 130, 255);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(100, 150, 255);
            visuals.widgets.active.fg_stroke.color = Color32::WHITE;
            visuals
        }
        crate::settings::Theme::Purple => {
            let mut visuals = egui::Visuals::dark();
            visuals.widgets.active.bg_fill = Color32::from_rgb(160, 90, 255);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(180, 110, 255);
            visuals.widgets.active.fg_stroke.color = Color32::WHITE;
            visuals
        }
        crate::settings::Theme::Green => {
            let mut visuals = egui::Visuals::dark();
            visuals.widgets.active.bg_fill = Color32::from_rgb(50, 205, 100);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(70, 225, 120);
            visuals.widgets.active.fg_stroke.color = Color32::WHITE;
            visuals
        }
        crate::settings::Theme::Warm => {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(30, 25, 20);
            visuals.widgets.active.bg_fill = Color32::from_rgb(255, 150, 50);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(255, 170, 70);
            visuals.widgets.active.fg_stroke.color = Color32::from_rgb(30, 25, 20);
            visuals
        }
        crate::settings::Theme::Cool => {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(20, 25, 35);
            visuals.widgets.active.bg_fill = Color32::from_rgb(50, 200, 220);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(70, 220, 240);
            visuals.widgets.active.fg_stroke.color = Color32::from_rgb(20, 25, 35);
            visuals
        }
    };

    // Apply accent color to active elements
    visuals.widgets.active.bg_fill = settings.accent_color.to_color();
    visuals.selection.bg_fill = settings.accent_color.to_color().linear_multiply(0.5);

    ctx.set_visuals(visuals);
}
