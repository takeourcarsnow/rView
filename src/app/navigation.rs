use crate::image_loader;
use crate::exif_data::ExifInfo;
use crate::profiler;
use eframe::egui::{self, Vec2};
use image::DynamicImage;
use std::path::PathBuf;
use std::sync::Arc;

use super::ImageViewerApp;

fn compare_paths_by_mode(a: &PathBuf, b: &PathBuf, sort_mode: crate::settings::SortMode) -> std::cmp::Ordering {
    match sort_mode {
        crate::settings::SortMode::Name => {
            let a_name = a.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            let b_name = b.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            natord::compare(&a_name, &b_name)
        },
        crate::settings::SortMode::Date | crate::settings::SortMode::DateTaken => {
            let a_time = a.metadata().and_then(|m| m.modified()).ok();
            let b_time = b.metadata().and_then(|m| m.modified()).ok();
            a_time.cmp(&b_time)
        },
        crate::settings::SortMode::Size => {
            let a_size = a.metadata().map(|m| m.len()).unwrap_or(0);
            let b_size = b.metadata().map(|m| m.len()).unwrap_or(0);
            a_size.cmp(&b_size)
        },
        crate::settings::SortMode::Type => {
            let a_ext = a.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            let b_ext = b.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            a_ext.cmp(&b_ext).then_with(|| {
                let a_name = a.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                let b_name = b.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                natord::compare(&a_name, &b_name)
            })
        },
        _ => unreachable!("Rating and Random handled separately"),
    }
}

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


    pub fn sort_images(&mut self) {
        let current_path = self.get_current_path();
        let sort_mode = self.settings.sort_mode;

        if matches!(sort_mode, crate::settings::SortMode::Random) {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            self.image_list.shuffle(&mut rng);
        } else if matches!(sort_mode, crate::settings::SortMode::Rating) {
            self.image_list.sort_by(|a, b| {
                let a_rating = self.metadata_db.get(a).rating;
                let b_rating = self.metadata_db.get(b).rating;
                b_rating.cmp(&a_rating)
            });
        } else {
            self.image_list.sort_by(|a, b| compare_paths_by_mode(a, b, sort_mode));
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

    fn compare_paths_with_mode(&self, a: &PathBuf, b: &PathBuf, sort_mode: crate::settings::SortMode) -> std::cmp::Ordering {
        match sort_mode {
            crate::settings::SortMode::Name => {
                let a_name = a.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                let b_name = b.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                natord::compare(&a_name, &b_name)
            },
            crate::settings::SortMode::Date | crate::settings::SortMode::DateTaken => {
                let a_time = a.metadata().and_then(|m| m.modified()).ok();
                let b_time = b.metadata().and_then(|m| m.modified()).ok();
                a_time.cmp(&b_time)
            },
            crate::settings::SortMode::Size => {
                let a_size = a.metadata().map(|m| m.len()).unwrap_or(0);
                let b_size = b.metadata().map(|m| m.len()).unwrap_or(0);
                a_size.cmp(&b_size)
            },
            crate::settings::SortMode::Type => {
                let a_ext = a.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                let b_ext = b.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                a_ext.cmp(&b_ext).then_with(|| {
                    let a_name = a.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                    let b_name = b.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                    natord::compare(&a_name, &b_name)
                })
            },
            crate::settings::SortMode::Rating => {
                let a_rating = self.metadata_db.get(a).rating;
                let b_rating = self.metadata_db.get(b).rating;
                b_rating.cmp(&a_rating) // Descending
            },
            crate::settings::SortMode::Random => unreachable!("Random sorting handled separately"),
        }
    }

    pub fn apply_filter(&mut self) {
        self.filtered_list.clear();

        for (idx, path) in self.image_list.iter().enumerate() {
            let _metadata = self.metadata_db.get(path);

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

    pub fn spawn_loader<F>(&self, f: F)
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
            self.reset_image_state();
            self.settings.last_file = Some(path.clone());

            if self.try_load_from_cache(&path) {
                return;
            }

            if image_loader::is_raw_file(&path) {
                self.load_raw_image(&path);
            } else {
                self.load_standard_image(&path);
            }

            self.load_exif_data(&path);
            self.preload_adjacent();
        }
    }

    fn reset_image_state(&mut self) {
        self.is_loading = true;
        self.load_error = None;
        self.current_exif = None;
        self.histogram_data = None;
        self.focus_peaking_texture = None;
        self.zebra_texture = None;
        self.showing_preview = false;
    }

    fn try_load_from_cache(&mut self, path: &PathBuf) -> bool {
        if let Some(image) = self.image_cache.get(path) {
            self.set_current_image(path, image);
            self.load_exif_data(path);
            return true;
        }
        false
    }

    fn load_raw_image(&mut self, path: &PathBuf) {
        // Load quick preview first
        let path_clone = path.clone();
        self.spawn_loader(move || {
            image_loader::load_thumbnail(&path_clone, 1920)
                .ok()
                .map(|preview| super::LoaderMessage::PreviewLoaded(path_clone, preview))
        });

        if self.settings.load_raw_full_size {
            // Spawn full image load
            let path_clone = path.clone();
            self.spawn_loader(move || {
                Some(match image_loader::load_image(&path_clone) {
                    Ok(image) => super::LoaderMessage::ImageLoaded(path_clone, image),
                    Err(e) => super::LoaderMessage::LoadError(path_clone, e.to_string()),
                })
            });
        }
    }

    fn load_standard_image(&mut self, path: &PathBuf) {
        // Load progressive versions for better UX
        let path_clone = path.clone();
        self.spawn_loader(move || {
            match image_loader::load_image(&path_clone) {
                Ok(full_image) => {
                    let preview = image_loader::generate_thumbnail(&full_image, 1920);
                    Some(super::LoaderMessage::ProgressiveLoaded(path_clone.clone(), preview))
                }
                Err(e) => Some(super::LoaderMessage::LoadError(path_clone, e.to_string())),
            }
        });

        // Then load the full resolution
        let path_clone = path.clone();
        self.spawn_loader(move || {
            Some(match image_loader::load_image(&path_clone) {
                Ok(image) => super::LoaderMessage::ImageLoaded(path_clone, image),
                Err(e) => super::LoaderMessage::LoadError(path_clone, e.to_string()),
            })
        });
    }

    fn load_exif_data(&self, path: &PathBuf) {
        let path_clone = path.clone();
        self.spawn_loader(move || {
            let exif = ExifInfo::from_file(&path_clone);
            Some(super::LoaderMessage::ExifLoaded(path_clone, Box::new(exif)))
        });
    }

    pub fn set_current_image(&mut self, path: &std::path::Path, image: DynamicImage) {
        let ctx = match &self.ctx {
            Some(c) => c.clone(),
            None => return,
        };

        self.current_image = Some(image.clone());

        // Apply adjustments if any (use GPU if available)
        let display_image = if !self.adjustments.is_default() && !self.show_original {
            profiler::with_profiler(|p| p.start_timer("apply_adjustments"));

            let adjusted = if let Some(gpu) = &self.gpu_processor {
                // Try GPU path first
                match gpu.apply_adjustments(&image, &self.adjustments) {
                    Ok(pixels) => {
                        // Convert back to DynamicImage
                        let width = image.width();
                        let height = image.height();
                        if let Some(buf) = image::ImageBuffer::from_raw(width, height, pixels) {
                            DynamicImage::ImageRgba8(buf)
                        } else {
                            // Fallback to CPU
                            log::warn!("GPU returned unexpected buffer size; falling back to CPU adjustments");
                            image_loader::apply_adjustments(&image, &self.adjustments)
                        }
                    }
                    Err(e) => {
                        log::warn!("GPU adjustments failed: {}; falling back to CPU", e);
                        image_loader::apply_adjustments(&image, &self.adjustments)
                    }
                }
            } else {
                image_loader::apply_adjustments(&image, &self.adjustments)
            };

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
        self.image_cache.insert(path.to_path_buf(), image);
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
        let mut full_paths = Vec::new();
        let mut thumb_paths = Vec::new();

        for i in 1..=count {
            if self.current_index + i < self.filtered_list.len() {
                if let Some(&idx) = self.filtered_list.get(self.current_index + i) {
                    if let Some(path) = self.image_list.get(idx) {
                        if crate::image_loader::is_raw_file(path) && !self.settings.load_raw_full_size {
                            thumb_paths.push(path.clone());
                        } else {
                            full_paths.push(path.clone());
                        }
                    }
                }
            }
            if self.current_index >= i {
                if let Some(&idx) = self.filtered_list.get(self.current_index - i) {
                    if let Some(path) = self.image_list.get(idx) {
                        if crate::image_loader::is_raw_file(path) && !self.settings.load_raw_full_size {
                            thumb_paths.push(path.clone());
                        } else {
                            full_paths.push(path.clone());
                        }
                    }
                }
            }
        }

        if !full_paths.is_empty() {
            self.image_cache.preload(full_paths);
        }
        if !thumb_paths.is_empty() {
            // Preload embedded previews for RAW files (size this to a large value to get good-quality previews)
            self.image_cache.preload_thumbnails_parallel(thumb_paths, 1920);
        }
    }

    /// Ensure a thumbnail is requested: short-circuit on in-memory cache or already-requested, otherwise spawn background work
    pub fn ensure_thumbnail_requested(&mut self, path: &PathBuf, ctx: &egui::Context) {
        // If texture already present, nothing to do
        if self.thumbnail_textures.contains_key(path) {
            return;
        }

        // If a request is already in flight, nothing to do
        if self.thumbnail_requests.contains(path) {
            return;
        }

        // Try a synchronous cache lookup to quickly satisfy from disk cache
        if let Some(img) = self.image_cache.get_thumbnail(path) {
            let _ = self.loader_tx.send(super::LoaderMessage::ThumbnailLoaded(path.clone(), img));
            ctx.request_repaint();
            return;
        }

        // Otherwise spawn the background request
        self.request_thumbnail(path.clone(), ctx.clone());
    }

    pub fn request_thumbnail(&mut self, path: PathBuf, ctx: egui::Context) {
        if self.thumbnail_requests.contains(&path) {
            return;
        }

        self.thumbnail_requests.insert(path.clone());

        let tx = self.loader_tx.clone();
        let size = self.settings.thumbnail_size as u32;
        let cache = Arc::clone(&self.image_cache);
        let load_raw_full_size = self.settings.load_raw_full_size;

        rayon::spawn(move || {
            // Clone once for use in sends to avoid moving the original too early
            let p = path.clone();

            profiler::with_profiler(|p| p.start_timer("thumbnail_cache_lookup"));
            let cache_hit = cache.get_thumbnail(&p).is_some();
            profiler::with_profiler(|p| p.end_timer("thumbnail_cache_lookup"));

            if cache_hit {
                let thumb = cache.get_thumbnail(&p).unwrap();
                let _ = tx.send(super::LoaderMessage::ThumbnailLoaded(p.clone(), thumb));
                ctx.request_repaint();
                return;
            }

            profiler::with_profiler(|p| p.start_timer("thumbnail_generation"));
            // If RAW files are configured to be preview-only, try embedded thumbnail extraction first
            // If that fails, fall back to generating a thumbnail via full RAW decode to ensure previews appear.
            if image_loader::is_raw_file(&p) && !load_raw_full_size {
                match image_loader::load_raw_embedded_thumbnail(&p, size) {
                    Ok(thumb) => {
                        cache.insert_thumbnail(p.clone(), thumb.clone());
                        let _ = tx.send(super::LoaderMessage::ThumbnailLoaded(p.clone(), thumb));
                        ctx.request_repaint();
                    }
                    Err(_) => {
                        log::warn!("No embedded thumbnail for {:?} — falling back to full decode for thumbnail", p);
                        if let Ok(thumb) = image_loader::load_thumbnail(&p, size) {
                            cache.insert_thumbnail(p.clone(), thumb.clone());
                            let _ = tx.send(super::LoaderMessage::ThumbnailLoaded(p.clone(), thumb));
                            ctx.request_repaint();
                        }
                    }
                }
            } else {
                if let Ok(thumb) = image_loader::load_thumbnail(&p, size) {
                    cache.insert_thumbnail(p.clone(), thumb.clone());
                    let _ = tx.send(super::LoaderMessage::ThumbnailLoaded(p.clone(), thumb));
                    ctx.request_repaint();
                }
            }
            profiler::with_profiler(|p| p.end_timer("thumbnail_generation"));
            // Notify main thread that this thumbnail request has completed (so it can clear in-flight flags)
            let _ = tx.send(super::LoaderMessage::ThumbnailRequestComplete(p));
        });
    }

    /// Return a single-frame spinner character used in small UI elements (thumbnails)
    pub fn spinner_char(&self, ui: &egui::Ui) -> &'static str {
        let time = ui.input(|i| i.time);
        match (time as i32) % 4 {
            0 => "◐",
            1 => "◓",
            2 => "◑",
            _ => "◒",
        }
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