use crate::image_cache::ImageCache;
use crate::image_loader::{self, is_supported_image, SUPPORTED_EXTENSIONS};
use crate::settings::{FitMode, Settings, SortMode};
use crate::exif_data::ExifInfo;

use eframe::egui::{self, Color32, TextureHandle, Vec2};
use image::DynamicImage;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

// Message types for async image loading
pub enum LoaderMessage {
    ImageLoaded(PathBuf, DynamicImage),
    ThumbnailLoaded(PathBuf, DynamicImage),
    LoadError(PathBuf, String),
    ExifLoaded(PathBuf, ExifInfo),
    HistogramCalculated(PathBuf, Vec<Vec<u32>>),
}

pub struct ImageViewerApp {
    // Settings
    pub settings: Settings,
    
    // Image list and navigation
    pub image_list: Vec<PathBuf>,
    pub current_index: usize,
    pub current_folder: Option<PathBuf>,
    
    // Current image state
    pub current_texture: Option<TextureHandle>,
    pub current_exif: Option<ExifInfo>,
    pub histogram_data: Option<Vec<Vec<u32>>>,
    pub is_loading: bool,
    
    // View state
    pub zoom: f32,
    pub pan_offset: Vec2,
    pub rotation: f32,
    
    // Cached data
    pub image_cache: Arc<ImageCache>,
    pub thumbnail_textures: HashMap<PathBuf, egui::TextureId>,
    thumbnail_requests: std::collections::HashSet<PathBuf>,
    
    // Async loading
    loader_tx: Sender<LoaderMessage>,
    loader_rx: Receiver<LoaderMessage>,
    
    // Slideshow
    pub slideshow_active: bool,
    slideshow_timer: f32,
    
    // Fullscreen
    is_fullscreen: bool,
    
    // Context for repaint requests
    ctx: Option<egui::Context>,
}

