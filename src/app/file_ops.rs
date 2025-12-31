use crate::image_loader::{SUPPORTED_EXTENSIONS, is_supported_image};
use eframe::egui;
use std::path::PathBuf;
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

    // Tab management methods
    pub fn create_tab(&mut self, folder_path: PathBuf) {
        let tab = super::ImageTab::new(folder_path.clone());
        self.tabs.push(tab);
        self.current_tab = self.tabs.len() - 1;

        // Load the folder for the new tab
        self.load_folder_for_current_tab(folder_path);
    }

    pub fn switch_to_tab(&mut self, tab_index: usize) {
        if tab_index < self.tabs.len() {
            // Save current tab state: capture snapshot then write back
            let folder = self.tabs[self.current_tab].folder_path.clone();
            let new_tab = super::ImageTab::capture_from_app(self, folder);
            self.tabs[self.current_tab] = new_tab;

            // Switch to new tab
            self.current_tab = tab_index;
            let tab = self.tabs[tab_index].clone();
            tab.apply_to_app(self);

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