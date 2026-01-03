use crate::app::ImageViewerApp;
use crate::settings::BackgroundColor;
use egui::{self, Color32, CornerRadius, Rect, Vec2};

impl ImageViewerApp {
    pub fn render_main_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(self.settings.background_color.to_color()))
            .show(ctx, |ui| {
                match self.view_mode {
                    crate::app::ViewMode::Single => self.render_single_view(ui, ctx),
                    crate::app::ViewMode::Lightbox => {} // Handled separately
                    crate::app::ViewMode::Compare => self.render_compare_view(ctx),
                }
            });
    }

    fn render_single_view(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let available = ui.available_size();
        self.available_view_size = available; // Store for fit functions
        let response = ui.allocate_response(available, egui::Sense::click_and_drag());
        let rect = response.rect;

        // Handle mouse input
        self.handle_image_input(&response, ui);

        // Draw checkered background if selected
        if self.settings.background_color == BackgroundColor::Checkered {
            self.draw_checkered_background(ui, rect);
        }

        // Draw image
        if let Some(tex) = &self.current_texture {
            let tex_size = tex.size_vec2();
            // Keep the on-screen image size stable while showing a preview: if a smaller
            // preview texture is used, upscale it to match the original image size so the
            // image doesn't appear to shrink while dragging adjustments.
            let display_size = if self.showing_preview {
                if let Some(orig) = &self.current_image {
                    egui::Vec2::new(orig.width() as f32, orig.height() as f32) * self.zoom
                } else {
                    tex_size * self.zoom
                }
            } else {
                tex_size * self.zoom
            };

            let image_rect = Rect::from_center_size(rect.center() + self.pan_offset, display_size);

            ui.painter().image(
                tex.id(),
                image_rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );

            // Draw overlays
            self.draw_overlays(ui, image_rect);

            // Show EXIF overlay on top of image if enabled (overlay toggle controls only overlay)
            if self.settings.show_exif_overlay {
                if let Some(ref exif) = self.current_exif {
                    self.draw_exif_overlay(ui, image_rect, exif);
                }
            }

            // Show "Loading full resolution..." indicator for previews
            if self.showing_preview && self.is_loading {
                let indicator_rect = Rect::from_min_size(
                    rect.left_top() + Vec2::new(10.0, 10.0),
                    Vec2::new(200.0, 30.0),
                );
                ui.painter().rect_filled(
                    indicator_rect,
                    CornerRadius::same(6),
                    Color32::from_rgba_unmultiplied(0, 0, 0, 200),
                );
                ui.painter().text(
                    indicator_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "⟳ Loading full resolution...",
                    egui::FontId::proportional(12.0),
                    Color32::from_rgb(200, 200, 200),
                );
            }
        } else if self.is_loading {
            // Enhanced loading indicator with animation and progress
            let time = ui.input(|i| i.time);
            let _spinner_phase = (time * 2.0) % (std::f64::consts::PI * 2.0);
            let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let spinner_idx = ((time * 10.0) as usize) % spinner_chars.len();

            let loading_text = if self.showing_preview {
                format!("{} Loading full resolution...", spinner_chars[spinner_idx])
            } else {
                format!("{} Loading image...", spinner_chars[spinner_idx])
            };

            // Draw a subtle background
            let bg_rect = Rect::from_center_size(rect.center(), Vec2::new(300.0, 80.0));
            ui.painter().rect_filled(
                bg_rect,
                CornerRadius::same(12),
                Color32::from_rgba_unmultiplied(20, 20, 25, 220),
            );

            ui.painter().text(
                rect.center() + Vec2::new(0.0, -10.0),
                egui::Align2::CENTER_CENTER,
                &loading_text,
                egui::FontId::proportional(16.0),
                Color32::from_rgb(220, 220, 220),
            );

            // Add a simple progress bar animation
            let progress_width = 200.0;
            let progress_height = 4.0;
            let progress_rect = Rect::from_center_size(
                rect.center() + Vec2::new(0.0, 15.0),
                Vec2::new(progress_width, progress_height),
            );

            // Background
            ui.painter().rect_filled(
                progress_rect,
                CornerRadius::same(2),
                Color32::from_rgb(60, 60, 65),
            );

            // Animated progress fill
            let progress_fill = (time.sin() * 0.5 + 0.5) * 0.7 + 0.1; // 10% to 80%
            let fill_width = progress_width * progress_fill as f32;
            let fill_rect = Rect::from_min_size(
                progress_rect.left_top(),
                Vec2::new(fill_width, progress_height),
            );
            ui.painter().rect_filled(
                fill_rect,
                CornerRadius::same(2),
                Color32::from_rgb(100, 150, 255),
            );

            ui.ctx().request_repaint();
        } else if let Some(ref error) = self.load_error {
            // Error message
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("Error: {}", error),
                egui::FontId::proportional(18.0),
                Color32::from_rgb(255, 100, 100),
            );
        } else if self.image_list.is_empty() {
            // Drop hint
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Drop images or folders here\nor use Ctrl+O to open",
                egui::FontId::proportional(20.0),
                Color32::from_rgb(150, 150, 150),
            );
        }

        // Draw loupe if enabled
        if self.settings.loupe_enabled {
            self.draw_loupe(ui);
        }

        // Draw color picker info
        if self.picked_color.is_some() {
            self.draw_color_info(ui, rect);
        }

        // Note: EXIF overlay shown inline above when drawing the image so it has access to image_rect
    }

    fn draw_exif_overlay(
        &self,
        ui: &mut egui::Ui,
        image_rect: Rect,
        exif: &crate::exif_data::ExifInfo,
    ) {
        // Small semi-transparent overlay in bottom-left showing camera/date/settings
        let overlay_size = egui::Vec2::new(320.0, 44.0);
        let pos = image_rect.left_bottom() + egui::Vec2::new(12.0, -12.0 - overlay_size.y);
        let overlay_rect = Rect::from_min_size(pos, overlay_size);

        ui.painter().rect_filled(
            overlay_rect,
            CornerRadius::same(6),
            Color32::from_rgba_unmultiplied(0, 0, 0, 180),
        );

        if !exif.has_data() {
            ui.painter().text(
                overlay_rect.center(),
                egui::Align2::CENTER_CENTER,
                "No EXIF data",
                egui::FontId::proportional(13.0),
                Color32::WHITE,
            );
        } else {
            let camera = exif
                .camera_model
                .clone()
                .unwrap_or_else(|| "Unknown Camera".to_string());
            let date = exif
                .date_taken
                .clone()
                .unwrap_or_else(|| "Unknown Date".to_string());
            let settings = format!(
                "{} • {} • ISO {}",
                exif.focal_length_formatted(),
                exif.aperture_formatted(),
                exif.iso.clone().unwrap_or_default()
            );

            ui.painter().text(
                overlay_rect.left_top() + egui::Vec2::new(8.0, 6.0),
                egui::Align2::LEFT_TOP,
                camera,
                egui::FontId::proportional(13.0),
                Color32::WHITE,
            );
            ui.painter().text(
                overlay_rect.left_bottom() + egui::Vec2::new(8.0, -6.0),
                egui::Align2::LEFT_BOTTOM,
                format!("{} — {}", settings, date),
                egui::FontId::proportional(11.0),
                Color32::from_rgb(200, 200, 200),
            );
        }
    }

    /// Return the pixel size of a texture given its `TextureId` by inspecting
    /// the current texture and cached thumbnails. Falls back to a sensible
    /// default if the texture is unknown.
    pub(crate) fn texture_size_from_id(&self, id: egui::TextureId) -> Vec2 {
        if let Some(ref t) = self.current_texture {
            if t.id() == id {
                return t.size_vec2();
            }
        }
        for (_path, tex) in self.thumbnail_textures.iter() {
            if tex.id() == id {
                return tex.size_vec2();
            }
        }
        // Fallback default size to avoid division by zero
        Vec2::new(800.0, 600.0)
    }
}
