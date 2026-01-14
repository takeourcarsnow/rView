use crate::exif_data::ExifInfo;
use crate::image_cache::ImageCache;
use crate::image_loader::ImageAdjustments;
use crate::metadata::{MetadataDb, UndoHistory};
use crate::profiler::{CacheStats, LoadingDiagnostics};
use crate::settings::Settings;
use crate::task_scheduler::{MemoryPool, TaskScheduler};

use eframe::egui::{self, TextureHandle, Vec2};
use image::DynamicImage;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use crate::gpu::types::GpuProcessor;

pub enum LoaderMessage {
    ImageLoaded(PathBuf, DynamicImage),
    PreviewLoaded(PathBuf, DynamicImage),
    ProgressiveLoaded(PathBuf, DynamicImage),
    ThumbnailLoaded(PathBuf, DynamicImage),
    ThumbnailRequestComplete(PathBuf),
    LoadError(PathBuf, String),
    ExifLoaded(PathBuf, Box<ExifInfo>),
    TextureCreated(PathBuf, egui::TextureHandle, DynamicImage),
    HistogramUpdated(Vec<Vec<u32>>),
    MoveCompleted {
        from: PathBuf,
        dest_folder: PathBuf,
        success: bool,
        error: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Single,
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
    pub custom_overlay_texture: Option<TextureHandle>,
    pub frame_texture: Option<TextureHandle>,

    // View state
    pub zoom: f32,
    pub target_zoom: f32,
    pub pan_offset: Vec2,
    pub target_pan: Vec2,
    pub rotation: f32,
    pub available_view_size: Vec2, // Available space for image display

    // Crop state
    pub crop_mode: bool,                    // Whether crop tool is active
    pub crop_rect: Option<egui::Rect>,      // Current crop rectangle in image coordinates
    pub crop_start_pos: Option<egui::Pos2>, // Starting position for crop drag

    // Adjustments
    pub adjustments: ImageAdjustments,
    pub current_film_preset: crate::image_loader::FilmPreset,
    pub show_original: bool, // Before/After toggle
    pub last_adjustment_time: std::time::Instant,
    pub adjustments_dirty: bool, // Flag to indicate adjustments need to be applied
    pub slider_dragging: bool,   // True while user is actively dragging a slider
    pub pre_drag_adjustments: Option<ImageAdjustments>, // Adjustments before drag started (for undo)

    // Cached data
    pub image_cache: Arc<ImageCache>,
    pub texture_cache: HashMap<String, (egui::TextureHandle, std::time::Instant)>, // Cache for created textures with access time
    pub texture_access_order: VecDeque<String>, // LRU order tracking
    pub thumbnail_textures: HashMap<PathBuf, egui::TextureHandle>,
    pub thumbnail_requests: HashSet<PathBuf>,
    pub compare_large_preview_requests: HashSet<PathBuf>,

    // File tree state
    pub expanded_dirs: HashSet<PathBuf>,

    // Async loading
    pub loader_tx: Sender<LoaderMessage>,
    pub loader_rx: Receiver<LoaderMessage>,

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

    // Compare view interaction state (zoom per side)
    pub compare_zoom: [f32; 2],
    pub compare_pan: [egui::Vec2; 2],

    // GPU initialization state
    pub gpu_initialization_attempted: bool,

    // Thumbnail scroll state
    pub thumbnail_scroll_offset: Vec2,

    // Telemetry
    #[allow(dead_code)]
    pub telemetry: Option<crate::telemetry::Telemetry>,

    // Performance optimizations
    pub task_scheduler: TaskScheduler,
    pub memory_pool: MemoryPool,
}

impl ImageViewerApp {
    pub fn set_status_message(&mut self, msg: String) {
        self.status_message = Some((msg, std::time::Instant::now()));
    }

    /// Mark adjustments as needing refresh without debounce check
    pub fn mark_adjustments_dirty(&mut self) {
        self.adjustments_dirty = true;
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
    }

    pub fn invalidate_texture_cache_for_path(&mut self, path: &Path) {
        let prefix = format!("{}_", path.to_string_lossy());
        self.texture_cache.retain(|k, _| !k.starts_with(&prefix));
        self.texture_access_order
            .retain(|k| !k.starts_with(&prefix));
    }

    /// Get the path of the currently displayed image
    pub fn get_current_image_path(&self) -> Option<&PathBuf> {
        self.filtered_list
            .get(self.current_index)
            .and_then(|&idx| self.image_list.get(idx))
    }

