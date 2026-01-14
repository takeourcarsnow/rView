use crate::image_loader::{is_supported_image, SUPPORTED_EXTENSIONS};
use eframe::egui;
use image::DynamicImage;
use std::path::PathBuf;
use std::sync::Arc;
use walkdir::WalkDir;

use super::ImageViewerApp;

#[allow(dead_code)]
impl ImageViewerApp {
    // File dialogs
    pub fn open_file_dialog(&mut self) {
        let extensions: Vec<&str> = SUPPORTED_EXTENSIONS.to_vec();

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &extensions)
            .pick_file()
        {
            self.load_image_file(path);
        }
    }

    pub fn open_folder_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.load_folder(path);
        }
    }

    pub fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());

        for file in dropped {
            if let Some(path) = &file.path {
                if path.is_file() && is_supported_image(path) {
                    self.load_image_file(path.clone());
                    break;
                } else if path.is_dir() {
                    self.load_folder(path.clone());
                    break;
                }
            }
        }
    }

    pub fn open_in_file_manager(&self) {
        if let Some(path) = self.get_current_path() {
            let _ = open::that(path.parent().unwrap_or(&path));
        }
    }

    pub fn open_in_external_editor(&self, editor_path: &std::path::Path) {
        if let Some(path) = self.get_current_path() {
            let _ = std::process::Command::new(editor_path).arg(&path).spawn();
        }
    }

    pub fn set_as_wallpaper(&self) {
        if let Some(path) = self.get_current_path() {
            let _ = wallpaper::set_from_path(path.to_string_lossy().as_ref());
            // self.show_status("Set as wallpaper");
        }
    }

    pub fn copy_to_clipboard(&self) {
        if let Some(path) = self.get_current_path() {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(path.display().to_string());
            }
        }
    }

    pub fn export_image(&mut self) {
        if let Some(image) = &self.current_image {
            let extensions = vec!["jpg", "jpeg", "png", "bmp", "tiff", "tif"];

            // Generate default filename based on current image path
            let default_filename = if let Some(current_path) = self.get_current_path() {
                if let Some(file_stem) = current_path.file_stem() {
                    if let Some(extension) = current_path.extension() {
                        format!(
                            "{}_rView.{}",
                            file_stem.to_string_lossy(),
                            extension.to_string_lossy()
                        )
                    } else {
                        format!("{}_rView.jpg", file_stem.to_string_lossy())
                    }
                } else {
                    "exported_image_rView.jpg".to_string()
                }
            } else {
                "exported_image_rView.jpg".to_string()
            };

            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Images", &extensions)
                .set_file_name(&default_filename)
                .save_file()
            {
                // Apply current adjustments to the image before saving
                let image_to_save = if !self.adjustments.is_default() && !self.show_original {
                    // Use CPU for frame processing since GPU doesn't support it yet
                    if self.adjustments.frame_enabled {
                        crate::image_loader::apply_adjustments(image, &self.adjustments)
                    } else if let Some(gpu) = &self.gpu_processor {
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
                                    "GPU texture export failed: {}; falling back to buffer method",
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
                                            crate::image_loader::apply_adjustments(
                                                &image_clone,
                                                &adjustments_clone,
                                            )
                                        }
                                    }
                                    Err(_) => crate::image_loader::apply_adjustments(
                                        &image_clone,
                                        &adjustments_clone,
                                    ),
                                }
                            }
                        }
                    } else {
                        crate::image_loader::apply_adjustments(image, &self.adjustments)
                    }
                } else {
                    image.clone()
                };

                match image_to_save.save(&path) {
                    Ok(_) => {
                        self.show_status(&format!("Exported to {}", path.display()));
                    }
                    Err(e) => {
                        self.show_status(&format!("Failed to export image: {}", e));
                    }
                }
            }
        } else {
            self.show_status("No image to export");
        }
    }

    pub fn toggle_panels(&mut self) {
        self.panels_hidden = !self.panels_hidden;
        // Schedule a fit operation for the next frame after UI layout is updated
        self.pending_fit_to_window = true;
    }

    // Load a folder (tabs removed) - populate app image list directly
    pub fn load_folder(&mut self, folder: PathBuf) {
        self.current_folder = Some(folder.clone());
        self.settings.add_recent_folder(folder.clone());

        self.image_list.clear();
        self.thumbnail_textures.clear();
        self.thumbnail_requests.clear();

        if self.settings.include_subfolders {
            for entry in WalkDir::new(&folder)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path().to_path_buf();
                if path.is_file() && is_supported_image(&path) {
                    self.image_list.push(path);
                }
            }
        } else if let Ok(entries) = std::fs::read_dir(&folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && is_supported_image(&path) {
                    self.image_list.push(path);
                }
            }
        }

        self.sort_images();
        self.apply_filter();

        if !self.filtered_list.is_empty() {
            self.current_index = 0;
            // Load adjustments for the first image
            self.load_adjustments_for_current();
            self.load_current_image();
        }

        self.show_status(&format!("Loaded {} images", self.image_list.len()));
    }
}
