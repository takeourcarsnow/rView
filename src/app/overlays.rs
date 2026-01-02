use crate::image_loader;
use image::DynamicImage;
use super::ImageViewerApp;

impl ImageViewerApp {
    pub fn generate_focus_peaking_overlay(&mut self, image: &DynamicImage, ctx: &egui::Context) {
        let overlay = if let Some(gpu) = &self.gpu_processor {
            match pollster::block_on(async {
                gpu.generate_focus_peaking_overlay(image, self.settings.focus_peaking_threshold).await
            }) {
                Ok(overlay) => overlay,
                Err(e) => {
                    log::warn!("GPU focus peaking failed: {}; falling back to CPU", e);
                    DynamicImage::ImageRgba8(image_loader::generate_focus_peaking_overlay(image, self.settings.focus_peaking_threshold))
                }
            }
        } else {
            DynamicImage::ImageRgba8(image_loader::generate_focus_peaking_overlay(image, self.settings.focus_peaking_threshold))
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
                gpu.generate_zebra_overlay(image, self.settings.zebra_high_threshold as f32 / 255.0, self.settings.zebra_low_threshold as f32 / 255.0).await
            }) {
                Ok(overlay) => overlay,
                Err(e) => {
                    log::warn!("GPU zebra overlay failed: {}; falling back to CPU", e);
                    DynamicImage::ImageRgba8(image_loader::generate_zebra_overlay(
                        image,
                        self.settings.zebra_high_threshold,
                        self.settings.zebra_low_threshold
                    ))
                }
            }
        } else {
            DynamicImage::ImageRgba8(image_loader::generate_zebra_overlay(
                image,
                self.settings.zebra_high_threshold,
                self.settings.zebra_low_threshold
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
}