    /// Clean up unused textures to free GPU memory
    pub fn cleanup_unused_textures(&mut self) {
        // Keep only textures for current image and recent images
        let mut used_texture_names = HashSet::new();

        // Keep current image texture
        if let Some(current_path) = self.get_current_image_path() {
            if let Some(current_image) = &self.current_image {
                let texture_name = format!(
                    "{}_{}_{}x{}",
                    current_path.to_string_lossy(),
                    self.adjustments.frame_enabled as u8,
                    current_image.width(),
                    current_image.height()
                );
                used_texture_names.insert(texture_name);
            }
        }

        // Keep textures for adjacent images (preload)
        for offset in -2..=2 {
            let idx = (self.current_index as isize + offset) as usize;
            if let Some(&real_idx) = self.filtered_list.get(idx) {
                if let Some(path) = self.image_list.get(real_idx) {
                    // We don't know the exact dimensions, so we'll be conservative
                    // and keep any texture that starts with this path
                    let path_prefix = path.to_string_lossy().to_string();
                    used_texture_names.extend(
                        self.texture_cache
                            .keys()
                            .filter(|k| k.starts_with(&path_prefix))
                            .cloned(),
                    );
                }
            }
        }

        // Remove unused textures
        self.texture_cache
            .retain(|name, _| used_texture_names.contains(name));
        // Update access order to match
        self.texture_access_order
            .retain(|name| used_texture_names.contains(name));
    }

    pub fn refresh_adjustments_if_dirty(&mut self) {
        if !self.adjustments_dirty {
            return;
        }

        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_adjustment_time);

        // Use longer debounce while dragging for smoother feel, shorter on release
        // Reduced from 200ms -> 80ms to improve responsiveness while dragging
        let debounce_ms = if self.slider_dragging { 80 } else { 16 };

        if elapsed.as_millis() >= debounce_ms {
            self.adjustments_dirty = false;
            self.last_adjustment_time = now;
            // Use lightweight refresh while dragging (skip histogram/overlays)
            crate::profiler::with_profiler(|p| p.start_timer("refresh_adjustments_if_dirty"));
            self.refresh_adjustments_internal(!self.slider_dragging);
            crate::profiler::with_profiler(|p| p.end_timer("refresh_adjustments_if_dirty"));
        } else {
            // Schedule another repaint to process later
            if let Some(ctx) = &self.ctx {
                let remaining = debounce_ms as u64 - elapsed.as_millis() as u64;
                ctx.request_repaint_after(std::time::Duration::from_millis(remaining));
            }
        }
    }
}

impl ImageViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);

        let (tx, rx) = channel();

        let settings = Settings::load();
        let telemetry_enabled = settings.telemetry_enabled;
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
            custom_overlay_texture: None,
            frame_texture: None,
            compare_exifs: HashMap::new(),
            compare_large_preview_requests: HashSet::new(),
            zoom: 1.0,
            target_zoom: 1.0,
            pan_offset: Vec2::ZERO,
            target_pan: Vec2::ZERO,
            rotation: 0.0,
            available_view_size: Vec2::new(800.0, 600.0), // Default fallback
            crop_mode: false,
            crop_rect: None,
            crop_start_pos: None,
            adjustments: ImageAdjustments::default(),
            current_film_preset: crate::image_loader::FilmPreset::None,
            show_original: false,
            last_adjustment_time: std::time::Instant::now(),
            adjustments_dirty: false,
            slider_dragging: false,
            pre_drag_adjustments: None,
            image_cache: Arc::new(ImageCache::new(1024)),
            texture_cache: HashMap::new(),
            texture_access_order: VecDeque::new(),
            thumbnail_textures: HashMap::new(),
            thumbnail_requests: HashSet::new(),
            expanded_dirs: HashSet::new(),
            loader_tx: tx,
            loader_rx: rx,
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
            gpu_initialization_attempted: false,
            thumbnail_scroll_offset: Vec2::ZERO,
            telemetry: Some(crate::telemetry::Telemetry::new(telemetry_enabled)),
            task_scheduler: TaskScheduler::default(),
            memory_pool: MemoryPool::default(),
        };

        // (Update checking removed)

        // Restore session
        if app.settings.restore_session {
            if let Some(ref folder) = app.settings.last_folder.clone() {
                if folder.exists() {
                    app.load_folder(folder.clone());
                    if let Some(ref file) = app.settings.last_file.clone() {
                        if let Some(idx) = app.image_list.iter().position(|p| p == file) {
                            app.current_index = idx;
                            // Load adjustments for restored session
                            app.load_adjustments_for_current();
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
