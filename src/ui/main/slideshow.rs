use crate::app::ImageViewerApp;

impl ImageViewerApp {
    pub fn update_slideshow(&mut self, ctx: &egui::Context) {
        if self.slideshow_active {
            self.slideshow_timer += ctx.input(|i| i.stable_dt);

            if self.slideshow_timer >= self.settings.slideshow_interval {
                self.slideshow_timer = 0.0;

                if self.settings.slideshow_loop || self.current_index < self.filtered_list.len() - 1 {
                    self.next_image();
                } else {
                    self.slideshow_active = false;
                }
            }

            ctx.request_repaint();
        }
    }
}