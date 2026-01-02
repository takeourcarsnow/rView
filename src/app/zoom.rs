use eframe::egui::Vec2;
use super::ImageViewerApp;

impl ImageViewerApp {
    pub fn reset_view(&mut self) {
        // Reset to 100% zoom and center the image
        self.target_zoom = 1.0;
        self.zoom = 1.0;
        self.pan_offset = Vec2::ZERO;
        self.target_pan = Vec2::ZERO;
    }

    pub fn fit_to_window_internal(&mut self) {
        if let Some(texture) = &self.current_texture {
            let image_size = texture.size_vec2();
            // Use the actual available view size from the UI
            let available = self.available_view_size;

            let scale_x = available.x / image_size.x;
            let scale_y = available.y / image_size.y;
            self.target_zoom = scale_x.min(scale_y).min(1.0);

            if !self.settings.smooth_zoom {
                self.zoom = self.target_zoom;
            }
        } else {
            self.target_zoom = 1.0;
            self.zoom = 1.0;
        }

        self.target_pan = Vec2::ZERO;
        if !self.settings.smooth_zoom {
            self.pan_offset = Vec2::ZERO;
        }
    }

    // Zoom
    pub fn zoom_in(&mut self) {
        self.set_zoom(self.target_zoom * 1.25);
    }

    pub fn zoom_out(&mut self) {
        self.set_zoom(self.target_zoom / 1.25);
    }

    pub fn zoom_to(&mut self, level: f32) {
        self.set_zoom(level.clamp(0.05, 32.0));
    }

    fn set_zoom(&mut self, target: f32) {
        self.target_zoom = target;
        if !self.settings.smooth_zoom {
            self.zoom = self.target_zoom;
        }
    }

    pub fn fit_to_window(&mut self) {
        if let Some(texture) = &self.current_texture {
            let image_size = texture.size_vec2();
            let available = self.available_view_size;

            let scale_x = available.x / image_size.x;
            let scale_y = available.y / image_size.y;
            self.target_zoom = scale_x.min(scale_y).min(1.0);

            if !self.settings.smooth_zoom {
                self.zoom = self.target_zoom;
            }
        }
        self.target_pan = Vec2::ZERO;
        self.pan_offset = Vec2::ZERO;
    }

    pub fn fill_window(&mut self) {
        if let Some(texture) = &self.current_texture {
            let image_size = texture.size_vec2();
            let available = self.available_view_size;

            let scale_x = available.x / image_size.x;
            let scale_y = available.y / image_size.y;
            self.target_zoom = scale_x.max(scale_y);

            if !self.settings.smooth_zoom {
                self.zoom = self.target_zoom;
            }
        }
        self.target_pan = Vec2::ZERO;
        self.pan_offset = Vec2::ZERO;
    }
}