impl ImageViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure fonts and styling
        configure_style(&cc.egui_ctx);
        
        // Create async channel
        let (tx, rx) = channel();
        
        let mut app = Self {
            settings: Settings::default(),
            image_list: Vec::new(),
            current_index: 0,
            current_folder: None,
            current_texture: None,
            current_exif: None,
            histogram_data: None,
            is_loading: false,
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
            rotation: 0.0,
            image_cache: Arc::new(ImageCache::new(512)),
            thumbnail_textures: HashMap::new(),
            thumbnail_requests: std::collections::HashSet::new(),
            loader_tx: tx,
            loader_rx: rx,
            slideshow_active: false,
            slideshow_timer: 0.0,
            is_fullscreen: false,
            ctx: Some(cc.egui_ctx.clone()),
        };
        
        // Check command line arguments for file/folder
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
            
            // Find and select the specific file
            if let Some(idx) = self.image_list.iter().position(|p| p == &path) {
                self.current_index = idx;
                self.load_current_image();
            }
        }
    }
    
    pub fn load_folder(&mut self, folder: PathBuf) {
        self.current_folder = Some(folder.clone());
        self.settings.add_recent_folder(folder.clone());
        
        // Collect all images in the folder
        self.image_list.clear();
        
        if let Ok(entries) = std::fs::read_dir(&folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && is_supported_image(&path) {
                    self.image_list.push(path);
                }
            }
        }
        
        self.sort_images();
        
        if !self.image_list.is_empty() {
            self.current_index = 0;
            self.load_current_image();
        }
    }
    
    pub fn sort_images(&mut self) {
        let current_path = self.image_list.get(self.current_index).cloned();
        
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
                    a_ext.cmp(&b_ext)
                });
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
    
    fn load_current_image(&mut self) {
        if let Some(path) = self.image_list.get(self.current_index).cloned() {
            self.is_loading = true;
            self.current_exif = None;
            self.histogram_data = None;
            
            // Check cache first
            if let Some(image) = self.image_cache.get(&path) {
                self.set_current_image(&path, image);
                return;
            }
            
            // Load asynchronously
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
                let _ = tx.send(LoaderMessage::ExifLoaded(path_clone, exif));
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            });
            
            // Preload adjacent images
            self.preload_adjacent();
        }
    }
    
    fn set_current_image(&mut self, path: &PathBuf, image: DynamicImage) {
        if let Some(ctx) = &self.ctx {
            // Convert to texture
            let size = [image.width() as usize, image.height() as usize];
            let rgba = image.to_rgba8();
            let pixels = rgba.as_flat_samples();
            
            let texture = ctx.load_texture(
                path.to_string_lossy(),
                egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
                egui::TextureOptions::LINEAR,
            );
            
            self.current_texture = Some(texture);
            self.is_loading = false;
            
            // Apply fit mode (unless maintaining zoom)
            if !self.settings.maintain_zoom_on_navigate {
                self.reset_view();
            }
            
            if !self.settings.maintain_pan_on_navigate {
                self.pan_offset = Vec2::ZERO;
            }
            
            // Calculate histogram
            self.calculate_histogram(&image);
            
            // Cache the image
            self.image_cache.insert(path.clone(), image);
        }
    }
    
    fn calculate_histogram(&mut self, image: &DynamicImage) {
        let rgb = image.to_rgb8();
        let mut histogram = vec![vec![0u32; 256]; 3];
        
        for pixel in rgb.pixels() {
            histogram[0][pixel[0] as usize] += 1;
            histogram[1][pixel[1] as usize] += 1;
            histogram[2][pixel[2] as usize] += 1;
        }
        
        self.histogram_data = Some(histogram);
    }
    
    fn preload_adjacent(&self) {
        let count = self.settings.preload_adjacent;
        let mut paths = Vec::new();
        
        for i in 1..=count {
            if self.current_index + i < self.image_list.len() {
                paths.push(self.image_list[self.current_index + i].clone());
            }
            if self.current_index >= i {
                paths.push(self.image_list[self.current_index - i].clone());
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
            // Check cache first
            if let Some(thumb) = cache.get_thumbnail(&path) {
                let _ = tx.send(LoaderMessage::ThumbnailLoaded(path, thumb));
                ctx.request_repaint();
                return;
            }
            
            // Load thumbnail
            if let Ok(thumb) = image_loader::load_thumbnail(&path, size) {
                cache.insert_thumbnail(path.clone(), thumb.clone());
                let _ = tx.send(LoaderMessage::ThumbnailLoaded(path, thumb));
                ctx.request_repaint();
            }
        });
    }
    
    pub fn reset_view(&mut self) {
        if let Some(texture) = &self.current_texture {
            let image_size = texture.size_vec2();
            
            // Get approximate available size (will be refined in render)
            let available = Vec2::new(1200.0, 700.0);
            
            self.zoom = match self.settings.fit_mode {
                FitMode::Fit => {
                    let scale_x = available.x / image_size.x;
                    let scale_y = available.y / image_size.y;
                    scale_x.min(scale_y).min(1.0)
                }
                FitMode::Fill => {
                    let scale_x = available.x / image_size.x;
                    let scale_y = available.y / image_size.y;
                    scale_x.max(scale_y)
                }
                FitMode::OneToOne => 1.0,
                FitMode::FitWidth => (available.x / image_size.x).min(1.0),
                FitMode::FitHeight => (available.y / image_size.y).min(1.0),
            };
        } else {
            self.zoom = 1.0;
        }
        
        self.pan_offset = Vec2::ZERO;
    }
    
    // Navigation functions
    pub fn next_image(&mut self) {
        if self.image_list.is_empty() {
            return;
        }
        
        let saved_zoom = self.zoom;
        let saved_pan = self.pan_offset;
        
        self.current_index = (self.current_index + 1) % self.image_list.len();
        self.load_current_image();
        
        if self.settings.maintain_zoom_on_navigate {
            self.zoom = saved_zoom;
        }
        if self.settings.maintain_pan_on_navigate {
            self.pan_offset = saved_pan;
        }
    }
    
    pub fn previous_image(&mut self) {
        if self.image_list.is_empty() {
            return;
        }
        
        let saved_zoom = self.zoom;
        let saved_pan = self.pan_offset;
        
        self.current_index = if self.current_index == 0 {
            self.image_list.len() - 1
        } else {
            self.current_index - 1
        };
        self.load_current_image();
        
        if self.settings.maintain_zoom_on_navigate {
            self.zoom = saved_zoom;
        }
        if self.settings.maintain_pan_on_navigate {
            self.pan_offset = saved_pan;
        }
    }
    
    pub fn go_to_first(&mut self) {
        if !self.image_list.is_empty() {
            self.current_index = 0;
            self.load_current_image();
        }
    }
    
    pub fn go_to_last(&mut self) {
        if !self.image_list.is_empty() {
            self.current_index = self.image_list.len() - 1;
            self.load_current_image();
        }
    }
    
    pub fn go_to_index(&mut self, index: usize) {
        if index < self.image_list.len() {
            let saved_zoom = self.zoom;
            let saved_pan = self.pan_offset;
            
            self.current_index = index;
            self.load_current_image();
            
            if self.settings.maintain_zoom_on_navigate {
                self.zoom = saved_zoom;
            }
            if self.settings.maintain_pan_on_navigate {
                self.pan_offset = saved_pan;
            }
        }
    }
    
    // Zoom functions
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.2).min(20.0);
    }
    
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.2).max(0.1);
    }
    
    // Rotation functions
    pub fn rotate_left(&mut self) {
        self.rotation = (self.rotation - 90.0) % 360.0;
    }
    
    pub fn rotate_right(&mut self) {
        self.rotation = (self.rotation + 90.0) % 360.0;
    }
    
    // Slideshow
    pub fn toggle_slideshow(&mut self) {
        self.slideshow_active = !self.slideshow_active;
        self.slideshow_timer = 0.0;
    }
    
    // File dialogs
    pub fn open_file_dialog(&mut self) {
        let filter_name = "Images";
        let extensions: Vec<&str> = SUPPORTED_EXTENSIONS.to_vec();
        
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(filter_name, &extensions)
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
        if let Some(path) = self.image_list.get(self.current_index) {
            let _ = open::that(path.parent().unwrap_or(path));
        }
    }
    
    pub fn delete_current_image(&mut self) {
        if let Some(path) = self.image_list.get(self.current_index).cloned() {
            // Move to trash instead of permanent delete
            if let Err(_) = trash::delete(&path) {
                // Fallback: try permanent delete
                let _ = std::fs::remove_file(&path);
            }
            
            self.image_list.remove(self.current_index);
            self.image_cache.remove(&path);
            self.thumbnail_textures.remove(&path);
            
            if self.current_index >= self.image_list.len() && !self.image_list.is_empty() {
                self.current_index = self.image_list.len() - 1;
            }
            
            if !self.image_list.is_empty() {
                self.load_current_image();
            } else {
                self.current_texture = None;
            }
        }
    }
    
    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Navigation
            if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::A) {
                self.previous_image();
            }
            if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::D) {
                self.next_image();
            }
            if i.key_pressed(egui::Key::Home) {
                self.go_to_first();
            }
            if i.key_pressed(egui::Key::End) {
                self.go_to_last();
            }
            if i.key_pressed(egui::Key::PageUp) {
                for _ in 0..10 {
                    self.previous_image();
                }
            }
            if i.key_pressed(egui::Key::PageDown) {
                for _ in 0..10 {
                    self.next_image();
                }
            }
            
            // Zoom
            if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                self.zoom_in();
            }
            if i.key_pressed(egui::Key::Minus) {
                self.zoom_out();
            }
            if i.key_pressed(egui::Key::Num0) {
                self.reset_view();
            }
            if i.key_pressed(egui::Key::Num1) {
                self.zoom = 1.0;
                self.pan_offset = Vec2::ZERO;
            }
            
            // Rotation
            if i.key_pressed(egui::Key::L) {
                self.rotate_left();
            }
            if i.key_pressed(egui::Key::R) {
                self.rotate_right();
            }
            
            // Slideshow
            if i.key_pressed(egui::Key::Space) {
                self.toggle_slideshow();
            }
            
            // Fullscreen
            if i.key_pressed(egui::Key::F11) || i.key_pressed(egui::Key::F) {
                self.is_fullscreen = !self.is_fullscreen;
            }
            if i.key_pressed(egui::Key::Escape) {
                if self.is_fullscreen {
                    self.is_fullscreen = false;
                } else if self.slideshow_active {
                    self.slideshow_active = false;
                }
            }
            
            // Delete
            if i.key_pressed(egui::Key::Delete) {
                self.delete_current_image();
            }
            
            // Toggle panels
            if i.key_pressed(egui::Key::I) {
                self.settings.show_exif = !self.settings.show_exif;
            }
            if i.key_pressed(egui::Key::T) {
                self.settings.show_thumbnails = !self.settings.show_thumbnails;
            }
            if i.key_pressed(egui::Key::S) && !i.modifiers.ctrl {
                self.settings.show_sidebar = !self.settings.show_sidebar;
            }
        });
    }
    
    fn process_loader_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.loader_rx.try_recv() {
            match msg {
                LoaderMessage::ImageLoaded(path, image) => {
                    if self.image_list.get(self.current_index) == Some(&path) {
                        self.set_current_image(&path, image);
                    } else {
                        self.image_cache.insert(path, image);
                    }
                }
                LoaderMessage::ThumbnailLoaded(path, thumb) => {
                    // Convert to texture
                    let size = [thumb.width() as usize, thumb.height() as usize];
                    let rgba = thumb.to_rgba8();
                    let pixels = rgba.as_flat_samples();
                    
                    let texture = ctx.load_texture(
                        format!("thumb_{}", path.display()),
                        egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
                        egui::TextureOptions::LINEAR,
                    );
                    
                    self.thumbnail_textures.insert(path, texture.id());
                }
                LoaderMessage::LoadError(path, error) => {
                    log::error!("Failed to load {}: {}", path.display(), error);
                    if self.image_list.get(self.current_index) == Some(&path) {
                        self.is_loading = false;
                    }
                }
                LoaderMessage::ExifLoaded(path, exif) => {
                    if self.image_list.get(self.current_index) == Some(&path) {
                        self.current_exif = Some(exif);
                    }
                }
                LoaderMessage::HistogramCalculated(path, histogram) => {
                    if self.image_list.get(self.current_index) == Some(&path) {
                        self.histogram_data = Some(histogram);
                    }
                }
            }
        }
    }
    
    fn update_slideshow(&mut self, ctx: &egui::Context) {
        if self.slideshow_active {
            self.slideshow_timer += ctx.input(|i| i.stable_dt);
            
            if self.slideshow_timer >= self.settings.slideshow_interval {
                self.slideshow_timer = 0.0;
                self.next_image();
            }
            
            ctx.request_repaint();
        }
    }
}

