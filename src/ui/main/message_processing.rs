use crate::app::{ImageViewerApp, LoaderMessage};
use crate::metadata::FileOperation;
use egui::ColorImage;

impl ImageViewerApp {
    pub fn process_loader_messages(&mut self, ctx: &egui::Context) {
        // Limit the number of messages processed per frame to prevent UI blocking
        let max_messages_per_frame = 10;
        let mut messages_processed = 0;

        while messages_processed < max_messages_per_frame {
            match self.loader_rx.try_recv() {
                Ok(msg) => {
                    match msg {
                        LoaderMessage::ImageLoaded(path, image) => {
                            crate::profiler::with_profiler(|p| p.increment_counter("images_loaded"));
                            if self.get_current_path().as_ref() == Some(&path) {
                                self.showing_preview = false;
                                self.set_current_image(&path, image.clone());
                                // Default to fitting the image to the view when it is loaded
                                self.pending_fit_to_window = true;
                            } else {
                                self.image_cache.insert(path.clone(), image.clone());
                            }

                        }
                        LoaderMessage::PreviewLoaded(path, preview) => {
                            crate::profiler::with_profiler(|p| p.increment_counter("previews_loaded"));
                            // Only use preview if we're still waiting for this image and don't have the full one yet
                            if self.get_current_path().as_ref() == Some(&path) && self.is_loading {
                                self.showing_preview = true;
                                self.set_current_image(&path, preview);
                                // If this is a RAW file and the user disabled full-size RAW decoding, stop the loading indicator
                                if crate::image_loader::is_raw_file(&path) && !self.settings.load_raw_full_size {
                                    self.is_loading = false;
                                } else {
                                    self.is_loading = true; // Keep loading indicator for full image
                                }
                            }
                        }
                        LoaderMessage::ProgressiveLoaded(path, progressive) => {
                            crate::profiler::with_profiler(|p| p.increment_counter("progressive_loaded"));
                            // Use progressive image if we're still waiting for the full image
                            if self.get_current_path().as_ref() == Some(&path) && self.is_loading {
                                self.showing_preview = true;
                                self.set_current_image(&path, progressive);
                                // Keep loading indicator for full image
                            }
                        }
                        LoaderMessage::ThumbnailLoaded(path, thumb) => {
                            crate::profiler::with_profiler(|p| p.increment_counter("thumbnails_loaded"));
                            let size = [thumb.width() as usize, thumb.height() as usize];
                            let rgba = thumb.to_rgba8();
                            let pixels = rgba.as_flat_samples();

                            let texture = ctx.load_texture(
                                format!("thumb_{}", path.display()),
                                ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
                                egui::TextureOptions::LINEAR,
                            );

                            self.thumbnail_textures.insert(path, texture);
                        }
                        LoaderMessage::LoadError(path, error) => {
                            crate::profiler::with_profiler(|p| p.increment_counter("load_errors"));
                            log::error!("Failed to load {}: {}", path.display(), error);
                            if self.get_current_path().as_ref() == Some(&path) {
                                self.is_loading = false;
                                self.load_error = Some(error);
                            }
                        }
                        LoaderMessage::ExifLoaded(path, exif) => {
                            crate::profiler::with_profiler(|p| p.increment_counter("exif_loaded"));
                            // Clone the exif info to avoid moving it twice
                            let exif_val = (*exif).clone();
                            self.compare_exifs.insert(path.clone(), exif_val.clone());
                            if self.get_current_path().as_ref() == Some(&path) {
                                self.current_exif = Some(exif_val);
                            }
                        }
                        LoaderMessage::MoveCompleted { from, dest_folder, success, error } => {
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
                    messages_processed += 1;
                }
                Err(_) => break, // No more messages
            }
        }
    }
}