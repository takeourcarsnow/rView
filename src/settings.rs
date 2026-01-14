use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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
    pub show_thumbnail_labels: bool,
    pub show_exif: bool,
    /// Whether the small EXIF overlay on the image is visible (separate from the sidebar)
    pub show_exif_overlay: bool,
    pub show_histogram: bool,
    pub show_adjustments: bool,
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

    // GPU acceleration
    pub gpu_enabled: bool,

    // Telemetry
    pub telemetry_enabled: bool,

    // Export presets

    // Window state
    pub window_maximized: bool,
    pub window_size: (f32, f32),
    pub window_position: Option<(f32, f32)>,

    // Session
    pub restore_session: bool,
    pub last_folder: Option<PathBuf>,
    pub last_file: Option<PathBuf>,

    // Loupe
    pub loupe_size: f32,
    pub loupe_zoom: f32,
    /// If false, RAW files will not be decoded to full resolution; only embedded JPEG previews will be used
    pub load_raw_full_size: bool,
    pub loupe_enabled: bool,
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
            show_thumbnail_labels: false,
            show_exif: true,
            show_exif_overlay: true,
            show_histogram: true,
            show_adjustments: true,
            show_toolbar: true,
            show_statusbar: true,

            zoom_increment: 0.1,
            smooth_zoom: true,
            zoom_animation_speed: 8.0,
            maintain_zoom_on_navigate: true,
            maintain_pan_on_navigate: true,

            fit_mode: FitMode::Fit,
            auto_rotate_exif: true,

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

            // GPU
            gpu_enabled: true,

            // Telemetry (disabled by default)
            telemetry_enabled: false,

            window_maximized: false,
            window_size: (1400.0, 900.0),
            window_position: None,

            restore_session: true,
            last_folder: None,
            last_file: None,

            loupe_size: 200.0,
            loupe_zoom: 2.0,
            load_raw_full_size: true,
            loupe_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
    Oled,
    System,
    SolarizedDark,
    SolarizedLight,
    HighContrast,
    Blue,
    Purple,
    Green,
    Warm,
    Cool,
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
    pub fn to_color(self) -> egui::Color32 {
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
    pub fn to_color(self) -> egui::Color32 {
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
    #[allow(dead_code)]
    pub fn to_color(self) -> egui::Color32 {
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
    pub fn to_color(self) -> egui::Color32 {
        match self {
            ColorLabel::None => egui::Color32::TRANSPARENT,
            ColorLabel::Red => egui::Color32::from_rgb(255, 80, 80),
            ColorLabel::Yellow => egui::Color32::from_rgb(255, 220, 50),
            ColorLabel::Green => egui::Color32::from_rgb(80, 220, 80),
            ColorLabel::Blue => egui::Color32::from_rgb(80, 150, 255),
            ColorLabel::Purple => egui::Color32::from_rgb(180, 100, 255),
        }
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            ColorLabel::None => "None",
            ColorLabel::Red => "Red",
            ColorLabel::Yellow => "Yellow",
            ColorLabel::Green => "Green",
            ColorLabel::Blue => "Blue",
            ColorLabel::Purple => "Purple",
        }
    }

    #[allow(dead_code)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum PanelPosition {
    Left,
    Right,
    Top,
    Bottom,
    Hidden,
}

#[allow(dead_code)]
fn default_panel_positions() -> HashMap<String, PanelPosition> {
    let mut positions = HashMap::new();
    positions.insert("sidebar".to_string(), PanelPosition::Right);
    positions.insert("thumbnails".to_string(), PanelPosition::Bottom);
    positions.insert("toolbar".to_string(), PanelPosition::Top);
    positions.insert("statusbar".to_string(), PanelPosition::Bottom);
    positions
}

impl Settings {
    pub fn load() -> Self {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "imageviewer", "ImageViewer")
        {
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
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "imageviewer", "ImageViewer")
        {
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

    pub fn add_quick_move_folder(&mut self, path: PathBuf) {
        self.quick_move_folders.retain(|p| p != &path);
        self.quick_move_folders.insert(0, path);
        if self.quick_move_folders.len() > 10 {
            // Keep max 10 quick move folders
            self.quick_move_folders.truncate(10);
        }
    }
}
