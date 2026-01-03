use crate::app::ImageViewerApp;

impl ImageViewerApp {
    pub fn animate_view(&mut self, ctx: &egui::Context) {
        if self.settings.smooth_zoom {
            let dt = ctx.input(|i| i.stable_dt);
            let speed = self.settings.zoom_animation_speed;

            // Smooth zoom
            if (self.zoom - self.target_zoom).abs() > 0.001 {
                self.zoom += (self.target_zoom - self.zoom) * speed * dt;
                ctx.request_repaint();
            } else {
                self.zoom = self.target_zoom;
            }

            // Smooth pan
            let pan_diff = self.target_pan - self.pan_offset;
            if pan_diff.length() > 0.1 {
                self.pan_offset += pan_diff * speed * dt;
                ctx.request_repaint();
            } else {
                self.pan_offset = self.target_pan;
            }
        }
    }
}
