use crate::app::ImageViewerApp;

impl ImageViewerApp {
    pub fn render_dialogs(&mut self, ctx: &egui::Context) {
        self.render_settings_dialog(ctx);
        self.render_command_palette(ctx);
    }
}
