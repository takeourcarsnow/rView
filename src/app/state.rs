use crate::image_cache::ImageCache;
use crate::image_loader::{ImageAdjustments};
use crate::settings::Settings;
use crate::exif_data::ExifInfo;
use crate::metadata::{MetadataDb, UndoHistory};
use crate::profiler::{CacheStats, LoadingDiagnostics};

use eframe::egui::{self, TextureHandle, Vec2};
use image::DynamicImage;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use crate::gpu::GpuProcessor;

#[derive(Debug)]
pub enum LoaderMessage {
    ImageLoaded(PathBuf, DynamicImage),
    PreviewLoaded(PathBuf, DynamicImage),
    ProgressiveLoaded(PathBuf, DynamicImage),
    ThumbnailLoaded(PathBuf, DynamicImage),
    ThumbnailRequestComplete(PathBuf),
    LoadError(PathBuf, String),
    ExifLoaded(PathBuf, Box<ExifInfo>),
    MoveCompleted { from: PathBuf, dest_folder: PathBuf, success: bool, error: Option<String> },
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Single,
    Lightbox,
    Compare,
}

pub struct ImageViewerApp {
    // Settings
    pub settings: Settings,

    // Metadata database
    pub metadata_db: MetadataDb,

    // Tabs removed

    // Current tab's data (for backward compatibility)
    pub image_list: Vec<PathBuf>,
    pub filtered_list: Vec<usize>, // Indices into image_list
    pub current_index: usize,
    pub current_folder: Option<PathBuf>,

    // Multi-selection
    pub selected_indices: HashSet<usize>,

    // Current image state
    pub current_texture: Option<TextureHandle>,
    pub current_image: Option<DynamicImage>,
    pub current_exif: Option<ExifInfo>,
    pub histogram_data: Option<Vec<Vec<u32>>>,

    // EXIF data cached for arbitrary paths (used for compare and overlays)
    pub compare_exifs: std::collections::HashMap<PathBuf, ExifInfo>,
    pub is_loading: bool,
    pub load_error: Option<String>,
    /// Tracks if we're showing a preview (not full resolution)
    pub showing_preview: bool,

    // Overlays
    pub focus_peaking_texture: Option<TextureHandle>,
    pub zebra_texture: Option<TextureHandle>,

    // View state
    pub zoom: f32,
    pub target_zoom: f32,
    pub pan_offset: Vec2,
    pub target_pan: Vec2,
    pub rotation: f32,
    pub available_view_size: Vec2, // Available space for image display

    // Adjustments
    pub adjustments: ImageAdjustments,
    pub current_film_preset: crate::image_loader::FilmPreset,
    pub show_original: bool, // Before/After toggle
    pub last_adjustment_time: std::time::Instant,

    // Cached data
    pub image_cache: Arc<ImageCache>,
    pub thumbnail_textures: HashMap<PathBuf, egui::TextureHandle>,
    pub thumbnail_requests: HashSet<PathBuf>,
    pub compare_large_preview_requests: HashSet<PathBuf>,

    // File tree state
    pub expanded_dirs: HashSet<PathBuf>,

    // Async loading
    pub loader_tx: Sender<LoaderMessage>,
    pub loader_rx: Receiver<LoaderMessage>,

    // Slideshow
    pub slideshow_active: bool,
    pub slideshow_timer: f32,

    // Fullscreen
    pub is_fullscreen: bool,

    // View modes
    pub view_mode: ViewMode,

    // Dialogs
    pub show_settings_dialog: bool,
    pub show_go_to_dialog: bool,
    pub show_move_dialog: bool,
    pub go_to_input: String,
    pub search_query: String,
    pub search_visible: bool,
    pub command_palette_open: bool,
    pub command_palette_query: String,

    // Pending navigation actions (deferred to avoid UI blocking)
    pub pending_navigate_next: bool,
    pub pending_navigate_prev: bool,
    pub pending_navigate_first: bool,
    pub pending_navigate_last: bool,
    pub pending_navigate_page_up: bool,
    pub pending_navigate_page_down: bool,
    pub pending_fit_to_window: bool,

    // Undo history
    pub undo_history: UndoHistory,

    // Mouse state
    pub loupe_position: Option<egui::Pos2>,
    pub picked_color: Option<(u8, u8, u8)>,

    // Context for repaint requests
    pub ctx: Option<egui::Context>,

    // GPU processor (optional)
    pub gpu_processor: Option<Arc<GpuProcessor>>,

    // Status message
    pub status_message: Option<(String, std::time::Instant)>,

