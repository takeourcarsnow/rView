use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // Appearance
    pub theme: Theme,
    pub background_color: BackgroundColor,
    pub accent_color: AccentColor,
    
    // Panels
    pub show_sidebar: bool,
    pub show_thumbnails: bool,
    pub thumbnail_size: f32,
    pub thumbnail_position: ThumbnailPosition,
    pub show_exif: bool,
    pub show_histogram: bool,
    pub show_minimap: bool,
    pub show_toolbar: bool,
    pub show_statusbar: bool,
    
    // Zoom behavior
    pub zoom_increment: f32,
    pub smooth_zoom: bool,
    pub zoom_animation_speed: f32,
    pub maintain_zoom_on_navigate: bool,
    pub maintain_pan_on_navigate: bool,
    
    // View modes
    pub fit_mode: FitMode,
    pub auto_rotate_exif: bool,
    
    // Slideshow
    pub slideshow_interval: f32,
    pub slideshow_loop: bool,
    pub slideshow_random: bool,
    
    // Overlays
    pub show_focus_peaking: bool,
    pub focus_peaking_color: FocusPeakingColor,
    pub focus_peaking_threshold: f32,
    pub show_zebras: bool,
    pub zebra_high_threshold: u8,
    pub zebra_low_threshold: u8,
    pub show_grid_overlay: bool,
    pub grid_type: GridType,
    
    // Sorting and filtering
    pub sort_mode: SortMode,
    pub sort_order: SortOrder,
    pub sort_ascending: bool,
    pub include_subfolders: bool,
    pub filter_by_rating: Option<u8>,
    pub filter_by_color: Option<ColorLabel>,
    
    // File management
    pub recent_folders: Vec<PathBuf>,
    pub max_recent_folders: usize,
    pub favorite_folders: Vec<PathBuf>,
    pub quick_move_folders: Vec<PathBuf>,
    pub external_editors: Vec<ExternalEditor>,
    pub confirm_delete: bool,
    pub delete_to_trash: bool,
    
    // Cache and performance
    pub preload_adjacent: usize,
    pub cache_size_mb: usize,
    pub thumbnail_cache_size: usize,
    pub use_embedded_thumbnails: bool,
    pub parallel_thumbnail_threads: usize,
    
    // Export presets
    pub export_presets: Vec<ExportPreset>,
    pub default_export_preset: Option<String>,
    
    // Keyboard shortcuts
    pub shortcuts: HashMap<String, KeyShortcut>,
    
    // Window state
    pub window_maximized: bool,
    pub window_size: (f32, f32),
    pub window_position: Option<(f32, f32)>,
    
    // Session
    pub restore_session: bool,
    pub last_folder: Option<PathBuf>,
    pub last_file: Option<PathBuf>,
    
    // Collections/Albums
    pub collections: Vec<Collection>,
    
    // Duplicate detection
    pub duplicate_threshold: f32,
    
    // Loupe
    pub loupe_size: f32,
    pub loupe_zoom: f32,
    pub loupe_enabled: bool,
    
    // Adjustments
    pub show_adjustments: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            background_color: BackgroundColor::Dark,
            accent_color: AccentColor::Blue,
            
            show_sidebar: true,
            show_thumbnails: true,
            thumbnail_size: 100.0,
            thumbnail_position: ThumbnailPosition::Bottom,
            show_exif: true,
            show_histogram: false,
            show_minimap: false,
            show_toolbar: true,
            show_statusbar: true,
            
            zoom_increment: 0.1,
            smooth_zoom: true,
            zoom_animation_speed: 8.0,
            maintain_zoom_on_navigate: true,
            maintain_pan_on_navigate: true,
            
            fit_mode: FitMode::Fit,
            auto_rotate_exif: true,
            
            slideshow_interval: 3.0,
            slideshow_loop: true,
            slideshow_random: false,
            
            show_focus_peaking: false,
            focus_peaking_color: FocusPeakingColor::Red,
            focus_peaking_threshold: 50.0,
            show_zebras: false,
            zebra_high_threshold: 250,
            zebra_low_threshold: 5,
            show_grid_overlay: false,
            grid_type: GridType::RuleOfThirds,
            
            sort_mode: SortMode::Name,
            sort_order: SortOrder::Ascending,
            sort_ascending: true,
            include_subfolders: false,
            filter_by_rating: None,
            filter_by_color: None,
            
            recent_folders: Vec::new(),
            max_recent_folders: 20,
            favorite_folders: Vec::new(),
            quick_move_folders: Vec::new(),
            external_editors: Vec::new(),
            confirm_delete: true,
            delete_to_trash: true,
            
            preload_adjacent: 3,
            cache_size_mb: 1024,
            thumbnail_cache_size: 1000,
            use_embedded_thumbnails: true,
            parallel_thumbnail_threads: 4,
            
            export_presets: vec![
                ExportPreset::default_web(),
                ExportPreset::default_print(),
                ExportPreset::default_social(),
            ],
            default_export_preset: Some("Web".to_string()),
            
            shortcuts: default_shortcuts(),
            
            window_maximized: false,
            window_size: (1400.0, 900.0),
            window_position: None,
            
            restore_session: true,
            last_folder: None,
            last_file: None,
            
            collections: Vec::new(),
            
            duplicate_threshold: 0.95,
            
            loupe_size: 200.0,
            loupe_zoom: 2.0,
            loupe_enabled: false,
            
            show_adjustments: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
    OLED,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackgroundColor {
    Dark,
    Light,
    Gray,
    Checkered,
    Black,
}

