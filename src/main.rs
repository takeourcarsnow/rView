#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod errors;
mod exif_data;
mod image_cache;
mod image_loader;
mod metadata;
mod profiler;
mod settings;
mod tests;
mod gpu;
mod ui;

use app::ImageViewerApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .format_timestamp(None)
        .init();

    // Get command line arguments for opening files/folders
    let args: Vec<String> = std::env::args().collect();
    let initial_path = args.get(1).map(std::path::PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RustView - Image Viewer")
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(load_icon())
            .with_drag_and_drop(true),
        vsync: true,
        multisampling: 0,
        ..Default::default()
    };

    eframe::run_native(
        "RustView",
        options,
        Box::new(move |cc| {
            // Enable image loading
            egui_extras::install_image_loaders(&cc.egui_ctx);
            
            // Create app
            let mut app = ImageViewerApp::new(cc);
            
            // Load initial path if provided
            if let Some(path) = initial_path {
                if path.is_dir() {
                    app.load_folder(path);
                } else {
                    app.load_image_file(path);
                }
            }
            
            Ok(Box::new(app))
        }),
    )
}

fn load_icon() -> egui::IconData {
    // Create a simple icon (32x32 gradient)
    let size = 32;
    let mut pixels = Vec::with_capacity(size * size * 4);
    
    for y in 0..size {
        for x in 0..size {
            let r = (x as f32 / size as f32 * 255.0) as u8;
            let g = (y as f32 / size as f32 * 255.0) as u8;
            let b = 200u8;
            let a = 255u8;
            
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
            pixels.push(a);
        }
    }
    
    egui::IconData {
        rgba: pixels,
        width: size as u32,
        height: size as u32,
    }
}
