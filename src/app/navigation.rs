use std::path::PathBuf;
use super::ImageViewerApp;

impl ImageViewerApp {
    pub fn get_current_path(&self) -> Option<PathBuf> {
        self.filtered_list.get(self.current_index)
            .and_then(|&idx| self.image_list.get(idx))
            .cloned()
    }

    // Navigation
    pub fn next_image(&mut self) {
        if self.filtered_list.is_empty() {
            return;
        }

        let new_index = (self.current_index + 1) % self.filtered_list.len();
        self.navigate_to_index(new_index);
    }

    pub fn previous_image(&mut self) {
        if self.filtered_list.is_empty() {
            return;
        }

        let new_index = if self.current_index == 0 {
            self.filtered_list.len() - 1
        } else {
            self.current_index - 1
        };
        self.navigate_to_index(new_index);
    }

    pub fn go_to_first(&mut self) {
        if !self.filtered_list.is_empty() {
            self.navigate_to_index(0);
        }
    }

    pub fn go_to_last(&mut self) {
        if !self.filtered_list.is_empty() {
            self.navigate_to_index(self.filtered_list.len() - 1);
        }
    }

    fn navigate_to_index(&mut self, index: usize) {
        if index >= self.filtered_list.len() {
            return;
        }

        let saved_zoom = self.zoom;
        let saved_pan = self.pan_offset;

        self.current_index = index;
        self.load_current_image();

        if self.settings.maintain_zoom_on_navigate {
            self.zoom = saved_zoom;
            self.target_zoom = saved_zoom;
        }
        if self.settings.maintain_pan_on_navigate {
            self.pan_offset = saved_pan;
            self.target_pan = saved_pan;
        }
    }

    pub fn go_to_index(&mut self, index: usize) {
        self.navigate_to_index(index);
    }
}