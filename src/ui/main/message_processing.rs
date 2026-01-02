use crate::app::{ImageViewerApp, LoaderMessage};
use crate::metadata::FileOperation;
use crate::exif_data::ExifInfo;
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
            LoaderMessage::PreviewLoaded(path, preview) => self.handle_preview_loaded(path, preview),
            LoaderMessage::ProgressiveLoaded(path, progressive) => self.handle_progressive_loaded(path, progressive),
            LoaderMessage::ThumbnailLoaded(path, thumb) => self.handle_thumbnail_loaded(path, thumb, ctx),
            LoaderMessage::LoadError(path, error) => self.handle_load_error(path, error),
            LoaderMessage::ExifLoaded(path, exif) => self.handle_exif_loaded(path, exif),
            LoaderMessage::ThumbnailRequestComplete(path) => self.handle_thumbnail_request_complete(path),
            LoaderMessage::MoveCompleted { from, dest_folder, success, error } => self.handle_move_completed(from, dest_folder, success, error),
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
            self.is_loading = !crate::image_loader::is_raw_file(&path) || self.settings.load_raw_full_size;
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
        let size = [thumb.width() as usize, thumb.height() as usize];
        let rgba = thumb.to_rgba8();
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

    fn handle_move_completed(&mut self, from: PathBuf, dest_folder: PathBuf, success: bool, error: Option<String>) {
        if success {
            let filename = from.file_name().unwrap_or_default();
            let to = dest_folder.join(filename);
            self.undo_history.push(FileOperation::Move { from: from.clone(), to: to.clone() });

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
                self.load_current_image();
            } else {
                self.current_texture = None;
                self.current_image = None;
            }
            self.show_status(&format!("Moved to {}", dest_folder.display()));
            self.settings.add_quick_move_folder(dest_folder);
        } else {
            self.show_status(&format!("Failed to move image: {}", error.unwrap_or_else(|| "Unknown error".to_string())));
        }
    }
}