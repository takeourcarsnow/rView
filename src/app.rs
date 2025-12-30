use crate::image_cache::ImageCache;
use crate::image_loader::{self, is_supported_image, ImageAdjustments, SUPPORTED_EXTENSIONS};
use crate::settings::{ColorLabel, Settings, SortMode};
use crate::exif_data::ExifInfo;
use crate::metadata::{MetadataDb, UndoHistory, FileOperation, RenamePattern};
use crate::profiler::{CacheStats, LoadingDiagnostics};

use eframe::egui::{self, Color32, TextureHandle, Vec2};
use image::DynamicImage;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use walkdir::WalkDir;

pub enum LoaderMessage {
    ImageLoaded(PathBuf, DynamicImage),
    ThumbnailLoaded(PathBuf, DynamicImage),
    LoadError(PathBuf, String),
    ExifLoaded(PathBuf, Box<ExifInfo>),
    /// Quick preview loaded (lower resolution, for RAW files)
    PreviewLoaded(PathBuf, DynamicImage),
}

#[derive(Debug, Clone)]
pub struct ImageTab {
    pub id: String,
    pub name: String,
    pub folder_path: PathBuf,
    pub image_list: Vec<PathBuf>,
    pub filtered_list: Vec<usize>,
    pub current_index: usize,
    pub zoom: f32,
    pub target_zoom: f32,
    pub pan_offset: Vec2,
    pub target_pan: Vec2,
    pub rotation: f32,
    pub adjustments: ImageAdjustments,
    pub show_original: bool,
    pub view_mode: ViewMode,
    pub compare_index: Option<usize>,
    pub lightbox_columns: usize,
    pub selected_indices: HashSet<usize>,
    pub last_selected: Option<usize>,
    pub search_query: String,
}

pub struct ImageViewerApp {
    // Settings
    pub settings: Settings,
    
    // Metadata database
    pub metadata_db: MetadataDb,
    
    // Tabs - multiple open folders
    pub tabs: Vec<ImageTab>,
    pub current_tab: usize,
    
    // Current tab's data (for backward compatibility)
    pub image_list: Vec<PathBuf>,
    pub filtered_list: Vec<usize>, // Indices into image_list
    pub current_index: usize,
    pub current_folder: Option<PathBuf>,
    
    // Multi-selection
    pub selected_indices: HashSet<usize>,
    pub last_selected: Option<usize>,
    
    // Current image state
    pub current_texture: Option<TextureHandle>,
    pub current_image: Option<DynamicImage>,
    pub current_exif: Option<ExifInfo>,
    pub histogram_data: Option<Vec<Vec<u32>>>,
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
    
    // Adjustments
    pub adjustments: ImageAdjustments,
    pub show_original: bool, // Before/After toggle
    
    // Cached data
    pub image_cache: Arc<ImageCache>,
    pub thumbnail_textures: HashMap<PathBuf, egui::TextureHandle>,
    pub thumbnail_requests: HashSet<PathBuf>,
    
    // File tree state
    pub expanded_dirs: HashSet<PathBuf>,
    
    // Async loading
    loader_tx: Sender<LoaderMessage>,
    pub loader_rx: Receiver<LoaderMessage>,
    
    // Slideshow
    pub slideshow_active: bool,
    pub slideshow_timer: f32,
    
    // Fullscreen
    pub is_fullscreen: bool,
    
    // View modes
    pub view_mode: ViewMode,
    pub compare_index: Option<usize>,
    pub lightbox_columns: usize,
    
    // Dialogs
    pub show_settings_dialog: bool,
    pub show_export_dialog: bool,
    pub show_batch_rename_dialog: bool,
    pub show_about_dialog: bool,
    pub show_shortcuts_dialog: bool,
    pub show_go_to_dialog: bool,
    pub go_to_input: String,
    pub search_query: String,
    pub command_palette_open: bool,
    pub command_palette_query: String,
    
    // Batch rename
    pub rename_pattern: RenamePattern,
    
    // Undo history
    pub undo_history: UndoHistory,
    
    // Mouse state
    pub loupe_position: Option<egui::Pos2>,
    pub color_picker_position: Option<egui::Pos2>,
    pub picked_color: Option<(u8, u8, u8)>,
    
    // Context for repaint requests
    pub ctx: Option<egui::Context>,
    
    // Status message
    pub status_message: Option<(String, std::time::Instant)>,
    
    // File watcher (for auto-refresh)
    pub watch_folder: bool,
    
