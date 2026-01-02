use crate::app::ImageViewerApp;
use egui;

impl ImageViewerApp {
    pub fn render_dialogs(&mut self, ctx: &egui::Context) {
        self.render_settings_dialog(ctx);
        self.render_go_to_dialog(ctx);
        self.render_move_dialog(ctx);
        self.render_command_palette(ctx);
        self.render_catalog_import_dialog(ctx);
        self.render_new_collection_dialog(ctx);
    }
}
