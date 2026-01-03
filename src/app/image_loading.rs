use crate::exif_data::ExifInfo;
use crate::image_loader;
use crate::profiler;
use eframe::egui::{self, Vec2};
use image::DynamicImage;
use pollster;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use super::ImageViewerApp;

impl ImageViewerApp {
    pub fn load_image_file(&mut self, path: PathBuf) {
        if let Some(parent) = path.parent() {
            self.load_folder(parent.to_path_buf());

            if let Some(idx) = self.image_list.iter().position(|p| p == &path) {
                self.current_index = idx;
                // Load adjustments for this image
                self.load_adjustments_for_current();
                self.load_current_image();
            }
        }
    }

    pub fn spawn_loader<F>(&self, f: F)
    where
        F: FnOnce(&Sender<super::LoaderMessage>) -> Option<super::LoaderMessage> + Send + 'static,
    {
        let tx = self.loader_tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            if let Some(msg) = f(&tx) {
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

    fn try_load_from_cache(&mut self, path: &Path) -> bool {
        if let Some(image) = self.image_cache.get(path) {
            self.set_current_image(path, image);
            self.load_exif_data(path);
            return true;
        }
        false
    }

    fn load_raw_image(&mut self, path: &Path) {
        // Load quick preview first
        let path_clone = path.to_path_buf();
        self.spawn_loader(move |_tx| {
            image_loader::load_thumbnail(&path_clone, 1920)
                .ok()
                .map(|preview| super::LoaderMessage::PreviewLoaded(path_clone, preview))
        });

        if self.settings.load_raw_full_size {
            // Spawn full image load
            let path_clone = path.to_path_buf();
            self.spawn_loader(move |_tx| {
                Some(match image_loader::load_image(&path_clone) {
                    Ok(image) => super::LoaderMessage::ImageLoaded(path_clone, image),
                    Err(e) => super::LoaderMessage::LoadError(path_clone, format!("{}", e)),
                })
            });
        }
    }

    fn load_standard_image(&mut self, path: &Path) {
        // Load progressive versions for better UX
        let path_clone = path.to_path_buf();
        self.spawn_loader(move |_tx| match image_loader::load_image(&path_clone) {
            Ok(full_image) => {
                let preview = image_loader::generate_thumbnail(&full_image, 1920);
                Some(super::LoaderMessage::ProgressiveLoaded(
                    path_clone.clone(),
                    preview,
                ))
            }
            Err(e) => Some(super::LoaderMessage::LoadError(
                path_clone,
                format!("{}", e),
            )),
        });

        // Then load the full resolution
        let path_clone = path.to_path_buf();
        self.spawn_loader(move |_tx| {
            Some(match image_loader::load_image(&path_clone) {
                Ok(image) => super::LoaderMessage::ImageLoaded(path_clone, image),
                Err(e) => super::LoaderMessage::LoadError(path_clone, format!("{}", e)),
            })
        });
    }

    pub fn load_exif_data(&self, path: &Path) {
        let path_clone = path.to_path_buf();
        self.spawn_loader(move |_tx| {
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
        // Ensure full-refresh clears the preview flag
        self.showing_preview = false;

        // Apply adjustments if any (use GPU if available and no frames)
        let display_image = if !self.adjustments.is_default() && !self.show_original {
            profiler::with_profiler(|p| p.start_timer("apply_adjustments"));

            // Use CPU for frame processing since GPU doesn't support it yet
            let adjusted = if self.adjustments.frame_enabled {
                image_loader::apply_adjustments(&image, &self.adjustments)
            } else if let Some(gpu) = &self.gpu_processor {
                // Try GPU texture-based path first (async)
                let gpu_clone = Arc::clone(gpu);
                let image_clone = image.clone();
                let adjustments_clone = self.adjustments.clone();

                match pollster::block_on(async {
                    gpu_clone
                        .apply_adjustments_texture(&image_clone, &adjustments_clone)
                        .await
                }) {
                    Ok(img) => img,
                    Err(e) => {
                        log::warn!(
                            "GPU texture adjustments failed: {}; falling back to buffer method",
                            e
                        );
                        // Fallback to buffer-based GPU method
                        match gpu.apply_adjustments(&image_clone, &adjustments_clone) {
                            Ok(pixels) => {
                                let width = image_clone.width();
                                let height = image_clone.height();
                                if let Some(buf) =
                                    image::ImageBuffer::from_raw(width, height, pixels)
                                {
                                    DynamicImage::ImageRgba8(buf)
                                } else {
                                    log::warn!(
                                        "GPU returned unexpected buffer size; falling back to CPU"
                                    );
                                    image_loader::apply_adjustments(
                                        &image_clone,
                                        &adjustments_clone,
                                    )
                                }
                            }
                            Err(e) => {
                                log::warn!(
                                    "GPU buffer adjustments failed: {}; falling back to CPU",
                                    e
                                );
                                image_loader::apply_adjustments(&image_clone, &adjustments_clone)
                            }
                        }
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

        let size = [
            display_image.width() as usize,
            display_image.height() as usize,
        ];
        let rgba = display_image.to_rgba8();
        let _pixels = rgba.as_flat_samples();

        profiler::with_profiler(|p| p.start_timer("texture_load"));
        // Generate unique texture name to avoid cache conflicts when dimensions change (e.g., with frame)
        let texture_name = format!(
            "{}_{}_{}x{}",
            path.to_string_lossy(),
            self.adjustments.frame_enabled as u8,
            size[0],
            size[1]
        );

        // Check if texture is already cached
        let cached_texture =
            if let Some((cached_texture, _)) = self.texture_cache.get(&texture_name) {
                Some(cached_texture.clone())
            } else {
                None
            };

        if let Some(texture) = cached_texture {
            // Update access time for LRU
            self.texture_cache.insert(
                texture_name.clone(),
                (texture.clone(), std::time::Instant::now()),
            );
            // Move to front of access order
            if let Some(pos) = self
                .texture_access_order
                .iter()
                .position(|x| x == &texture_name)
            {
                self.texture_access_order.remove(pos);
            }
            self.texture_access_order.push_front(texture_name);

            self.current_texture = Some(texture);
            self.is_loading = false;
            profiler::with_profiler(|p| p.end_timer("texture_load"));
            return;
        }

        // Create texture asynchronously to avoid blocking UI
        let ctx_clone = self.ctx.clone();
        let texture_name_clone = texture_name.clone();
        let display_image_clone = display_image.clone();
        let _tx_clone = self.loader_tx.clone();

        self.spawn_loader(move |_tx| {
            if let Some(ctx) = ctx_clone {
                let rgba = display_image_clone.to_rgba8();
                let pixels = rgba.as_flat_samples();
                let size = [
                    display_image_clone.width() as usize,
                    display_image_clone.height() as usize,
                ];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                let texture = ctx.load_texture(
                    texture_name_clone.clone(),
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                Some(super::LoaderMessage::TextureCreated(
                    PathBuf::from(texture_name_clone),
                    texture,
                    display_image_clone,
                ))
            } else {
                None
            }
        });

        profiler::with_profiler(|p| p.end_timer("texture_load"));

        if !self.settings.maintain_pan_on_navigate {
            self.pan_offset = Vec2::ZERO;
            self.target_pan = Vec2::ZERO;
        }

        // Calculate histogram
        self.histogram_data = if let Some(gpu) = &self.gpu_processor {
            match pollster::block_on(async { gpu.compute_histogram(&display_image).await }) {
                Ok(hist) => Some(hist),
                Err(e) => {
                    log::warn!("GPU histogram failed: {}; falling back to CPU", e);
                    Some(image_loader::calculate_histogram(&display_image))
                }
            }
        } else {
            Some(image_loader::calculate_histogram(&display_image))
        };

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

    /// Fast version of set_current_image that skips histogram and overlay generation.
    /// Used during slider dragging for responsive UI.
    pub fn set_current_image_fast(&mut self, path: &std::path::Path, image: DynamicImage) {
        self.set_current_image_fast_internal(path, image, false);
    }

    /// Internal version with histogram computation control
    pub fn set_current_image_fast_internal(&mut self, path: &std::path::Path, image: DynamicImage, compute_histogram: bool) {
        // Silence unused variable warning; spawn_loader will check ctx again as needed
        let _ctx = match &self.ctx {
            Some(c) => c.clone(),
            None => return,
        };

        crate::profiler::with_profiler(|p| p.start_timer("set_current_image_fast_total"));

        self.current_image = Some(image.clone());

        // Fast-path: when dragging, use a downscaled preview and avoid GPU/blocking calls.
        let mut display_input = image.clone();
        self.showing_preview = false;
        if self.slider_dragging {
            // Target maximum preview dimension (fast to compute) — keep it modest
            let max_preview_dim = 1024u32;
            let max_dim = std::cmp::max(display_input.width(), display_input.height());
            if max_dim > max_preview_dim {
                let scale = (max_preview_dim as f32) / (max_dim as f32);
                let new_w = ((display_input.width() as f32) * scale).max(1.0) as u32;
                let new_h = ((display_input.height() as f32) * scale).max(1.0) as u32;
                display_input = image::DynamicImage::ImageRgba8(image::imageops::resize(
                    &display_input,
                    new_w,
                    new_h,
                    image::imageops::FilterType::Triangle,
                ));
                self.showing_preview = true;
                log::debug!("Using low-res preview {}x{} for fast adjustment rendering", new_w, new_h);
            }
        }

        // Generate unique texture name to avoid cache conflicts when dimensions change (e.g., with frame)
        let size = [
            display_input.width() as usize,
            display_input.height() as usize,
        ];
        let texture_name = format!(
            "{}_{}_{}x{}",
            path.to_string_lossy(),
            self.adjustments.frame_enabled as u8,
            size[0],
            size[1]
        );

        // Check if texture is already cached (fast path)
        let cached_texture = if let Some((cached_texture, _)) = self.texture_cache.get(&texture_name) {
            Some(cached_texture.clone())
        } else {
            None
        };

        if let Some(texture) = cached_texture {
            // Update access time for LRU
            self.texture_cache.insert(
                texture_name.clone(),
                (texture.clone(), std::time::Instant::now()),
            );
            // Move to front of access order
            if let Some(pos) = self
                .texture_access_order
                .iter()
                .position(|x| x == &texture_name)
            {
                self.texture_access_order.remove(pos);
            }
            self.texture_access_order.push_front(texture_name);

            self.current_texture = Some(texture);
            self.is_loading = false;
            crate::profiler::with_profiler(|p| p.end_timer("set_current_image_fast_total"));
            return;
        }

        // Defer heavy work (apply_adjustments + texture creation) to background thread to keep UI responsive.
        let ctx_clone = self.ctx.clone();
        let texture_name_clone = texture_name.clone();
        let display_input_clone = display_input.clone();
        let adjustments_clone = if self.slider_dragging {
            self.adjustments.preview()
        } else {
            self.adjustments.clone()
        };
        let show_original_clone = self.show_original;
        let gpu_clone = self.gpu_processor.clone();
        let compute_histogram_clone = compute_histogram;
        self.spawn_loader(move |tx| {
            let start = std::time::Instant::now();
            let display_image = if !adjustments_clone.is_default() && !show_original_clone {
                image_loader::apply_adjustments(&display_input_clone, &adjustments_clone)
            } else {
                display_input_clone.clone()
            };
            let elapsed = start.elapsed().as_millis();
            log::debug!("apply_adjustments_fast worker took {} ms for preview", elapsed);

            if compute_histogram_clone {
                let hist = if let Some(gpu) = &gpu_clone {
                    match pollster::block_on(async { gpu.compute_histogram(&display_image).await }) {
                        Ok(h) => h,
                        Err(e) => {
                            log::warn!("GPU histogram failed: {}; falling back to CPU", e);
                            image_loader::calculate_histogram(&display_image)
                        }
                    }
                } else {
                    image_loader::calculate_histogram(&display_image)
                };
                let _ = tx.send(super::LoaderMessage::HistogramUpdated(hist));
            }

            if let Some(ctx) = ctx_clone {
                let rgba = display_image.to_rgba8();
                let pixels = rgba.as_flat_samples();
                let size = [display_image.width() as usize, display_image.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                let texture = ctx.load_texture(texture_name_clone.clone(), color_image, egui::TextureOptions::LINEAR);
                Some(super::LoaderMessage::TextureCreated(PathBuf::from(texture_name_clone), texture, display_image))
            } else {
                None
            }
        });
        crate::profiler::with_profiler(|p| p.end_timer("set_current_image_fast_total"));
        // NOTE: Histogram and overlays are NOT updated here for performance
    }

    fn preload_adjacent(&self) {
        let count = self.settings.preload_adjacent;
        let mut full_paths = Vec::new();
        let mut thumb_paths = Vec::new();

        for i in 1..=count {
            if self.current_index + i < self.filtered_list.len() {
                if let Some(&idx) = self.filtered_list.get(self.current_index + i) {
                    if let Some(path) = self.image_list.get(idx) {
                        if crate::image_loader::is_raw_file(path)
                            && !self.settings.load_raw_full_size
                        {
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
                        if crate::image_loader::is_raw_file(path)
                            && !self.settings.load_raw_full_size
                        {
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
            self.image_cache
                .preload_thumbnails_parallel(thumb_paths, 1920);
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
            let _ = self
                .loader_tx
                .send(super::LoaderMessage::ThumbnailLoaded(path.clone(), img));
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

        // Limit concurrent thumbnail loads to prevent overwhelming the system
        if self.thumbnail_requests.len() >= 10 {
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
            // If RAW files are configured to preview-only, try embedded thumbnail extraction first
            // If that fails, fall back to generating a thumbnail via full RAW decode to ensure previews appear.
            if image_loader::is_raw_file(&p) && !load_raw_full_size {
                match image_loader::load_raw_embedded_thumbnail(&p, size) {
                    Ok(thumb) => {
                        let thumb_clone = thumb.clone();
                        cache.insert_thumbnail(p.clone(), thumb_clone);
                        let _ = tx.send(super::LoaderMessage::ThumbnailLoaded(p.clone(), thumb));
                        ctx.request_repaint();
                    }
                    Err(_) => {
                        log::warn!("No embedded thumbnail for {:?} — falling back to full decode for thumbnail", p);
                        if let Ok(thumb) = image_loader::load_thumbnail(&p, size) {
                            let thumb_clone = thumb.clone();
                            cache.insert_thumbnail(p.clone(), thumb_clone);
                            let _ =
                                tx.send(super::LoaderMessage::ThumbnailLoaded(p.clone(), thumb));
                            ctx.request_repaint();
                        }
                    }
                }
            } else if let Ok(thumb) = image_loader::load_thumbnail(&p, size) {
                let thumb_clone = thumb.clone();
                cache.insert_thumbnail(p.clone(), thumb_clone);
                let _ = tx.send(super::LoaderMessage::ThumbnailLoaded(p.clone(), thumb));
                ctx.request_repaint();
            }
            profiler::with_profiler(|p| p.end_timer("thumbnail_generation"));
            // Notify main thread that this thumbnail request has completed (so it can clear in-flight flags)
            let _ = tx.send(super::LoaderMessage::ThumbnailRequestComplete(p));
        });
    }

    /// Return a single-frame spinner character used in small UI elements (thumbnails)
    pub fn spinner_char(&self, ui: &egui::Ui) -> &'static str {
        let time = ui.input(|i| i.time);
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let idx = ((time * 10.0) as usize) % spinner_chars.len();
        spinner_chars[idx]
    }
}