impl BackgroundColor {
    pub fn to_color(&self) -> egui::Color32 {
        match self {
            BackgroundColor::Dark => egui::Color32::from_rgb(18, 18, 20),
            BackgroundColor::Light => egui::Color32::from_rgb(245, 245, 247),
            BackgroundColor::Gray => egui::Color32::from_rgb(80, 80, 85),
            BackgroundColor::Checkered => egui::Color32::from_rgb(40, 40, 42),
            BackgroundColor::Black => egui::Color32::BLACK,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccentColor {
    Blue,
    Purple,
    Green,
    Orange,
    Red,
    Pink,
    Cyan,
}

impl AccentColor {
    pub fn to_color(&self) -> egui::Color32 {
        match self {
            AccentColor::Blue => egui::Color32::from_rgb(70, 130, 255),
            AccentColor::Purple => egui::Color32::from_rgb(160, 90, 255),
            AccentColor::Green => egui::Color32::from_rgb(50, 205, 100),
            AccentColor::Orange => egui::Color32::from_rgb(255, 150, 50),
            AccentColor::Red => egui::Color32::from_rgb(255, 80, 80),
            AccentColor::Pink => egui::Color32::from_rgb(255, 100, 180),
            AccentColor::Cyan => egui::Color32::from_rgb(50, 200, 220),
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
    DateTaken,
    Size,
    Type,
    Rating,
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThumbnailPosition {
    Bottom,
    Left,
    Right,
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FocusPeakingColor {
    Red,
    Green,
    Blue,
    Yellow,
    White,
}

impl FocusPeakingColor {
    pub fn to_color(&self) -> egui::Color32 {
        match self {
            FocusPeakingColor::Red => egui::Color32::from_rgb(255, 0, 0),
            FocusPeakingColor::Green => egui::Color32::from_rgb(0, 255, 0),
            FocusPeakingColor::Blue => egui::Color32::from_rgb(0, 100, 255),
            FocusPeakingColor::Yellow => egui::Color32::from_rgb(255, 255, 0),
            FocusPeakingColor::White => egui::Color32::WHITE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridType {
    RuleOfThirds,
    GoldenRatio,
    Diagonal,
    Center,
    Square,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, Default)]
pub enum ColorLabel {
    #[default]
    None,
    Red,
    Yellow,
    Green,
    Blue,
    Purple,
}

impl ColorLabel {
    pub fn to_color(&self) -> egui::Color32 {
        match self {
            ColorLabel::None => egui::Color32::TRANSPARENT,
            ColorLabel::Red => egui::Color32::from_rgb(255, 80, 80),
            ColorLabel::Yellow => egui::Color32::from_rgb(255, 220, 50),
            ColorLabel::Green => egui::Color32::from_rgb(80, 220, 80),
            ColorLabel::Blue => egui::Color32::from_rgb(80, 150, 255),
            ColorLabel::Purple => egui::Color32::from_rgb(180, 100, 255),
        }
    }
    
    pub fn all() -> &'static [ColorLabel] {
        &[
            ColorLabel::None,
            ColorLabel::Red,
            ColorLabel::Yellow,
            ColorLabel::Green,
            ColorLabel::Blue,
            ColorLabel::Purple,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalEditor {
    pub name: String,
    pub path: PathBuf,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPreset {
    pub name: String,
    pub format: ExportFormat,
    pub quality: u8,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub suffix: String,
}

impl ExportPreset {
    pub fn default_web() -> Self {
        Self {
            name: "Web".to_string(),
            format: ExportFormat::Jpeg,
            quality: 85,
            max_width: Some(2048),
            max_height: Some(2048),
            suffix: "_web".to_string(),
        }
    }
    
    pub fn default_print() -> Self {
        Self {
            name: "Print".to_string(),
            format: ExportFormat::Jpeg,
            quality: 95,
            max_width: None,
            max_height: None,
            suffix: "_print".to_string(),
        }
    }
    
    pub fn default_social() -> Self {
        Self {
            name: "Social".to_string(),
            format: ExportFormat::Jpeg,
            quality: 80,
            max_width: Some(1080),
            max_height: Some(1080),
            suffix: "_social".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Jpeg,
    Png,
    WebP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyShortcut {
    pub key: String,
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub images: Vec<PathBuf>,
    pub created: String,
    pub modified: String,
}

impl Collection {
    pub fn new(name: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            images: Vec::new(),
            created: now.clone(),
            modified: now,
        }
    }
}

fn default_shortcuts() -> HashMap<String, KeyShortcut> {
    let mut shortcuts = HashMap::new();
    
    shortcuts.insert("next_image".to_string(), KeyShortcut { key: "Right".to_string(), modifiers: vec![] });
    shortcuts.insert("prev_image".to_string(), KeyShortcut { key: "Left".to_string(), modifiers: vec![] });
    shortcuts.insert("first_image".to_string(), KeyShortcut { key: "Home".to_string(), modifiers: vec![] });
    shortcuts.insert("last_image".to_string(), KeyShortcut { key: "End".to_string(), modifiers: vec![] });
    shortcuts.insert("zoom_in".to_string(), KeyShortcut { key: "Plus".to_string(), modifiers: vec![] });
    shortcuts.insert("zoom_out".to_string(), KeyShortcut { key: "Minus".to_string(), modifiers: vec![] });
    shortcuts.insert("zoom_fit".to_string(), KeyShortcut { key: "0".to_string(), modifiers: vec![] });
    shortcuts.insert("zoom_100".to_string(), KeyShortcut { key: "1".to_string(), modifiers: vec![] });
    shortcuts.insert("delete".to_string(), KeyShortcut { key: "Delete".to_string(), modifiers: vec![] });
    shortcuts.insert("fullscreen".to_string(), KeyShortcut { key: "F11".to_string(), modifiers: vec![] });
    shortcuts.insert("slideshow".to_string(), KeyShortcut { key: "Space".to_string(), modifiers: vec![] });
    shortcuts.insert("rotate_left".to_string(), KeyShortcut { key: "L".to_string(), modifiers: vec![] });
    shortcuts.insert("rotate_right".to_string(), KeyShortcut { key: "R".to_string(), modifiers: vec![] });
    
    shortcuts
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
