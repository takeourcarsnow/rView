
use super::ImageViewerApp;

impl ImageViewerApp {
    pub fn get_cache_stats(&self) -> crate::image_cache::CacheStats {
        self.image_cache.get_stats()
    }

    pub fn clear_image_cache(&mut self) {
        self.image_cache.clear();
        self.thumbnail_textures.clear();
        self.thumbnail_requests.clear();
        self.show_status("Cache cleared");
    }

    pub fn preload_adjacent_images(&self) {
        let count = self.settings.preload_adjacent;
        let mut paths = Vec::new();

        for i in 1..=count {
            if self.current_index + i < self.filtered_list.len() {
                if let Some(&idx) = self.filtered_list.get(self.current_index + i) {
                    if let Some(path) = self.image_list.get(idx) {
                        paths.push(path.clone());
                    }
                }
            }
            if self.current_index >= i {
                if let Some(&idx) = self.filtered_list.get(self.current_index - i) {
                    if let Some(path) = self.image_list.get(idx) {
                        paths.push(path.clone());
                    }
                }
            }
        }

        self.image_cache.preload(paths);
    }

    pub fn preload_thumbnails_parallel(&self, paths: Vec<std::path::PathBuf>, size: u32) {
        self.image_cache.preload_thumbnails_parallel(paths, size);
    }
}