impl eframe::App for ImageViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Store context for repaint requests
        self.ctx = Some(ctx.clone());
        
        // Process async messages
        self.process_loader_messages(ctx);
        
        // Handle keyboard input
        self.handle_keyboard(ctx);
        
        // Update slideshow
        self.update_slideshow(ctx);
        
        // Apply theme
        apply_theme(ctx, &self.settings);
        
        // Render UI
        self.render_top_bar(ctx);
        self.render_sidebar(ctx);
        self.render_thumbnail_bar(ctx);
        self.render_main_view(ctx);
    }
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings.save();
    }
}

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    
    // Modern rounded corners
    style.visuals.window_rounding = egui::Rounding::same(8.0);
    style.visuals.menu_rounding = egui::Rounding::same(6.0);
    style.visuals.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 4.0),
        blur: 8.0,
        spread: 0.0,
        color: Color32::from_black_alpha(60),
    };
    
    // Button styling
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);
    
    // Spacing
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    
    ctx.set_style(style);
}

fn apply_theme(ctx: &egui::Context, settings: &Settings) {
    match settings.theme {
        crate::settings::Theme::Dark => {
            ctx.set_visuals(egui::Visuals::dark());
        }
        crate::settings::Theme::Light => {
            ctx.set_visuals(egui::Visuals::light());
        }
        crate::settings::Theme::System => {
            // Default to dark for now
            ctx.set_visuals(egui::Visuals::dark());
        }
    }
}
