mod app;
mod image_loader;
mod image_cache;
mod ui;
mod settings;
mod errors;
mod exif_data;

use app::ImageViewerApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(load_icon())
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "Image Viewer",
        native_options,
        Box::new(|cc| Ok(Box::new(ImageViewerApp::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    // Create a simple icon programmatically
    let size = 64;
    let mut rgba = vec![0u8; size * size * 4];
    
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            let cx = x as f32 - size as f32 / 2.0;
            let cy = y as f32 - size as f32 / 2.0;
            let dist = (cx * cx + cy * cy).sqrt();
            
            if dist < size as f32 / 2.0 - 2.0 {
                // Gradient from purple to blue
                let t = dist / (size as f32 / 2.0);
                rgba[idx] = (100.0 + 80.0 * t) as u8;     // R
                rgba[idx + 1] = (50.0 + 100.0 * t) as u8;  // G
                rgba[idx + 2] = (200.0 + 55.0 * (1.0 - t)) as u8; // B
                rgba[idx + 3] = 255; // A
            }
        }
    }
    
    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}
