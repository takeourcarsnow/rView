use crate::image_loader;
use crate::exif_data::ExifInfo;
use crate::profiler;
use eframe::egui::{self, Vec2};
use image::DynamicImage;
use std::path::PathBuf;
use std::sync::Arc;

use super::ImageViewerApp;

impl ImageViewerApp {
    pub fn load_image_file(&mut self, path: PathBuf) {
        if let Some(parent) = path.parent() {
            self.load_folder(parent.to_path_buf());

            if let Some(idx) = self.image_list.iter().position(|p| p == &path) {
                self.current_index = idx;
                self.load_current_image();
            }
        }
    }

    pub fn load_folder(&mut self, folder: PathBuf) {
        // Create a new tab for the folder
        self.create_tab(folder);
    }

    pub fn sort_images(&mut self) {
        let current_path = self.get_current_path();

        match self.settings.sort_mode {
            crate::settings::SortMode::Name => {
                self.image_list.sort_by(|a, b| {
                    let a_name = a.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                    let b_name = b.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                    natord::compare(&a_name, &b_name)
                });
            }
            crate::settings::SortMode::Date => {
                self.image_list.sort_by(|a, b| {
                    let a_time = a.metadata().and_then(|m| m.modified()).ok();
                    let b_time = b.metadata().and_then(|m| m.modified()).ok();
                    a_time.cmp(&b_time)
                });
            }
            crate::settings::SortMode::DateTaken => {
                // Would need EXIF data cached - fall back to file date for now
                self.image_list.sort_by(|a, b| {
                    let a_time = a.metadata().and_then(|m| m.modified()).ok();
                    let b_time = b.metadata().and_then(|m| m.modified()).ok();
                    a_time.cmp(&b_time)
                });
            }
            crate::settings::SortMode::Size => {
                self.image_list.sort_by(|a, b| {
                    let a_size = a.metadata().map(|m| m.len()).unwrap_or(0);
                    let b_size = b.metadata().map(|m| m.len()).unwrap_or(0);
                    a_size.cmp(&b_size)
                });
            }
            crate::settings::SortMode::Type => {
                self.image_list.sort_by(|a, b| {
                    let a_ext = a.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                    let b_ext = b.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                    a_ext.cmp(&b_ext).then_with(|| {
                        let a_name = a.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                        let b_name = b.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                        natord::compare(&a_name, &b_name)
                    })
                });
            }
            crate::settings::SortMode::Rating => {
                self.image_list.sort_by(|a, b| {
                    let a_rating = self.metadata_db.get(a).rating;
                    let b_rating = self.metadata_db.get(b).rating;
                    b_rating.cmp(&a_rating) // Descending
                });
            }
            crate::settings::SortMode::Random => {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                self.image_list.shuffle(&mut rng);
            }
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
            let metadata = self.metadata_db.get(path);

            // Filter by rating
            if let Some(min_rating) = self.settings.filter_by_rating {
                if metadata.rating < min_rating {
                    continue;
                }
            }

            // Filter by color
            if let Some(ref color) = self.settings.filter_by_color {
                if &metadata.color_label != color {
                    continue;
                }
            }

            // Filter by search query
            if !self.search_query.is_empty() {
                let filename = path.file_name()
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

    pub fn get_current_path(&self) -> Option<PathBuf> {
        self.filtered_list.get(self.current_index)
            .and_then(|&idx| self.image_list.get(idx))
            .cloned()
    }

    fn spawn_loader<F>(&self, f: F)
    where
        F: FnOnce() -> Option<super::LoaderMessage> + Send + 'static,
    {
        let tx = self.loader_tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            if let Some(msg) = f() {
                let _ = tx.send(msg);
            }
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
    }

    pub fn load_current_image(&mut self) {
        if let Some(path) = self.get_current_path() {
            self.is_loading = true;
            self.load_error = None;
            self.current_exif = None;
            self.histogram_data = None;
            self.focus_peaking_texture = None;
            self.zebra_texture = None;
            self.showing_preview = false;
            self.settings.last_file = Some(path.clone());

            // Check cache first
            if let Some(image) = self.image_cache.get(&path) {
                self.set_current_image(&path, image);
                return;
            }

            // For RAW files, load a quick preview first then the full image
            let is_raw = image_loader::is_raw_file(&path);

            if is_raw {
                // Load quick preview first (smaller resolution)
                let path_clone = path.clone();
                self.spawn_loader(move || {
                    image_loader::load_thumbnail(&path_clone, 1920)
                        .ok()
                        .map(|preview| super::LoaderMessage::PreviewLoaded(path_clone, preview))
                });
            }

            // Load full image asynchronously
            let path_clone = path.clone();
            self.spawn_loader(move || {
                Some(match image_loader::load_image(&path_clone) {
                    Ok(image) => super::LoaderMessage::ImageLoaded(path_clone, image),
                    Err(e) => super::LoaderMessage::LoadError(path_clone, e.to_string()),
                })
            });

            // Load EXIF data asynchronously
            let path_clone = path.clone();
            self.spawn_loader(move || {
                let exif = ExifInfo::from_file(&path_clone);
                Some(super::LoaderMessage::ExifLoaded(path_clone, exif))
            });

            self.preload_adjacent();
        }
    }

    pub fn set_current_image(&mut self, path: &PathBuf, image: DynamicImage) {
        let ctx = match &self.ctx {
            Some(c) => c.clone(),
            None => return,
        };

        self.current_image = Some(image.clone());

        // Apply adjustments if any
        let display_image = if !self.adjustments.is_default() && !self.show_original {
            profiler::with_profiler(|p| p.start_timer("apply_adjustments"));
            let adjusted = image_loader::apply_adjustments(&image, &self.adjustments);
            profiler::with_profiler(|p| p.end_timer("apply_adjustments"));
            adjusted
        } else {
            image.clone()
        };

        let size = [display_image.width() as usize, display_image.height() as usize];
        let rgba = display_image.to_rgba8();
        let pixels = rgba.as_flat_samples();

        profiler::with_profiler(|p| p.start_timer("texture_load"));
        let texture = ctx.load_texture(
            path.to_string_lossy(),
            egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
            egui::TextureOptions::LINEAR,
        );
        profiler::with_profiler(|p| p.end_timer("texture_load"));

        self.current_texture = Some(texture);
        self.is_loading = false;

        if !self.settings.maintain_pan_on_navigate {
            self.pan_offset = Vec2::ZERO;
            self.target_pan = Vec2::ZERO;
        }

        // Calculate histogram
        self.histogram_data = Some(image_loader::calculate_histogram(&image));

        // Generate overlays if enabled
        if self.settings.show_focus_peaking {
            self.generate_focus_peaking_overlay(&image, &ctx);
        }

        if self.settings.show_zebras {
            self.generate_zebra_overlay(&image, &ctx);
        }

        // Cache the image
        self.image_cache.insert(path.clone(), image);
    }

    pub fn generate_focus_peaking_overlay(&mut self, image: &DynamicImage, ctx: &egui::Context) {
        let overlay = image_loader::generate_focus_peaking_overlay(
            image,
            self.settings.focus_peaking_threshold
        );

        let size = [overlay.width() as usize, overlay.height() as usize];
        let pixels: Vec<u8> = overlay.into_raw();

        let texture = ctx.load_texture(
            "focus_peaking",
            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
            egui::TextureOptions::LINEAR,
        );

        self.focus_peaking_texture = Some(texture);
    }

    pub fn generate_zebra_overlay(&mut self, image: &DynamicImage, ctx: &egui::Context) {
        let overlay = image_loader::generate_zebra_overlay(
            image,
            self.settings.zebra_high_threshold,
            self.settings.zebra_low_threshold
        );

        let size = [overlay.width() as usize, overlay.height() as usize];
        let pixels: Vec<u8> = overlay.into_raw();

        let texture = ctx.load_texture(
            "zebra",
            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
            egui::TextureOptions::LINEAR,
        );

        self.zebra_texture = Some(texture);
    }

    fn preload_adjacent(&self) {
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

    pub fn request_thumbnail(&mut self, path: PathBuf, ctx: egui::Context) {
        if self.thumbnail_requests.contains(&path) {
            return;
        }

        self.thumbnail_requests.insert(path.clone());

        let tx = self.loader_tx.clone();
        let size = self.settings.thumbnail_size as u32;
        let cache = Arc::clone(&self.image_cache);

        std::thread::spawn(move || {
            profiler::with_profiler(|p| p.start_timer("thumbnail_cache_lookup"));
            let cache_hit = cache.get_thumbnail(&path).is_some();
            profiler::with_profiler(|p| p.end_timer("thumbnail_cache_lookup"));

            if cache_hit {
                let thumb = cache.get_thumbnail(&path).unwrap();
                let _ = tx.send(super::LoaderMessage::ThumbnailLoaded(path, thumb));
                ctx.request_repaint();
                return;
            }

            profiler::with_profiler(|p| p.start_timer("thumbnail_generation"));
            if let Ok(thumb) = image_loader::load_thumbnail(&path, size) {
                cache.insert_thumbnail(path.clone(), thumb.clone());
                let _ = tx.send(super::LoaderMessage::ThumbnailLoaded(path, thumb));
                ctx.request_repaint();
            }
            profiler::with_profiler(|p| p.end_timer("thumbnail_generation"));
        });
    }

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

    pub fn sort_file_list(&mut self) {
        self.sort_images();
        self.apply_filter();
    }
}