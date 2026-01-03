use crate::app::{ImageViewerApp, LoaderMessage};
use crate::exif_data::ExifInfo;
use crate::metadata::FileOperation;
use egui::ColorImage;
use image::DynamicImage;
use std::path::PathBuf;

impl ImageViewerApp {
    pub fn process_loader_messages(&mut self, ctx: &egui::Context) {
        // Limit the number of messages processed per frame to prevent UI blocking
        let max_messages_per_frame = 10;
        let mut messages_processed = 0;

        while messages_processed < max_messages_per_frame {
            match self.loader_rx.try_recv() {
                Ok(msg) => {
                    self.handle_loader_message(msg, ctx);
                    messages_processed += 1;
                }
                Err(_) => break, // No more messages
            }
        }
    }

    fn handle_loader_message(&mut self, msg: LoaderMessage, ctx: &egui::Context) {
        match msg {
            LoaderMessage::ImageLoaded(path, image) => self.handle_image_loaded(path, image),
            LoaderMessage::PreviewLoaded(path, preview) => {
                self.handle_preview_loaded(path, preview)
            }
            LoaderMessage::ProgressiveLoaded(path, progressive) => {
                self.handle_progressive_loaded(path, progressive)
            }
            LoaderMessage::ThumbnailLoaded(path, thumb) => {
                self.handle_thumbnail_loaded(path, thumb, ctx)
            }
            LoaderMessage::LoadError(path, error) => self.handle_load_error(path, error),
            LoaderMessage::ExifLoaded(path, exif) => self.handle_exif_loaded(path, exif),
            LoaderMessage::ThumbnailRequestComplete(path) => {
                self.handle_thumbnail_request_complete(path)
            }
            LoaderMessage::TextureCreated(texture_name, texture, image) => {
                self.handle_texture_created(texture_name, texture, image)
            }
            LoaderMessage::HistogramUpdated(hist) => {
                self.histogram_data = Some(hist);
            }
            LoaderMessage::MoveCompleted {
                from,
                dest_folder,
                success,
                error,
            } => self.handle_move_completed(from, dest_folder, success, error),
        }
    }

    fn handle_image_loaded(&mut self, path: PathBuf, image: DynamicImage) {
        crate::profiler::with_profiler(|p| p.increment_counter("images_loaded"));
        if self.get_current_path().as_ref() == Some(&path) {
            self.showing_preview = false;
            self.set_current_image(&path, image.clone());
            self.pending_fit_to_window = true;
        } else {
            self.image_cache.insert(path.clone(), image.clone());
        }
    }

    fn handle_preview_loaded(&mut self, path: PathBuf, preview: DynamicImage) {
        crate::profiler::with_profiler(|p| p.increment_counter("previews_loaded"));
        if self.get_current_path().as_ref() == Some(&path) && self.is_loading {
            self.showing_preview = true;
            self.set_current_image(&path, preview);
            self.is_loading =
                !crate::image_loader::is_raw_file(&path) || self.settings.load_raw_full_size;
        }
    }

    fn handle_progressive_loaded(&mut self, path: PathBuf, progressive: DynamicImage) {
        crate::profiler::with_profiler(|p| p.increment_counter("progressive_loaded"));
        if self.get_current_path().as_ref() == Some(&path) && self.is_loading {
            self.showing_preview = true;
            self.set_current_image(&path, progressive);
        }
    }

    fn handle_thumbnail_loaded(&mut self, path: PathBuf, thumb: DynamicImage, ctx: &egui::Context) {
        crate::profiler::with_profiler(|p| p.increment_counter("thumbnails_loaded"));

        // Apply adjustments to thumbnail if any exist for this image
        // Use the thumbnail-specific adjustment function to avoid parallel processing glitches
        let display_thumb = if let Some(adj) = self.metadata_db.get_adjustments(&path) {
            if !adj.is_default() {
                crate::image_loader::apply_adjustments_thumbnail(&thumb, &adj)
            } else {
                thumb
            }
        } else {
            thumb
        };

        let size = [
            display_thumb.width() as usize,
            display_thumb.height() as usize,
        ];
        let rgba = display_thumb.to_rgba8();
        let pixels = rgba.as_flat_samples();

        let texture = ctx.load_texture(
            format!("thumb_{}", path.display()),
            ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
            egui::TextureOptions::LINEAR,
        );

        self.thumbnail_textures.insert(path.clone(), texture);
        self.thumbnail_requests.remove(&path);
    }

