use crate::exif_data::ExifInfo;
use crate::gpu::types::GpuProcessor;
use crate::image_loader;
use crate::profiler;
use eframe::egui::{self, TextureHandle, Vec2};
use image::{imageops, DynamicImage, ImageBuffer, Rgba};
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

            // Use new task scheduler for prioritized loading
            self.task_scheduler.submit_task(crate::task_scheduler::ImageTask::LoadImage {
                path: path.clone(),
                priority: crate::task_scheduler::TaskPriority::Critical,
            });

            // Load EXIF with high priority
            self.task_scheduler.submit_task(crate::task_scheduler::ImageTask::LoadExif {
                path: path.clone(),
                priority: crate::task_scheduler::TaskPriority::High,
            });

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
        self.showing_preview = false;

        let adjusted_image = self.apply_adjustments_with_fallbacks(&image);
        let display_image = self.apply_frame_to_image(&adjusted_image);

        self.create_texture_and_setup(path, &display_image, &ctx, &adjusted_image, &image);
    }

    fn apply_frame_to_image(&self, image: &DynamicImage) -> DynamicImage {
        if self.adjustments.frame_enabled && self.adjustments.frame_thickness > 0.0 {
            let img = image.to_rgba8();
            let (width, height) = img.dimensions();
            let thickness = self.adjustments.frame_thickness as u32;
            let new_width = width + 2 * thickness;
            let new_height = height + 2 * thickness;

            let mut framed = ImageBuffer::new(new_width, new_height);

            let frame_r = (self.adjustments.frame_color[0] * 255.0) as u8;
            let frame_g = (self.adjustments.frame_color[1] * 255.0) as u8;
            let frame_b = (self.adjustments.frame_color[2] * 255.0) as u8;

            for pixel in framed.pixels_mut() {
                *pixel = Rgba([frame_r, frame_g, frame_b, 255]);
            }

            imageops::overlay(&mut framed, &img, thickness as i64, thickness as i64);

            DynamicImage::ImageRgba8(framed)
        } else {
            image.clone()
        }
    }

    fn apply_adjustments_with_fallbacks(&self, image: &DynamicImage) -> DynamicImage {
        // Try GPU texture-based path first (async)
        if let Some(gpu) = &self.gpu_processor {
            match pollster::block_on(async {
                gpu.apply_adjustments_texture(image, &self.adjustments)
                    .await
            }) {
                Ok(img) => return img,
                Err(e) => {
                    log::warn!(
                        "GPU texture adjustments failed: {}; falling back to buffer method",
                        e
                    );
                }
            }

            // Fallback to buffer-based GPU method
            match gpu.apply_adjustments(image, &self.adjustments) {
                Ok(pixels) => {
                    let width = image.width();
                    let height = image.height();
                    if let Some(buf) = image::ImageBuffer::from_raw(width, height, pixels) {
                        return DynamicImage::ImageRgba8(buf);
                    } else {
                        log::warn!("GPU returned unexpected buffer size; falling back to CPU");
                    }
                }
                Err(e) => {
                    log::warn!("GPU buffer adjustments failed: {}; falling back to CPU", e);
                }
            }
        }

        // Final fallback to CPU
        image_loader::apply_adjustments(image, &self.adjustments)
    }

    fn create_texture_and_setup(
        &mut self,
        path: &std::path::Path,
        display_image: &DynamicImage,
        ctx: &egui::Context,
        adjusted_image: &DynamicImage,
        original_image: &DynamicImage,
    ) {
        let size = [
            display_image.width() as usize,
            display_image.height() as usize,
        ];
        let rgba = display_image.to_rgba8();
        let _pixels = rgba.as_flat_samples();

        profiler::with_profiler(|p| p.start_timer("texture_load"));

        let texture_name = self.generate_texture_name(path, size[0], size[1]);

        // Check if texture is already cached
        if let Some(texture) = self.get_cached_texture(&texture_name) {
            self.current_texture = Some(texture);
            self.is_loading = false;
            profiler::with_profiler(|p| p.end_timer("texture_load"));
            return;
        }

        // Create texture asynchronously
        self.create_texture_async(path, display_image, ctx, texture_name);

        if !self.settings.maintain_pan_on_navigate {
            self.pan_offset = Vec2::ZERO;
            self.target_pan = Vec2::ZERO;
        }

        // Calculate histogram
        self.histogram_data = Some(self.compute_histogram(adjusted_image));

        // Generate overlays if enabled
        self.generate_overlays_if_needed(adjusted_image, ctx);

        // Cache the image
        self.image_cache
            .insert(path.to_path_buf(), original_image.clone());
    }

    fn generate_texture_name(&self, path: &std::path::Path, width: usize, height: usize) -> String {
        format!(
            "{}_{}_{}x{}",
            path.to_string_lossy(),
            self.adjustments.frame_enabled as u8,
            width,
            height
        )
    }

    fn get_cached_texture(&mut self, texture_name: &str) -> Option<TextureHandle> {
        if let Some((cached_texture, _)) = self.texture_cache.get(texture_name) {
            let texture = cached_texture.clone();
            // Update access time for LRU
            self.texture_cache.insert(
                texture_name.to_string(),
                (texture.clone(), std::time::Instant::now()),
            );
            // Move to front of access order
            self.update_texture_access_order(texture_name);
            Some(texture)
        } else {
            None
        }
    }

    fn update_texture_access_order(&mut self, texture_name: &str) {
        if let Some(pos) = self
            .texture_access_order
            .iter()
            .position(|x| x == texture_name)
        {
            self.texture_access_order.remove(pos);
        }
        self.texture_access_order
            .push_front(texture_name.to_string());
    }

    fn create_texture_async(
        &self,
        _path: &std::path::Path,
        display_image: &DynamicImage,
        _ctx: &egui::Context,
        texture_name: String,
    ) {
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
    }

    fn compute_histogram(&self, display_image: &DynamicImage) -> Vec<Vec<u32>> {
        if let Some(gpu) = &self.gpu_processor {
            match pollster::block_on(async { gpu.compute_histogram(display_image).await }) {
                Ok(hist) => hist,
                Err(e) => {
                    log::warn!("GPU histogram failed: {}; falling back to CPU", e);
                    self.compute_histogram_cpu(display_image)
                }
            }
        } else {
            self.compute_histogram_cpu(display_image)
        }
    }

    fn compute_histogram_cpu(&self, display_image: &DynamicImage) -> Vec<Vec<u32>> {
        // Use concurrent histogram computation for better performance
        let tile_count = crate::task_scheduler::concurrent_histogram::optimal_tile_count(
            display_image.width(),
            display_image.height(),
        );
        crate::task_scheduler::concurrent_histogram::compute_parallel(display_image, tile_count)
    }

    fn generate_overlays_if_needed(&mut self, image: &DynamicImage, ctx: &egui::Context) {
        if self.settings.show_focus_peaking {
            self.generate_focus_peaking_overlay(image, ctx);
        }

        if self.settings.show_zebras {
            self.generate_zebra_overlay(image, ctx);
        }

        if self.settings.show_custom_overlay {
            self.load_custom_overlay(ctx);
        }

        if self.settings.show_frame {
            self.load_frame(ctx);
        }
    }

    /// Fast version of set_current_image that skips histogram and overlay generation.
    /// Used during slider dragging for responsive UI.
    pub fn set_current_image_fast(&mut self, path: &std::path::Path, image: DynamicImage) {
        self.set_current_image_fast_internal(path, image, false);
    }

    /// Internal version with histogram computation control
    pub fn set_current_image_fast_internal(
        &mut self,
        path: &std::path::Path,
        image: DynamicImage,
        compute_histogram: bool,
    ) {
        crate::profiler::with_profiler(|p| p.start_timer("set_current_image_fast_total"));

        self.current_image = Some(image.clone());

        let display_input = self.prepare_display_image(&image);
        let size = [
            display_input.width() as usize,
            display_input.height() as usize,
        ];
        let texture_name = self.generate_texture_name(path, size[0], size[1]);

        // Check if texture is already cached (fast path)
        if let Some(texture) = self.get_cached_texture(&texture_name) {
            self.current_texture = Some(texture);
            self.is_loading = false;
            crate::profiler::with_profiler(|p| p.end_timer("set_current_image_fast_total"));
            return;
        }

        // Defer heavy work to background thread
        self.process_image_in_background(path, &display_input, &texture_name, compute_histogram);

        crate::profiler::with_profiler(|p| p.end_timer("set_current_image_fast_total"));
    }

    fn prepare_display_image(&mut self, image: &DynamicImage) -> DynamicImage {
        // Fast-path: when dragging, use a downscaled preview
        let mut display_input = image.clone();
        self.showing_preview = false;

        if self.slider_dragging {
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
                log::debug!(
                    "Using low-res preview {}x{} for fast adjustment rendering",
                    new_w,
                    new_h
                );
            }
        }

        display_input
    }

    fn process_image_in_background(
        &self,
        _path: &std::path::Path,
        display_input: &DynamicImage,
        texture_name: &str,
        compute_histogram: bool,
    ) {
        let ctx_clone = self.ctx.clone();
        let texture_name_clone = texture_name.to_string();
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
            let adjusted_image = if !adjustments_clone.is_default() && !show_original_clone {
                image_loader::apply_adjustments(&display_input_clone, &adjustments_clone)
            } else {
                display_input_clone.clone()
            };
            let display_image =
                if adjustments_clone.frame_enabled && adjustments_clone.frame_thickness > 0.0 {
                    let img = adjusted_image.to_rgba8();
                    let (width, height) = img.dimensions();
                    let thickness = adjustments_clone.frame_thickness as u32;
                    let new_width = width + 2 * thickness;
                    let new_height = height + 2 * thickness;

                    let mut framed = ImageBuffer::new(new_width, new_height);

                    let frame_r = (adjustments_clone.frame_color[0] * 255.0) as u8;
                    let frame_g = (adjustments_clone.frame_color[1] * 255.0) as u8;
                    let frame_b = (adjustments_clone.frame_color[2] * 255.0) as u8;

                    for pixel in framed.pixels_mut() {
                        *pixel = Rgba([frame_r, frame_g, frame_b, 255]);
                    }

                    imageops::overlay(&mut framed, &img, thickness as i64, thickness as i64);

                    DynamicImage::ImageRgba8(framed)
                } else {
                    adjusted_image.clone()
                };
            let elapsed = start.elapsed().as_millis();
            log::debug!(
                "apply_adjustments_fast worker took {} ms for preview",
                elapsed
            );

            if compute_histogram_clone {
                let hist = Self::compute_histogram_static(&adjusted_image, &gpu_clone);
                let _ = tx.send(super::LoaderMessage::HistogramUpdated(hist));
            }

            Self::create_texture_for_background(&ctx_clone, &texture_name_clone, &display_image, tx)
        });
    }

    fn compute_histogram_static(
        display_image: &DynamicImage,
        gpu: &Option<Arc<GpuProcessor>>,
    ) -> Vec<Vec<u32>> {
        if let Some(gpu) = gpu {
            match pollster::block_on(async { gpu.compute_histogram(display_image).await }) {
                Ok(h) => h,
                Err(e) => {
                    log::warn!("GPU histogram failed: {}; falling back to CPU", e);
                    image_loader::calculate_histogram(display_image)
                }
            }
        } else {
            image_loader::calculate_histogram(display_image)
        }
    }

    fn create_texture_for_background(
        ctx: &Option<egui::Context>,
        texture_name: &str,
        display_image: &DynamicImage,
        _tx: &Sender<super::LoaderMessage>,
    ) -> Option<super::LoaderMessage> {
        if let Some(ctx) = ctx {
            let rgba = display_image.to_rgba8();
            let pixels = rgba.as_flat_samples();
            let size = [
                display_image.width() as usize,
                display_image.height() as usize,
            ];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
            let texture = ctx.load_texture(
                texture_name.to_string(),
                color_image,
                egui::TextureOptions::LINEAR,
            );
            Some(super::LoaderMessage::TextureCreated(
                PathBuf::from(texture_name),
                texture,
                display_image.clone(),
            ))
        } else {
            None
        }
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

        // Submit preload tasks with appropriate priorities
        for path in full_paths {
            self.task_scheduler.submit_task(crate::task_scheduler::ImageTask::LoadImage {
                path,
                priority: crate::task_scheduler::TaskPriority::Medium,
            });
        }

        for path in thumb_paths {
            self.task_scheduler.submit_task(crate::task_scheduler::ImageTask::LoadThumbnail {
                path,
                size: 1920,
                priority: crate::task_scheduler::TaskPriority::Low,
            });
        }
    }

    /// Ensure a thumbnail is requested: short-circuit on in-memory cache or already-requested, otherwise spawn background work
    pub fn ensure_thumbnail_requested(&mut self, path: &PathBuf, _ctx: &egui::Context) {
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
            // Send result through task scheduler for consistency
            let result = crate::task_scheduler::TaskResult::ThumbnailLoaded {
                path: path.clone(),
                image: img,
            };
            // Since we're on the main thread, we need to handle this directly
            self.handle_task_result_main(result, _ctx);
            return;
        }

        // Otherwise submit task to scheduler
        self.request_thumbnail(path.clone(), _ctx.clone());
    }

    pub fn request_thumbnail(&mut self, path: PathBuf, _ctx: egui::Context) {
        if self.thumbnail_requests.contains(&path) {
            return;
        }

        // Limit concurrent thumbnail loads to prevent overwhelming the system
        if self.thumbnail_requests.len() >= 10 {
            return;
        }

        self.thumbnail_requests.insert(path.clone());

        // Use task scheduler for thumbnail loading
        self.task_scheduler.submit_task(crate::task_scheduler::ImageTask::LoadThumbnail {
            path,
            size: self.settings.thumbnail_size as u32,
            priority: crate::task_scheduler::TaskPriority::Low,
        });
    }

    /// Return a single-frame spinner character used in small UI elements (thumbnails)
    pub fn spinner_char(&self, ui: &egui::Ui) -> &'static str {
        let time = ui.input(|i| i.time);
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let idx = ((time * 10.0) as usize) % spinner_chars.len();
        spinner_chars[idx]
    }

    /// Handle task results from the task scheduler
    pub fn handle_task_result_main(&mut self, result: crate::task_scheduler::TaskResult, ctx: &egui::Context) {
        match result {
            crate::task_scheduler::TaskResult::ImageLoaded { path, image } => {
                crate::profiler::with_profiler(|p| p.increment_counter("images_loaded"));
                if self.get_current_path().as_ref() == Some(&path) {
                    self.showing_preview = false;
                    self.set_current_image(&path, image.clone());
                    self.pending_fit_to_window = true;
                } else {
                    self.image_cache.insert(path.clone(), image.clone());
                }
            }
            crate::task_scheduler::TaskResult::ThumbnailLoaded { path, image } => {
                crate::profiler::with_profiler(|p| p.increment_counter("thumbnails_loaded"));

                // Apply adjustments to thumbnail if any exist for this image
                let display_thumb = if let Some(adj) = self.metadata_db.get_adjustments(&path) {
                    if !adj.is_default() {
                        crate::image_loader::apply_adjustments_thumbnail(&image, &adj)
                    } else {
                        image
                    }
                } else {
                    image
                };

                let size = [
                    display_thumb.width() as usize,
                    display_thumb.height() as usize,
                ];
                let rgba = display_thumb.to_rgba8();
                let pixels = rgba.as_flat_samples();

                let texture = ctx.load_texture(
                    format!("thumb_{}", path.display()),
                    egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
                    egui::TextureOptions::LINEAR,
                );

                self.thumbnail_textures.insert(path.clone(), texture);
                self.thumbnail_requests.remove(&path);
            }
            crate::task_scheduler::TaskResult::ExifLoaded { path, exif } => {
                crate::profiler::with_profiler(|p| p.increment_counter("exif_loaded"));
                let exif_val = (*exif).clone();
                self.compare_exifs.insert(path.clone(), exif_val.clone());
                if self.get_current_path().as_ref() == Some(&path) {
                    self.current_exif = Some(exif_val);
                }
            }
            crate::task_scheduler::TaskResult::HistogramComputed { histogram } => {
                self.histogram_data = Some(histogram);
            }
            crate::task_scheduler::TaskResult::AdjustmentsApplied { image } => {
                self.current_image = Some(image);
                self.is_loading = false;
            }
            crate::task_scheduler::TaskResult::Error { task, error } => {
                match task {
                    crate::task_scheduler::ImageTask::LoadImage { path, .. } => {
                        crate::profiler::with_profiler(|p| p.increment_counter("load_errors"));
                        log::error!("Failed to load {}: {}", path.display(), error);
                        if self.get_current_path().as_ref() == Some(&path) {
                            self.is_loading = false;
                            self.load_error = Some(error);
                        }
                    }
                    crate::task_scheduler::ImageTask::LoadThumbnail { path, .. } => {
                        log::warn!("Thumbnail load failed for {:?}: {}", path, error);
                        self.thumbnail_requests.remove(&path);
                    }
                    crate::task_scheduler::ImageTask::LoadExif { path, .. } => {
                        log::warn!("EXIF load failed for {:?}: {}", path, error);
                    }
                    _ => {
                        log::warn!("Task failed: {}", error);
                    }
                }
            }
        }
    }
}