    // Profiler and diagnostics
    pub profiler_enabled: bool,
    pub cache_stats: CacheStats,
    pub loading_diagnostics: LoadingDiagnostics,

    // Panel visibility
    pub panels_hidden: bool,

    // Panel collapse states
    pub sidebar_collapsed: bool,
    pub thumbnail_collapsed: bool,

    // Compare view interaction state (zoom per side)
    pub compare_zoom: [f32; 2],
    pub compare_pan: [egui::Vec2; 2],

    // GPU initialization state
    pub gpu_initialization_attempted: bool,
}

impl ImageViewerApp {
    pub fn set_status_message(&mut self, msg: String) {
        self.status_message = Some((msg, std::time::Instant::now()));
    }

    pub fn should_apply_adjustments(&mut self) -> bool {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_adjustment_time);
        if elapsed.as_millis() >= 100 { // 100ms debounce
            self.last_adjustment_time = now;
            true
        } else {
            false
        }
    }
}

impl ImageViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);

        let (tx, rx) = channel();

        let settings = Settings::load();
        let metadata_db = MetadataDb::load();

        let mut app = Self {
            settings,
            metadata_db,
            image_list: Vec::new(),
            filtered_list: Vec::new(),
            current_index: 0,
            current_folder: None,
            selected_indices: HashSet::new(),
            current_texture: None,
            current_image: None,
            current_exif: None,
            histogram_data: None,
            is_loading: false,
            load_error: None,
            showing_preview: false,
            focus_peaking_texture: None,
            zebra_texture: None,
            compare_exifs: HashMap::new(),
            compare_large_preview_requests: HashSet::new(),
            zoom: 1.0,
            target_zoom: 1.0,
            pan_offset: Vec2::ZERO,
            target_pan: Vec2::ZERO,
            rotation: 0.0,
            available_view_size: Vec2::new(800.0, 600.0), // Default fallback
            adjustments: ImageAdjustments::default(),
            current_film_preset: crate::image_loader::FilmPreset::None,
            show_original: false,
            last_adjustment_time: std::time::Instant::now(),
            image_cache: Arc::new(ImageCache::new(1024)),
            thumbnail_textures: HashMap::new(),
            thumbnail_requests: HashSet::new(),
            expanded_dirs: HashSet::new(),
            loader_tx: tx,
            loader_rx: rx,
            slideshow_active: false,
            slideshow_timer: 0.0,
            is_fullscreen: false,
            view_mode: ViewMode::Single,
            show_settings_dialog: false,
            show_go_to_dialog: false,
            show_move_dialog: false,
            go_to_input: String::new(),
            search_query: String::new(),
            search_visible: false,
            command_palette_open: false,
            command_palette_query: String::new(),
            pending_navigate_next: false,
            pending_navigate_prev: false,
            pending_navigate_first: false,
            pending_navigate_last: false,
            pending_navigate_page_up: false,
            pending_navigate_page_down: false,
            pending_fit_to_window: false,
            undo_history: UndoHistory::new(50),
            loupe_position: None,
            picked_color: None,
            ctx: Some(cc.egui_ctx.clone()),
            gpu_processor: None,
            compare_zoom: [1.0, 1.0],
            compare_pan: [Vec2::ZERO, Vec2::ZERO],
            status_message: None,
            profiler_enabled: cfg!(debug_assertions), // Enabled in debug mode
            cache_stats: CacheStats::default(),
            loading_diagnostics: LoadingDiagnostics::default(),
            panels_hidden: false,
            sidebar_collapsed: false,
            thumbnail_collapsed: false,
            gpu_initialization_attempted: false,
        };

        // Restore session
        if app.settings.restore_session {
            if let Some(ref folder) = app.settings.last_folder.clone() {
                if folder.exists() {
                    app.load_folder(folder.clone());
                    if let Some(ref file) = app.settings.last_file.clone() {
                        if let Some(idx) = app.image_list.iter().position(|p| p == file) {
                            app.current_index = idx;
                            app.load_current_image();
                        }
                    }
                }
            }
        }

        // Check command line arguments
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            let path = PathBuf::from(&args[1]);
            if path.is_file() && crate::image_loader::is_supported_image(&path) {
                app.load_image_file(path);
            } else if path.is_dir() {
                app.load_folder(path);
            }
        }

        // GPU processor will be initialized asynchronously later
        app.gpu_processor = None;

        app
    }
}

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.window_shadow = egui::epaint::Shadow::NONE;
    style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
    ctx.set_style(style);
}