    fn handle_load_error(&mut self, path: PathBuf, error: String) {
        crate::profiler::with_profiler(|p| p.increment_counter("load_errors"));
        log::error!("Failed to load {}: {}", path.display(), error);
        if self.get_current_path().as_ref() == Some(&path) {
            self.is_loading = false;
            self.load_error = Some(error);
        }
    }

    fn handle_exif_loaded(&mut self, path: PathBuf, exif: Box<ExifInfo>) {
        crate::profiler::with_profiler(|p| p.increment_counter("exif_loaded"));
        let exif_val = (*exif).clone();
        self.compare_exifs.insert(path.clone(), exif_val.clone());
        if self.get_current_path().as_ref() == Some(&path) {
            self.current_exif = Some(exif_val);
        }
    }

    fn handle_thumbnail_request_complete(&mut self, path: PathBuf) {
        self.thumbnail_requests.remove(&path);
    }

    fn handle_move_completed(
        &mut self,
        from: PathBuf,
        dest_folder: PathBuf,
        success: bool,
        error: Option<String>,
    ) {
        if success {
            let filename = from.file_name().unwrap_or_default();
            let to = dest_folder.join(filename);
            self.undo_history.push(FileOperation::Move {
                from: from.clone(),
                to: to.clone(),
            });

            if let Some(&idx) = self.filtered_list.get(self.current_index) {
                self.image_list.remove(idx);
            }
            self.image_cache.remove(&from);
            self.thumbnail_textures.remove(&from);

            self.apply_filter();
            if self.current_index >= self.filtered_list.len() && !self.filtered_list.is_empty() {
                self.current_index = self.filtered_list.len() - 1;
            }
            if !self.filtered_list.is_empty() {
                // Load adjustments for the new current image
                self.load_adjustments_for_current();
                self.load_current_image();
            } else {
                self.current_texture = None;
                self.current_image = None;
            }
            self.show_status(&format!("Moved to {}", dest_folder.display()));
            self.settings.add_quick_move_folder(dest_folder);
        } else {
            self.show_status(&format!(
                "Failed to move image: {}",
                error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }
    }

    fn handle_texture_created(
        &mut self,
        texture_name: PathBuf,
        texture: egui::TextureHandle,
        image: DynamicImage,
    ) {
        // Cache the texture for future use with access time
        let texture_name_str = texture_name.to_string_lossy().to_string();
        let now = std::time::Instant::now();

        // Remove from access order if already exists
        if let Some(pos) = self
            .texture_access_order
            .iter()
            .position(|x| x == &texture_name_str)
        {
            self.texture_access_order.remove(pos);
        }

        // Add to front of access order
        self.texture_access_order
            .push_front(texture_name_str.clone());
        self.texture_cache
            .insert(texture_name_str.clone(), (texture.clone(), now));

        // Implement LRU eviction - keep only last 200 textures to prevent memory leaks
        const MAX_TEXTURE_CACHE_SIZE: usize = 200;
        while self.texture_cache.len() > MAX_TEXTURE_CACHE_SIZE {
            if let Some(oldest_key) = self.texture_access_order.pop_back() {
                self.texture_cache.remove(&oldest_key);
            }
        }

        // If this is for the current image, set the texture (but not current_image!)
        // current_image should remain as the original unadjusted image, which is already
        // set in set_current_image(). The `image` parameter here is the display image
        // with adjustments applied, which should NOT be stored as current_image.
        if let Some(current_path) = self.get_current_image_path() {
            let expected_texture_name = format!(
                "{}_{}_{}x{}",
                current_path.to_string_lossy(),
                self.adjustments.frame_enabled as u8,
                image.width(),
                image.height()
            );

            if texture_name_str == expected_texture_name {
                self.current_texture = Some(texture);
                // Do NOT update current_image here - it's the adjusted/display image
                // current_image should stay as the original for re-applying adjustments
                self.is_loading = false;
                // If the texture matches the original image size, clear preview flag (full-res).
                // Otherwise keep showing_preview=true while a full-res texture may still be generated.
                if let Some(orig) = &self.current_image {
                    self.showing_preview =
                        !(image.width() == orig.width() && image.height() == orig.height());
                } else {
                    self.showing_preview = false;
                }
            }
        }
    }
}
