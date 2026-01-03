use super::ImageViewerApp;
use crate::image_loader::ImageAdjustments;
use std::path::PathBuf;

impl ImageViewerApp {
    pub fn get_current_path(&self) -> Option<PathBuf> {
        self.filtered_list
            .get(self.current_index)
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

        // Save current adjustments before navigating
        self.save_current_adjustments();

        let saved_zoom = self.zoom;
        let saved_pan = self.pan_offset;

        self.current_index = index;

        // Load adjustments for the new image
        self.load_adjustments_for_current();

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

    /// Save current image's adjustments to metadata database
    pub fn save_current_adjustments(&mut self) {
        if let Some(path) = self.get_current_path() {
            self.metadata_db.set_adjustments(path, &self.adjustments);
            self.metadata_db.save();
        }
    }

    /// Load adjustments for the current image from metadata database
    pub fn load_adjustments_for_current(&mut self) {
        if let Some(path) = self.get_current_path() {
            if let Some(adjustments) = self.metadata_db.get_adjustments(&path) {
                self.adjustments = adjustments;
                // Also update the film preset if film is enabled
                self.current_film_preset = crate::image_loader::FilmPreset::None;
            } else {
                // Reset to default if no adjustments stored
                self.adjustments = ImageAdjustments::default();
                self.current_film_preset = crate::image_loader::FilmPreset::None;
            }
        }
    }
}
