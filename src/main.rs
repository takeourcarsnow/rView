#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod catalog;
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
mod logging;
mod updates;
mod telemetry;

use app::ImageViewerApp;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// Install icon fonts from iconflow (Lucide icons)
fn install_icon_fonts(ctx: &egui::Context) {
    let mut definitions = FontDefinitions::default();
    let fallback_fonts: Vec<String> = definitions.font_data.keys().cloned().collect();

    for font in iconflow::fonts() {
        definitions.font_data.insert(
            font.family.to_string(),
            Arc::new(FontData::from_static(font.bytes)),
        );
        let family = definitions
            .families
            .entry(FontFamily::Name(font.family.into()))
            .or_default();
        family.insert(0, font.family.to_string());
        
        // Add fallback fonts for text rendering
        for fallback in &fallback_fonts {
            if fallback != font.family {
                family.push(fallback.clone());
            }
        }
    }

    ctx.set_fonts(definitions);
}

fn main() -> eframe::Result<()> {
    // Get command line arguments for opening files/folders and detect debug flag
    let args: Vec<String> = std::env::args().collect();
    let debug_flag = args.iter().any(|a| a == "--debug" || a == "-d");
    logging::init_tracing(debug_flag);

    // Determine initial path (first non-flag argument that's not the program name)
    let initial_path = args.iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .map(|s| std::path::PathBuf::from(s));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("rView")
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(load_icon())
            .with_drag_and_drop(true),
        vsync: true,
        multisampling: 0,
        ..Default::default()
    };

    eframe::run_native(
        "rView",
        options,
        Box::new(move |cc| {
            // Enable image loading
            egui_extras::install_image_loaders(&cc.egui_ctx);
            
            // Install Lucide icon fonts
            install_icon_fonts(&cc.egui_ctx);
            
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
    // rView branded icon (32x32)
    // Rust orange with camera lens design and "r" motif
    let size = 32;
    let mut pixels = Vec::with_capacity(size * size * 4);
    
    let center = size as f32 / 2.0;
    let outer_radius = 14.0;
    let inner_radius = 10.0;
    let lens_radius = 7.0;
    
    // Brand colors (Rust orange palette)
    let rust_orange = (183u8, 65u8, 14u8);      // #B7410E
    let rust_dark = (139u8, 37u8, 0u8);         // #8B2500
    let lens_blue = (74u8, 144u8, 217u8);       // #4A90D9
    let dark_ring = (26u8, 26u8, 26u8);         // #1A1A1A
    
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            
            let (r, g, b, a) = if dist <= lens_radius {
                // Inner lens (blue gradient with highlight)
                let highlight = if dx < -1.0 && dy < -1.0 { 40u8 } else { 0u8 };
                (lens_blue.0.saturating_add(highlight), 
                 lens_blue.1.saturating_add(highlight), 
                 lens_blue.2.saturating_add(highlight), 255u8)
            } else if dist <= inner_radius {
                // Dark ring (camera body)
                (dark_ring.0, dark_ring.1, dark_ring.2, 255u8)
            } else if dist <= outer_radius {
                // Rust orange outer ring with gradient
                let t = (dist - inner_radius) / (outer_radius - inner_radius);
                let r = ((1.0 - t) * rust_orange.0 as f32 + t * rust_dark.0 as f32) as u8;
                let g = ((1.0 - t) * rust_orange.1 as f32 + t * rust_dark.1 as f32) as u8;
                let b = ((1.0 - t) * rust_orange.2 as f32 + t * rust_dark.2 as f32) as u8;
                (r, g, b, 255u8)
            } else if dist <= outer_radius + 1.5 {
                // Anti-aliased edge
                let alpha = ((outer_radius + 1.5 - dist) / 1.5 * 255.0) as u8;
                (rust_dark.0, rust_dark.1, rust_dark.2, alpha)
            } else {
                // Transparent background
                (0, 0, 0, 0)
            };
            
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
