use super::ImageViewerApp;
use std::path::Path;

fn compare_paths_by_mode(
    a: &Path,
    b: &Path,
    sort_mode: crate::settings::SortMode,
) -> std::cmp::Ordering {
    match sort_mode {
        crate::settings::SortMode::Name => {
            let a_name = a
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let b_name = b
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            natord::compare(&a_name, &b_name)
        }
        crate::settings::SortMode::Date | crate::settings::SortMode::DateTaken => {
            let a_time = a.metadata().and_then(|m| m.modified()).ok();
            let b_time = b.metadata().and_then(|m| m.modified()).ok();
            a_time.cmp(&b_time)
        }
        crate::settings::SortMode::Size => {
            let a_size = a.metadata().map(|m| m.len()).unwrap_or(0);
            let b_size = b.metadata().map(|m| m.len()).unwrap_or(0);
            a_size.cmp(&b_size)
        }
        crate::settings::SortMode::Type => {
            let a_ext = a
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let b_ext = b
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            a_ext.cmp(&b_ext).then_with(|| {
                let a_name = a
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                let b_name = b
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                natord::compare(&a_name, &b_name)
            })
        }
        _ => unreachable!("Rating and Random handled separately"),
    }
}

impl ImageViewerApp {
    pub fn sort_images(&mut self) {
        let current_path = self.get_current_path();
        let sort_mode = self.settings.sort_mode;

        if matches!(sort_mode, crate::settings::SortMode::Random) {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            self.image_list.shuffle(&mut rng);
        } else {
            self.image_list
                .sort_by(|a, b| compare_paths_by_mode(a, b, sort_mode));
        }

        if !self.settings.sort_ascending {
            self.image_list.reverse();
        }

        // Restore selection
        if let Some(path) = current_path {
            if let Some(idx) = self.image_list.iter().position(|p| p == &path) {
                self.current_index = idx;
            }
        }
    }

    pub fn apply_filter(&mut self) {
        self.filtered_list.clear();

        for (idx, path) in self.image_list.iter().enumerate() {
            let _metadata = self.metadata_db.get(path);

            // Filter by search query
            if !self.search_query.is_empty() {
                let filename = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                if !filename.contains(&self.search_query.to_lowercase()) {
                    continue;
                }
            }

            self.filtered_list.push(idx);
        }

        // Ensure current_index is valid
        if self.current_index >= self.filtered_list.len() {
            self.current_index = self.filtered_list.len().saturating_sub(1);
        }
    }

    pub fn sort_file_list(&mut self) {
        self.sort_images();
        self.apply_filter();
    }
}