    // Profiler and diagnostics
    pub profiler_enabled: bool,
    pub cache_stats: CacheStats,
    pub loading_diagnostics: LoadingDiagnostics,
    
    // Panel visibility
    pub panels_hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Single,
    Compare,
    Lightbox,
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
            tabs: Vec::new(),
            current_tab: 0,
            image_list: Vec::new(),
            filtered_list: Vec::new(),
            current_index: 0,
            current_folder: None,
            selected_indices: HashSet::new(),
            last_selected: None,
            current_texture: None,
            current_image: None,
            current_exif: None,
            histogram_data: None,
            is_loading: false,
            load_error: None,
            showing_preview: false,
            focus_peaking_texture: None,
            zebra_texture: None,
            zoom: 1.0,
            target_zoom: 1.0,
            pan_offset: Vec2::ZERO,
            target_pan: Vec2::ZERO,
            rotation: 0.0,
            adjustments: ImageAdjustments::default(),
            show_original: false,
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
            compare_index: None,
            lightbox_columns: 4,
            show_settings_dialog: false,
            show_export_dialog: false,
            show_batch_rename_dialog: false,
            show_about_dialog: false,
            show_shortcuts_dialog: false,
            show_go_to_dialog: false,
            go_to_input: String::new(),
            search_query: String::new(),
            command_palette_open: false,
            command_palette_query: String::new(),
            rename_pattern: RenamePattern::default(),
            undo_history: UndoHistory::new(50),
            loupe_position: None,
            color_picker_position: None,
            picked_color: None,
            ctx: Some(cc.egui_ctx.clone()),
            status_message: None,
            watch_folder: false,
            profiler_enabled: cfg!(debug_assertions), // Enabled in debug mode
            cache_stats: CacheStats::default(),
            loading_diagnostics: LoadingDiagnostics::default(),
            panels_hidden: false,
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
            if path.is_file() && is_supported_image(&path) {
                app.load_image_file(path);
            } else if path.is_dir() {
                app.load_folder(path);
            }
        }
        
        app
    }
    
    pub fn load_image_file(&mut self, path: PathBuf) {
        if let Some(parent) = path.parent() {
            self.load_folder(parent.to_path_buf());
            
            if let Some(idx) = self.image_list.iter().position(|p| p == &path) {
                self.current_index = idx;
                self.load_current_image();
            }
        }
    }
    
    pub fn load_folder(&mut self, folder: PathBuf) {
        // Create a new tab for the folder
        self.create_tab(folder);
    }
    
    pub fn sort_images(&mut self) {
        let current_path = self.get_current_path();
        
        match self.settings.sort_mode {
            SortMode::Name => {
                self.image_list.sort_by(|a, b| {
                    let a_name = a.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                    let b_name = b.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                    natord::compare(&a_name, &b_name)
                });
            }
            SortMode::Date => {
                self.image_list.sort_by(|a, b| {
                    let a_time = a.metadata().and_then(|m| m.modified()).ok();
                    let b_time = b.metadata().and_then(|m| m.modified()).ok();
                    a_time.cmp(&b_time)
                });
            }
            SortMode::DateTaken => {
                // Would need EXIF data cached - fall back to file date for now
                self.image_list.sort_by(|a, b| {
                    let a_time = a.metadata().and_then(|m| m.modified()).ok();
                    let b_time = b.metadata().and_then(|m| m.modified()).ok();
                    a_time.cmp(&b_time)
                });
            }
            SortMode::Size => {
                self.image_list.sort_by(|a, b| {
                    let a_size = a.metadata().map(|m| m.len()).unwrap_or(0);
                    let b_size = b.metadata().map(|m| m.len()).unwrap_or(0);
                    a_size.cmp(&b_size)
                });
            }
            SortMode::Type => {
                self.image_list.sort_by(|a, b| {
                    let a_ext = a.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                    let b_ext = b.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                    a_ext.cmp(&b_ext).then_with(|| {
                        let a_name = a.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                        let b_name = b.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                        natord::compare(&a_name, &b_name)
                    })
                });
            }
            SortMode::Rating => {
                self.image_list.sort_by(|a, b| {
                    let a_rating = self.metadata_db.get(a).rating;
                    let b_rating = self.metadata_db.get(b).rating;
                    b_rating.cmp(&a_rating) // Descending
                });
            }
            SortMode::Random => {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                self.image_list.shuffle(&mut rng);
            }
        }
        
        if !self.settings.sort_ascending {
            self.image_list.reverse();
        }
        
        // Restore selection
        if let Some(path) = current_path {
            if let Some(idx) = self.image_list.iter().position(|p| p == &path) {
                self.current_index = idx;
            }
        }
    }
    
    pub fn apply_filter(&mut self) {
        self.filtered_list.clear();
        
        for (idx, path) in self.image_list.iter().enumerate() {
            let metadata = self.metadata_db.get(path);
            
            // Filter by rating
            if let Some(min_rating) = self.settings.filter_by_rating {
                if metadata.rating < min_rating {
                    continue;
                }
            }
            
            // Filter by color
            if let Some(ref color) = self.settings.filter_by_color {
                if &metadata.color_label != color {
                    continue;
                }
            }
            
            // Filter by search query
            if !self.search_query.is_empty() {
                let filename = path.file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                if !filename.contains(&self.search_query.to_lowercase()) {
                    continue;
                }
            }
            
            self.filtered_list.push(idx);
        }
        
        // Ensure current_index is valid
        if self.current_index >= self.filtered_list.len() {
            self.current_index = self.filtered_list.len().saturating_sub(1);
        }
    }
    
    pub fn get_current_path(&self) -> Option<PathBuf> {
        self.filtered_list.get(self.current_index)
            .and_then(|&idx| self.image_list.get(idx))
            .cloned()
    }
    
    pub fn load_current_image(&mut self) {
        if let Some(path) = self.get_current_path() {
            self.is_loading = true;
            self.load_error = None;
            self.current_exif = None;
            self.histogram_data = None;
            self.focus_peaking_texture = None;
            self.zebra_texture = None;
            self.showing_preview = false;
            self.settings.last_file = Some(path.clone());
            
            // Check cache first
            if let Some(image) = self.image_cache.get(&path) {
                self.set_current_image(&path, image);
                return;
            }
            
            // For RAW files, load a quick preview first then the full image
            let is_raw = image_loader::is_raw_file(&path);
            
            if is_raw {
                // Load quick preview first (smaller resolution)
                let tx = self.loader_tx.clone();
                let path_clone = path.clone();
                let ctx = self.ctx.clone();
                
                thread::spawn(move || {
                    // Try to load a preview-sized version first
                    if let Ok(preview) = image_loader::load_thumbnail(&path_clone, 1920) {
                        let _ = tx.send(LoaderMessage::PreviewLoaded(path_clone.clone(), preview));
                        if let Some(ctx) = &ctx {
                            ctx.request_repaint();
                        }
                    }
                });
            }
            
            // Load full image asynchronously
            let tx = self.loader_tx.clone();
            let path_clone = path.clone();
            let ctx = self.ctx.clone();
            
            thread::spawn(move || {
                match image_loader::load_image(&path_clone) {
                    Ok(image) => {
                        let _ = tx.send(LoaderMessage::ImageLoaded(path_clone.clone(), image));
                    }
                    Err(e) => {
                        let _ = tx.send(LoaderMessage::LoadError(path_clone, e.to_string()));
                    }
                }
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            });
            
            // Load EXIF data asynchronously
            let tx = self.loader_tx.clone();
            let path_clone = path.clone();
            let ctx = self.ctx.clone();
            
            thread::spawn(move || {
                let exif = ExifInfo::from_file(&path_clone);
                let _ = tx.send(LoaderMessage::ExifLoaded(path_clone, Box::new(exif)));
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            });
            
            self.preload_adjacent();
        }
    }
    
    pub fn set_current_image(&mut self, path: &PathBuf, image: DynamicImage) {
        let ctx = match &self.ctx {
            Some(c) => c.clone(),
            None => return,
        };
        
        self.current_image = Some(image.clone());
        
        // Apply adjustments if any
        let display_image = if !self.adjustments.is_default() && !self.show_original {
            image_loader::apply_adjustments(&image, &self.adjustments)
        } else {
            image.clone()
        };
        
        let size = [display_image.width() as usize, display_image.height() as usize];
        let rgba = display_image.to_rgba8();
        let pixels = rgba.as_flat_samples();
        
        let texture = ctx.load_texture(
            path.to_string_lossy(),
            egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
            egui::TextureOptions::LINEAR,
        );
        
        self.current_texture = Some(texture);
        self.is_loading = false;
        
        // Always fit to window when loading a new image (unless maintaining zoom)
        if !self.settings.maintain_zoom_on_navigate {
            self.fit_to_window_internal();
        }
        
        if !self.settings.maintain_pan_on_navigate {
            self.pan_offset = Vec2::ZERO;
            self.target_pan = Vec2::ZERO;
        }
        
        // Calculate histogram
        self.histogram_data = Some(image_loader::calculate_histogram(&image));
        
        // Generate overlays if enabled
        if self.settings.show_focus_peaking {
            self.generate_focus_peaking_overlay(&image, &ctx);
        }
        
        if self.settings.show_zebras {
            self.generate_zebra_overlay(&image, &ctx);
        }
        
        // Cache the image
        self.image_cache.insert(path.clone(), image);
    }
    
    pub fn generate_focus_peaking_overlay(&mut self, image: &DynamicImage, ctx: &egui::Context) {
        let overlay = image_loader::generate_focus_peaking_overlay(
            image, 
            self.settings.focus_peaking_threshold
        );
        
        let size = [overlay.width() as usize, overlay.height() as usize];
        let pixels: Vec<u8> = overlay.into_raw();
        
        let texture = ctx.load_texture(
            "focus_peaking",
            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
            egui::TextureOptions::LINEAR,
        );
        
        self.focus_peaking_texture = Some(texture);
    }
    
    pub fn generate_zebra_overlay(&mut self, image: &DynamicImage, ctx: &egui::Context) {
        let overlay = image_loader::generate_zebra_overlay(
            image,
            self.settings.zebra_high_threshold,
            self.settings.zebra_low_threshold
        );
        
        let size = [overlay.width() as usize, overlay.height() as usize];
        let pixels: Vec<u8> = overlay.into_raw();
        
        let texture = ctx.load_texture(
            "zebra",
            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
            egui::TextureOptions::LINEAR,
        );
        
        self.zebra_texture = Some(texture);
    }
    
    fn preload_adjacent(&self) {
        let count = self.settings.preload_adjacent;
        let mut paths = Vec::new();
        
        for i in 1..=count {
            if self.current_index + i < self.filtered_list.len() {
                if let Some(&idx) = self.filtered_list.get(self.current_index + i) {
                    if let Some(path) = self.image_list.get(idx) {
                        paths.push(path.clone());
                    }
                }
            }
            if self.current_index >= i {
                if let Some(&idx) = self.filtered_list.get(self.current_index - i) {
                    if let Some(path) = self.image_list.get(idx) {
                        paths.push(path.clone());
                    }
                }
            }
        }
        
        self.image_cache.preload(paths);
    }
    
    pub fn request_thumbnail(&mut self, path: PathBuf, ctx: egui::Context) {
        if self.thumbnail_requests.contains(&path) {
            return;
        }
        
        self.thumbnail_requests.insert(path.clone());
        
        let tx = self.loader_tx.clone();
        let size = self.settings.thumbnail_size as u32;
        let cache = Arc::clone(&self.image_cache);
        
        thread::spawn(move || {
            if let Some(thumb) = cache.get_thumbnail(&path) {
                let _ = tx.send(LoaderMessage::ThumbnailLoaded(path, thumb));
                ctx.request_repaint();
                return;
            }
            
            if let Ok(thumb) = image_loader::load_thumbnail(&path, size) {
                cache.insert_thumbnail(path.clone(), thumb.clone());
                let _ = tx.send(LoaderMessage::ThumbnailLoaded(path, thumb));
                ctx.request_repaint();
            }
        });
    }
    
    pub fn reset_view(&mut self) {
        // Always fit to window by default
        self.fit_to_window_internal();
    }
    
    fn fit_to_window_internal(&mut self) {
        if let Some(texture) = &self.current_texture {
            let image_size = texture.size_vec2();
            // Estimate available space based on panel visibility
            let (width_est, height_est) = if self.panels_hidden {
                // More space available when panels are hidden
                (1800.0, 1000.0)
            } else {
                // Less space when panels are visible
                (1200.0, 700.0)
            };
            
            let available = Vec2::new(width_est, height_est);
            
            let scale_x = available.x / image_size.x;
            let scale_y = available.y / image_size.y;
            self.target_zoom = scale_x.min(scale_y).min(1.0);
            
            if !self.settings.smooth_zoom {
                self.zoom = self.target_zoom;
            }
        } else {
            self.target_zoom = 1.0;
            self.zoom = 1.0;
        }
        
        self.target_pan = Vec2::ZERO;
        if !self.settings.smooth_zoom {
            self.pan_offset = Vec2::ZERO;
        }
    }
    
    // Navigation
    pub fn next_image(&mut self) {
        if self.filtered_list.is_empty() {
            return;
        }
        
        let saved_zoom = self.zoom;
        let saved_pan = self.pan_offset;
        
        self.current_index = (self.current_index + 1) % self.filtered_list.len();
        self.load_current_image();
        
        if self.settings.maintain_zoom_on_navigate {
            self.zoom = saved_zoom;
            self.target_zoom = saved_zoom;
        }
        if self.settings.maintain_pan_on_navigate {
            self.pan_offset = saved_pan;
            self.target_pan = saved_pan;
        }
    }
    
    pub fn previous_image(&mut self) {
        if self.filtered_list.is_empty() {
            return;
        }
        
        let saved_zoom = self.zoom;
        let saved_pan = self.pan_offset;
        
        self.current_index = if self.current_index == 0 {
            self.filtered_list.len() - 1
        } else {
            self.current_index - 1
        };
        self.load_current_image();
        
        if self.settings.maintain_zoom_on_navigate {
            self.zoom = saved_zoom;
            self.target_zoom = saved_zoom;
        }
        if self.settings.maintain_pan_on_navigate {
            self.pan_offset = saved_pan;
            self.target_pan = saved_pan;
        }
    }
    
    pub fn go_to_first(&mut self) {
        if !self.filtered_list.is_empty() {
            self.current_index = 0;
            self.load_current_image();
        }
    }
    
    pub fn go_to_last(&mut self) {
        if !self.filtered_list.is_empty() {
            self.current_index = self.filtered_list.len() - 1;
            self.load_current_image();
        }
    }
    
    pub fn go_to_index(&mut self, index: usize) {
        if index < self.filtered_list.len() {
            let saved_zoom = self.zoom;
            let saved_pan = self.pan_offset;
            
            self.current_index = index;
            self.load_current_image();
            
            if self.settings.maintain_zoom_on_navigate {
                self.zoom = saved_zoom;
                self.target_zoom = saved_zoom;
            }
            if self.settings.maintain_pan_on_navigate {
                self.pan_offset = saved_pan;
                self.target_pan = saved_pan;
            }
        }
    }
    
    // Zoom
    pub fn zoom_in(&mut self) {
        self.target_zoom = (self.target_zoom * 1.25).min(32.0);
        if !self.settings.smooth_zoom {
            self.zoom = self.target_zoom;
        }
    }
    
    pub fn zoom_out(&mut self) {
        self.target_zoom = (self.target_zoom / 1.25).max(0.05);
        if !self.settings.smooth_zoom {
            self.zoom = self.target_zoom;
        }
    }
    
    pub fn zoom_to(&mut self, level: f32) {
        self.target_zoom = level.clamp(0.05, 32.0);
        if !self.settings.smooth_zoom {
            self.zoom = self.target_zoom;
        }
    }
    
    pub fn fit_to_window(&mut self) {
        if let Some(texture) = &self.current_texture {
            let image_size = texture.size_vec2();
            let available = Vec2::new(1200.0, 700.0); // Approximate available space
            
            let scale_x = available.x / image_size.x;
            let scale_y = available.y / image_size.y;
            self.target_zoom = scale_x.min(scale_y).min(1.0);
            
            if !self.settings.smooth_zoom {
                self.zoom = self.target_zoom;
            }
        }
        self.target_pan = Vec2::ZERO;
        self.pan_offset = Vec2::ZERO;
    }
    
    pub fn fill_window(&mut self) {
        if let Some(texture) = &self.current_texture {
            let image_size = texture.size_vec2();
            let available = Vec2::new(1200.0, 700.0); // Approximate available space
            
            let scale_x = available.x / image_size.x;
            let scale_y = available.y / image_size.y;
            self.target_zoom = scale_x.max(scale_y);
            
            if !self.settings.smooth_zoom {
                self.zoom = self.target_zoom;
            }
        }
        self.target_pan = Vec2::ZERO;
        self.pan_offset = Vec2::ZERO;
    }
    
    pub fn sort_file_list(&mut self) {
        self.sort_images();
        self.apply_filter();
    }
    
    // Rotation
    pub fn rotate_left(&mut self) {
        if let Some(path) = self.get_current_path() {
            let previous_rotation = self.rotation;
            self.rotation = (self.rotation - 90.0) % 360.0;
            self.undo_history.push(FileOperation::Rotate {
                path: path.clone(),
                degrees: -90,
                previous_rotation,
            });
        }
    }
    
    pub fn rotate_right(&mut self) {
        if let Some(path) = self.get_current_path() {
            let previous_rotation = self.rotation;
            self.rotation = (self.rotation + 90.0) % 360.0;
            self.undo_history.push(FileOperation::Rotate {
                path: path.clone(),
                degrees: 90,
                previous_rotation,
            });
        }
    }
    
    // Slideshow
    pub fn toggle_slideshow(&mut self) {
        self.slideshow_active = !self.slideshow_active;
        self.slideshow_timer = 0.0;
    }
    
    // Ratings
    pub fn set_rating(&mut self, rating: u8) {
        if let Some(path) = self.get_current_path() {
            let previous_rating = self.metadata_db.get(&path).rating;
            self.metadata_db.set_rating(path.clone(), rating);
            self.metadata_db.save();
            
            self.undo_history.push(FileOperation::Rate {
                path: path.clone(),
                rating,
                previous_rating,
            });
            
            self.show_status(&format!("Rating: {}", "★".repeat(rating as usize)));
        }
    }
    
    // Color labels
    pub fn set_color_label(&mut self, color: ColorLabel) {
        if let Some(path) = self.get_current_path() {
            let previous_color_label = self.metadata_db.get(&path).color_label;
            self.metadata_db.set_color_label(path.clone(), color);
            self.metadata_db.save();
            
            self.undo_history.push(FileOperation::Label {
                path: path.clone(),
                color_label: color,
                previous_color_label,
            });
        }
    }
    
    // File operations
    pub fn delete_current_image(&mut self) {
        if let Some(path) = self.get_current_path() {
            // Backup metadata before deletion
            let metadata_backup = serde_json::to_string(&self.metadata_db.get(&path)).ok();

            if self.settings.delete_to_trash {
                if trash::delete(&path).is_ok() {
                    self.undo_history.push(FileOperation::Delete {
                        original_path: path.clone(),
                        trash_path: None, // TODO: Get actual trash path if possible
                        metadata_backup,
                    });
                }
            } else {
                let _ = std::fs::remove_file(&path);
            }
            
            if let Some(&idx) = self.filtered_list.get(self.current_index) {
                self.image_list.remove(idx);
            }
            self.image_cache.remove(&path);
            self.thumbnail_textures.remove(&path);
            
            self.apply_filter();
            
            if self.current_index >= self.filtered_list.len() && !self.filtered_list.is_empty() {
                self.current_index = self.filtered_list.len() - 1;
            }
            
            if !self.filtered_list.is_empty() {
                self.load_current_image();
            } else {
                self.current_texture = None;
                self.current_image = None;
            }
            
            self.show_status("Image deleted");
        }
    }
    
    pub fn move_to_folder(&mut self, dest_folder: PathBuf) {
        if let Some(path) = self.get_current_path() {
            let filename = path.file_name().unwrap_or_default();
            let dest_path = dest_folder.join(filename);
            
            if std::fs::rename(&path, &dest_path).is_ok() {
                self.undo_history.push(FileOperation::Move {
                    from: path.clone(),
                    to: dest_path,
                });
                
                if let Some(&idx) = self.filtered_list.get(self.current_index) {
                    self.image_list.remove(idx);
                }
                self.apply_filter();
                
                if self.current_index >= self.filtered_list.len() && !self.filtered_list.is_empty() {
                    self.current_index = self.filtered_list.len() - 1;
                }
                
                if !self.filtered_list.is_empty() {
                    self.load_current_image();
                }
                
                self.show_status(&format!("Moved to {}", dest_folder.display()));
            }
        }
    }
    
    pub fn copy_to_folder(&mut self, dest_folder: PathBuf) {
        if let Some(path) = self.get_current_path() {
            let filename = path.file_name().unwrap_or_default();
            let dest_path = dest_folder.join(filename);
            
            if std::fs::copy(&path, &dest_path).is_ok() {
                self.show_status(&format!("Copied to {}", dest_folder.display()));
            }
        }
    }
    
    pub fn move_to_selected_folder(&mut self) {
        if let Some(path) = self.get_current_path() {
            if let Some(parent) = path.parent() {
                let selected_folder = parent.join("selected");
                
                // Create the selected folder if it doesn't exist
                if let Err(e) = std::fs::create_dir_all(&selected_folder) {
                    self.show_status(&format!("Failed to create selected folder: {}", e));
                    return;
                }
                
                let filename = path.file_name().unwrap_or_default();
                let dest_path = selected_folder.join(filename);
                
                if std::fs::rename(&path, &dest_path).is_ok() {
                    self.undo_history.push(FileOperation::Move {
                        from: path.clone(),
                        to: dest_path,
                    });
                    
                    if let Some(&idx) = self.filtered_list.get(self.current_index) {
                        self.image_list.remove(idx);
                    }
                    self.image_cache.remove(&path);
                    self.thumbnail_textures.remove(&path);
                    
                    self.apply_filter();
                    
                    if self.current_index >= self.filtered_list.len() && !self.filtered_list.is_empty() {
                        self.current_index = self.filtered_list.len() - 1;
                    }
                    
                    if !self.filtered_list.is_empty() {
                        self.load_current_image();
                    } else {
                        self.current_texture = None;
                        self.current_image = None;
                    }
                    
                    self.show_status(&format!("Moved to {}", selected_folder.display()));
                } else {
                    self.show_status("Failed to move file");
                }
            }
        }
    }
    
    pub fn undo_last_operation(&mut self) {
        let current_path = self.get_current_path();
        let op = self.undo_history.undo().cloned();
        
        if let Some(op) = op {
            match op {
                FileOperation::Delete { original_path, trash_path, metadata_backup } => {
                    // Try to restore from trash or show message
                    if let Some(trash_path) = trash_path {
                        if std::fs::rename(trash_path, &original_path).is_ok() {
                            // Restore metadata if available
                            if let Some(metadata_json) = metadata_backup {
                                if let Ok(metadata) = serde_json::from_str::<crate::metadata::ImageMetadata>(&metadata_json) {
                                    self.metadata_db.restore_metadata(original_path.clone(), metadata);
                                }
                            }
                            self.image_list.push(original_path.clone());
                            self.sort_images();
                            self.apply_filter();
                            self.show_status("Undo: File restored");
                        } else {
                            self.show_status(&format!("Cannot undo delete of {}", original_path.display()));
                        }
                    } else {
                        self.show_status(&format!("Cannot undo delete of {}", original_path.display()));
                    }
                }
                FileOperation::Move { from, to } => {
                    if std::fs::rename(&to, &from).is_ok() {
                        self.image_list.push(from.clone());
                        self.sort_images();
                        self.apply_filter();
                        self.show_status("Undo: Move reverted");
                    }
                }
                FileOperation::Rename { from, to } => {
                    if std::fs::rename(&to, &from).is_ok() {
                        if let Some(pos) = self.image_list.iter().position(|p| p == &*to) {
                            self.image_list[pos] = from.clone();
                        }
                        self.show_status("Undo: Rename reverted");
                    }
                }
                FileOperation::Rotate { path, degrees: _degrees, previous_rotation } => {
                    if current_path.as_ref() == Some(&path) {
                        self.rotation = previous_rotation;
                        self.refresh_adjustments();
                    }
                    self.show_status("Undo: Rotation reverted");
                }
                FileOperation::Adjust { path, previous_adjustments, .. } => {
                    if current_path.as_ref() == Some(&path) {
                        self.adjustments = previous_adjustments.clone();
                        self.refresh_adjustments();
                    }
                    self.show_status("Undo: Adjustments reverted");
                }
                FileOperation::Rate { path, previous_rating, .. } => {
                    self.metadata_db.set_rating(path.clone(), previous_rating);
                    self.metadata_db.save();
                    self.show_status("Undo: Rating reverted");
                }
                FileOperation::Label { path, previous_color_label, .. } => {
                    self.metadata_db.set_color_label(path.clone(), previous_color_label);
                    self.metadata_db.save();
                    self.show_status("Undo: Label reverted");
                }
            }
        }
    }

    pub fn redo_last_operation(&mut self) {
        let current_path = self.get_current_path();
        let op = self.undo_history.redo().cloned();
        
        if let Some(op) = op {
            match op {
                FileOperation::Delete { original_path, .. } => {
                    // Re-delete the file
                    if trash::delete(&original_path).is_ok() {
                        if let Some(&idx) = self.filtered_list.get(self.current_index) {
                            self.image_list.remove(idx);
                        }
                        self.apply_filter();
                        self.show_status("Redo: File deleted again");
                    }
                }
                FileOperation::Move { from, to } => {
                    if std::fs::rename(&to, &from).is_ok() {
                        if let Some(pos) = self.image_list.iter().position(|p| *p == from) {
                            self.image_list[pos] = from.clone();
                        }
                        self.show_status("Redo: Move reapplied");
                    }
                }
                FileOperation::Rename { from, to } => {
                    if std::fs::rename(&from, &to).is_ok() {
                        if let Some(pos) = self.image_list.iter().position(|p| *p == from) {
                            self.image_list[pos] = to.clone();
                        }
                        self.show_status("Redo: Rename reapplied");
                    }
                }
                FileOperation::Rotate { path, degrees, .. } => {
                    if current_path.as_ref() == Some(&path) {
                        self.rotation = (self.rotation + degrees as f32) % 360.0;
                        self.refresh_adjustments();
                    }
                    self.show_status("Redo: Rotation reapplied");
                }
                FileOperation::Adjust { path, adjustments, .. } => {
                    if current_path.as_ref() == Some(&path) {
                        self.adjustments = adjustments.clone();
                        self.refresh_adjustments();
                    }
                    self.show_status("Redo: Adjustments reapplied");
                }
                FileOperation::Rate { path, rating, .. } => {
                    self.metadata_db.set_rating(path.clone(), rating);
                    self.metadata_db.save();
                    self.show_status("Redo: Rating reapplied");
                }
                FileOperation::Label { path, color_label, .. } => {
                    self.metadata_db.set_color_label(path.clone(), color_label);
                    self.metadata_db.save();
                    self.show_status("Redo: Label reapplied");
                }
            }
        }
    }
    
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
    
    pub fn show_status(&mut self, message: &str) {
        self.status_message = Some((message.to_string(), std::time::Instant::now()));
    }
    
    pub fn toggle_compare_mode(&mut self) {
        if self.view_mode == ViewMode::Compare {
            self.view_mode = ViewMode::Single;
            self.compare_index = None;
        } else {
            self.view_mode = ViewMode::Compare;
            self.compare_index = Some(self.current_index);
        }
    }
    
    pub fn toggle_lightbox_mode(&mut self) {
        if self.view_mode == ViewMode::Lightbox {
            self.view_mode = ViewMode::Single;
        } else {
            self.view_mode = ViewMode::Lightbox;
        }
    }
    
    pub fn toggle_panels(&mut self) {
        self.panels_hidden = !self.panels_hidden;
        // Refit the image when panels are toggled
        self.fit_to_window_internal();
    }
    
    // Export
    pub fn export_current(&mut self, preset_name: &str) {
        if let (Some(image), Some(path)) = (&self.current_image, self.get_current_path()) {
            if let Some(preset) = self.settings.export_presets.iter().find(|p| p.name == preset_name) {
                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                let ext = match preset.format {
                    crate::settings::ExportFormat::Jpeg => "jpg",
                    crate::settings::ExportFormat::Png => "png",
                    crate::settings::ExportFormat::WebP => "webp",
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
    
    pub fn refresh_adjustments(&mut self) {
        if let (Some(image), Some(path)) = (self.current_image.clone(), self.get_current_path()) {
            if let Some(ctx) = &self.ctx {
                let display_image = if !self.adjustments.is_default() && !self.show_original {
                    image_loader::apply_adjustments(&image, &self.adjustments)
                } else {
                    image
                };
                
                let size = [display_image.width() as usize, display_image.height() as usize];
                let rgba = display_image.to_rgba8();
                let pixels = rgba.as_flat_samples();
                
                let texture = ctx.load_texture(
                    path.to_string_lossy(),
                    egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
                    egui::TextureOptions::LINEAR,
                );
                
                self.current_texture = Some(texture);
            }
        }
    }

    // Tab management methods
    pub fn create_tab(&mut self, folder_path: PathBuf) {
        let tab_name = folder_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "New Tab".to_string());
        
        let tab = ImageTab {
            id: uuid::Uuid::new_v4().to_string(),
            name: tab_name,
            folder_path: folder_path.clone(),
            image_list: Vec::new(),
            filtered_list: Vec::new(),
            current_index: 0,
            zoom: 1.0,
            target_zoom: 1.0,
            pan_offset: Vec2::ZERO,
            target_pan: Vec2::ZERO,
            rotation: 0.0,
            adjustments: ImageAdjustments::default(),
            show_original: false,
            view_mode: ViewMode::Single,
            compare_index: None,
            lightbox_columns: 4,
            selected_indices: HashSet::new(),
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

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    
    style.visuals.window_rounding = egui::Rounding::same(8.0);
    style.visuals.menu_rounding = egui::Rounding::same(6.0);
    style.visuals.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 4.0),
        blur: 8.0,
        spread: 0.0,
        color: Color32::from_black_alpha(60),
    };
    
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);
    
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    
    ctx.set_style(style);
}
