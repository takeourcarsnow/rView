use crate::image_loader::{self, SUPPORTED_EXTENSIONS, is_supported_image};
use crate::settings::ExportFormat;
use eframe::egui;
use std::path::PathBuf;
use uuid;
use walkdir::WalkDir;

use super::ImageViewerApp;

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

    pub fn open_in_external_editor(&self, editor_path: &PathBuf) {
        if let Some(path) = self.get_current_path() {
            let _ = std::process::Command::new(editor_path)
                .arg(&path)
                .spawn();
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

    pub fn toggle_compare_mode(&mut self) {
        if self.view_mode == super::ViewMode::Compare {
            self.view_mode = super::ViewMode::Single;
            self.compare_index = None;
        } else {
            self.view_mode = super::ViewMode::Compare;
            self.compare_index = Some(self.current_index);
        }
    }

    pub fn toggle_lightbox_mode(&mut self) {
        if self.view_mode == super::ViewMode::Lightbox {
            self.view_mode = super::ViewMode::Single;
        } else {
            self.view_mode = super::ViewMode::Lightbox;
        }
    }

    pub fn toggle_panels(&mut self) {
        self.panels_hidden = !self.panels_hidden;
        // Schedule a fit operation for the next frame after UI layout is updated
        self.pending_fit_to_window = true;
    }

    // Export
    pub fn export_current(&mut self, preset_name: &str) {
        if let (Some(image), Some(path)) = (&self.current_image, self.get_current_path()) {
            if let Some(preset) = self.settings.export_presets.iter().find(|p| p.name == preset_name) {
                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                let ext = match preset.format {
                    ExportFormat::Jpeg => "jpg",
                    ExportFormat::Png => "png",
                    ExportFormat::WebP => "webp",
                };

                let output_name = format!("{}{}.{}", stem, preset.suffix, ext);
                let output_path = path.parent().unwrap_or(&path).join(output_name);

                if image_loader::export_image(
                    image,
                    &output_path,
                    preset.format,
                    preset.quality,
                    preset.max_width,
                    preset.max_height,
                ).is_ok() {
                    self.show_status(&format!("Exported to {}", output_path.display()));
                }
            }
        }
    }

    // Tab management methods
    pub fn create_tab(&mut self, folder_path: PathBuf) {
        let tab_name = folder_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "New Tab".to_string());

        let tab = super::ImageTab {
            id: uuid::Uuid::new_v4().to_string(),
            name: tab_name,
            folder_path: folder_path.clone(),
            image_list: Vec::new(),
            filtered_list: Vec::new(),
            current_index: 0,
            zoom: 1.0,
            target_zoom: 1.0,
            pan_offset: egui::Vec2::ZERO,
            target_pan: egui::Vec2::ZERO,
            rotation: 0.0,
            adjustments: image_loader::ImageAdjustments::default(),
            show_original: false,
            view_mode: super::ViewMode::Single,
            compare_index: None,
            lightbox_columns: 4,
            selected_indices: std::collections::HashSet::new(),
            last_selected: None,
            search_query: String::new(),
        };

        self.tabs.push(tab);
        self.current_tab = self.tabs.len() - 1;

        // Load the folder for the new tab
        self.load_folder_for_current_tab(folder_path);
    }

    pub fn switch_to_tab(&mut self, tab_index: usize) {
        if tab_index < self.tabs.len() {
            // Save current tab state
            if let Some(current_tab) = self.tabs.get_mut(self.current_tab) {
                current_tab.image_list = self.image_list.clone();
                current_tab.filtered_list = self.filtered_list.clone();
                current_tab.current_index = self.current_index;
                current_tab.zoom = self.zoom;
                current_tab.target_zoom = self.target_zoom;
                current_tab.pan_offset = self.pan_offset;
                current_tab.target_pan = self.target_pan;
                current_tab.rotation = self.rotation;
                current_tab.adjustments = self.adjustments.clone();
                current_tab.show_original = self.show_original;
                current_tab.view_mode = self.view_mode;
                current_tab.compare_index = self.compare_index;
                current_tab.lightbox_columns = self.lightbox_columns;
                current_tab.selected_indices = self.selected_indices.clone();
                current_tab.last_selected = self.last_selected;
                current_tab.search_query = self.search_query.clone();
            }

            // Switch to new tab
            self.current_tab = tab_index;
            let tab = &self.tabs[tab_index];

            // Restore tab state
            self.image_list = tab.image_list.clone();
            self.filtered_list = tab.filtered_list.clone();
            self.current_index = tab.current_index;
            self.zoom = tab.zoom;
            self.target_zoom = tab.target_zoom;
            self.pan_offset = tab.pan_offset;
            self.target_pan = tab.target_pan;
            self.rotation = tab.rotation;
            self.adjustments = tab.adjustments.clone();
            self.show_original = tab.show_original;
            self.view_mode = tab.view_mode;
            self.compare_index = tab.compare_index;
            self.lightbox_columns = tab.lightbox_columns;
            self.selected_indices = tab.selected_indices.clone();
            self.last_selected = tab.last_selected;
            self.search_query = tab.search_query.clone();
            self.current_folder = Some(tab.folder_path.clone());

            // Load current image
            self.load_current_image();
        }
    }

    pub fn close_tab(&mut self, tab_index: usize) {
        if self.tabs.len() > 1 && tab_index < self.tabs.len() {
            self.tabs.remove(tab_index);

            if self.current_tab >= tab_index && self.current_tab > 0 {
                self.current_tab -= 1;
            } else if self.tabs.is_empty() {
                self.current_tab = 0;
            }

            // Switch to the current tab
            if !self.tabs.is_empty() {
                self.switch_to_tab(self.current_tab);
            }
        }
    }

    pub fn load_folder_for_current_tab(&mut self, folder: PathBuf) {
        if let Some(tab) = self.tabs.get_mut(self.current_tab) {
            tab.folder_path = folder.clone();
            tab.name = folder.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "New Tab".to_string());
        }

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
            self.load_current_image();
        }

        self.show_status(&format!("Loaded {} images", self.image_list.len()));
    }
}