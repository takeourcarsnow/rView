use super::ImageViewerApp;
use crate::image_loader;
use image::DynamicImage;

impl ImageViewerApp {
    pub fn generate_focus_peaking_overlay(&mut self, image: &DynamicImage, ctx: &egui::Context) {
        let overlay = if let Some(gpu) = &self.gpu_processor {
            match pollster::block_on(async {
                // Normalize threshold for GPU: the shader works with normalized luminance (0.0-1.0)
                // and Sobel edge detection on normalized values produces edge strengths roughly in 0.0-1.0 range.
                // The CPU version uses raw pixel values (0-255), so threshold of 50 means magnitude ~50.
                // For GPU, we need to convert: threshold / 255.0 gives us a comparable normalized threshold.
                let normalized_threshold = self.settings.focus_peaking_threshold / 255.0;
                gpu.generate_focus_peaking_overlay(image, normalized_threshold)
                    .await
            }) {
                Ok(overlay) => overlay,
                Err(e) => {
                    log::warn!("GPU focus peaking failed: {}; falling back to CPU", e);
                    DynamicImage::ImageRgba8(image_loader::generate_focus_peaking_overlay(
                        image,
                        self.settings.focus_peaking_threshold,
                    ))
                }
            }
        } else {
            DynamicImage::ImageRgba8(image_loader::generate_focus_peaking_overlay(
                image,
                self.settings.focus_peaking_threshold,
            ))
        };

        let size = [overlay.width() as usize, overlay.height() as usize];
        let rgba = overlay.to_rgba8();
        let pixels: Vec<u8> = rgba.into_raw();

        let texture = ctx.load_texture(
            "focus_peaking",
            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
            egui::TextureOptions::LINEAR,
        );

        self.focus_peaking_texture = Some(texture);
    }

    pub fn generate_zebra_overlay(&mut self, image: &DynamicImage, ctx: &egui::Context) {
        let overlay = if let Some(gpu) = &self.gpu_processor {
            match pollster::block_on(async {
                gpu.generate_zebra_overlay(
                    image,
                    self.settings.zebra_high_threshold as f32 / 255.0,
                    self.settings.zebra_low_threshold as f32 / 255.0,
                )
                .await
            }) {
                Ok(overlay) => overlay,
                Err(e) => {
                    log::warn!("GPU zebra overlay failed: {}; falling back to CPU", e);
                    DynamicImage::ImageRgba8(image_loader::generate_zebra_overlay(
                        image,
                        self.settings.zebra_high_threshold,
                        self.settings.zebra_low_threshold,
                    ))
                }
            }
        } else {
            DynamicImage::ImageRgba8(image_loader::generate_zebra_overlay(
                image,
                self.settings.zebra_high_threshold,
                self.settings.zebra_low_threshold,
            ))
        };

        let size = [overlay.width() as usize, overlay.height() as usize];
        let rgba = overlay.to_rgba8();
        let pixels: Vec<u8> = rgba.into_raw();

        let texture = ctx.load_texture(
            "zebra",
            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
            egui::TextureOptions::LINEAR,
        );

        self.zebra_texture = Some(texture);
    }

    pub fn load_custom_overlay(&mut self, ctx: &egui::Context) {
        if let Some(overlay_name) = &self.settings.selected_overlay {
            let overlay_path = std::path::Path::new("src/images/overlays").join(overlay_name);
            if overlay_path.exists() {
                match image::open(&overlay_path) {
                    Ok(img) => {
                        let size = [img.width() as usize, img.height() as usize];
                        let rgba = img.to_rgba8();
                        let pixels: Vec<u8> = rgba.into_raw();

                        let texture = ctx.load_texture(
                            &format!("custom_overlay_{}", overlay_name),
                            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                            egui::TextureOptions::LINEAR,
                        );

                        self.custom_overlay_texture = Some(texture);
                    }
                    Err(e) => {
                        log::warn!("Failed to load overlay {}: {}", overlay_name, e);
                        self.custom_overlay_texture = None;
                    }
                }
            } else {
                log::warn!("Overlay file not found: {}", overlay_name);
                self.custom_overlay_texture = None;
            }
        } else {
            self.custom_overlay_texture = None;
        }
    }

    pub fn load_frame(&mut self, ctx: &egui::Context) {
        if let Some(frame_name) = &self.settings.selected_frame {
            let frame_path = std::path::Path::new("src/images/frames").join(frame_name);
            if frame_path.exists() {
                match image::open(&frame_path) {
                    Ok(img) => {
                        let size = [img.width() as usize, img.height() as usize];
                        let rgba = img.to_rgba8();
                        let pixels: Vec<u8> = rgba.into_raw();

                        let texture = ctx.load_texture(
                            &format!("frame_{}", frame_name),
                            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                            egui::TextureOptions::LINEAR,
                        );

                        self.frame_texture = Some(texture);
                    }
                    Err(e) => {
                        log::warn!("Failed to load frame {}: {}", frame_name, e);
                        self.frame_texture = None;
                    }
                }
            } else {
                log::warn!("Frame file not found: {}", frame_name);
                self.frame_texture = None;
            }
        } else {
            self.frame_texture = None;
        }
    }
}
