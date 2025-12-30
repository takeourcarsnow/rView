use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: Theme,
    pub background_color: BackgroundColor,
    pub show_sidebar: bool,
    pub show_thumbnails: bool,
    pub thumbnail_size: f32,
    pub zoom_increment: f32,
    pub smooth_zoom: bool,
    pub maintain_zoom_on_navigate: bool,
    pub maintain_pan_on_navigate: bool,
    pub slideshow_interval: f32,
    pub recent_folders: Vec<PathBuf>,
    pub max_recent_folders: usize,
    pub preload_adjacent: usize,
    pub cache_size_mb: usize,
    pub show_exif: bool,
    pub show_histogram: bool,
    pub fit_mode: FitMode,
    pub sort_mode: SortMode,
    pub sort_ascending: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            background_color: BackgroundColor::Dark,
            show_sidebar: true,
            show_thumbnails: true,
            thumbnail_size: 120.0,
            zoom_increment: 0.1,
            smooth_zoom: true,
            maintain_zoom_on_navigate: true,  // Key feature!
            maintain_pan_on_navigate: true,   // Key feature!
            slideshow_interval: 3.0,
            recent_folders: Vec::new(),
            max_recent_folders: 10,
            preload_adjacent: 2,
            cache_size_mb: 512,
            show_exif: false,
            show_histogram: false,
            fit_mode: FitMode::Fit,
            sort_mode: SortMode::Name,
            sort_ascending: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackgroundColor {
    Dark,
    Light,
    Gray,
    Checkered,
}

impl BackgroundColor {
    pub fn to_color(&self) -> egui::Color32 {
        match self {
            BackgroundColor::Dark => egui::Color32::from_rgb(18, 18, 20),
            BackgroundColor::Light => egui::Color32::from_rgb(245, 245, 247),
            BackgroundColor::Gray => egui::Color32::from_rgb(80, 80, 85),
            BackgroundColor::Checkered => egui::Color32::from_rgb(40, 40, 42),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FitMode {
    Fit,
    Fill,
    OneToOne,
    FitWidth,
    FitHeight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortMode {
    Name,
    Date,
    Size,
    Type,
}

impl Settings {
    pub fn load() -> Self {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "imageviewer", "ImageViewer") {
            let config_path = proj_dirs.config_dir().join("settings.json");
            if config_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&config_path) {
                    if let Ok(settings) = serde_json::from_str(&content) {
                        return settings;
                    }
                }
            }
        }
        Self::default()
    }
    
    pub fn save(&self) {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "imageviewer", "ImageViewer") {
            let config_dir = proj_dirs.config_dir();
            let _ = std::fs::create_dir_all(config_dir);
            let config_path = config_dir.join("settings.json");
            if let Ok(content) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(config_path, content);
            }
        }
    }
    
    pub fn add_recent_folder(&mut self, path: PathBuf) {
        self.recent_folders.retain(|p| p != &path);
        self.recent_folders.insert(0, path);
        if self.recent_folders.len() > self.max_recent_folders {
            self.recent_folders.truncate(self.max_recent_folders);
        }
    }
}
