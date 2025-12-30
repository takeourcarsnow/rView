use crate::app::ImageViewerApp;
use crate::settings::{ThumbnailPosition, ColorLabel};
use egui::{self, Color32, Vec2, Rounding, Margin, Rect};

impl ImageViewerApp {
    /// Pre-fetch thumbnails for items near the current view
    fn prefetch_visible_thumbnails(&mut self, ctx: &egui::Context) {
        // Request thumbnails for current index and nearby items
        let start = self.current_index.saturating_sub(10);
        let end = (self.current_index + 20).min(self.filtered_list.len());
        
        for display_idx in start..end {
            if let Some(&real_idx) = self.filtered_list.get(display_idx) {
                if let Some(path) = self.image_list.get(real_idx) {
                    if !self.thumbnail_textures.contains_key(path) {
                        self.request_thumbnail(path.clone(), ctx.clone());
                    }
                }
            }
        }
    }
    
    pub fn render_thumbnail_bar(&mut self, ctx: &egui::Context) {
        if !self.settings.show_thumbnails || self.filtered_list.is_empty() {
            return;
        }
        
        // Pre-request thumbnails for visible items
        self.prefetch_visible_thumbnails(ctx);
        
        let thumb_size = self.settings.thumbnail_size;
        let bar_size = thumb_size + 20.0;
        
        match self.settings.thumbnail_position {
            ThumbnailPosition::Bottom => {
                egui::TopBottomPanel::bottom("thumbnails")
                    .resizable(false)
                    .exact_height(bar_size)
                    .frame(egui::Frame::none()
                        .fill(Color32::from_rgb(25, 25, 28))
                        .inner_margin(Margin::symmetric(4.0, 8.0)))
                    .show(ctx, |ui| {
                        self.render_thumbnail_contents(ui, ctx, true);
                    });
            }
            ThumbnailPosition::Top => {
                egui::TopBottomPanel::top("thumbnails_top")
                    .resizable(false)
                    .exact_height(bar_size)
                    .frame(egui::Frame::none()
                        .fill(Color32::from_rgb(25, 25, 28))
                        .inner_margin(Margin::symmetric(4.0, 8.0)))
                    .show(ctx, |ui| {
                        self.render_thumbnail_contents(ui, ctx, true);
                    });
            }
            ThumbnailPosition::Left => {
                egui::SidePanel::left("thumbnails_left")
                    .resizable(false)
                    .exact_width(bar_size)
                    .frame(egui::Frame::none()
                        .fill(Color32::from_rgb(25, 25, 28))
                        .inner_margin(Margin::symmetric(8.0, 4.0)))
                    .show(ctx, |ui| {
                        self.render_thumbnail_contents(ui, ctx, false);
                    });
            }
            ThumbnailPosition::Right => {
                egui::SidePanel::right("thumbnails_right")
                    .resizable(false)
                    .exact_width(bar_size)
                    .frame(egui::Frame::none()
                        .fill(Color32::from_rgb(25, 25, 28))
                        .inner_margin(Margin::symmetric(8.0, 4.0)))
                    .show(ctx, |ui| {
                        self.render_thumbnail_contents(ui, ctx, false);
                    });
            }
        }
    }
    
    fn render_thumbnail_contents(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, horizontal: bool) {
        let thumb_size = self.settings.thumbnail_size;
        
        if horizontal {
            egui::ScrollArea::horizontal()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                        self.render_thumbnails_inner(ui, ctx, thumb_size);
                    });
                });
        } else {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 4.0);
                        self.render_thumbnails_inner(ui, ctx, thumb_size);
                    });
                });
        }
    }
    
    fn render_thumbnails_inner(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, thumb_size: f32) {
        // Collect data first to avoid borrow issues
        let thumb_data: Vec<_> = self.filtered_list.iter().enumerate().map(|(display_idx, &real_idx)| {
            let path = self.image_list.get(real_idx).cloned();
            let is_current = display_idx == self.current_index;
            let is_selected = self.selected_indices.contains(&display_idx);
            let tex_id = path.as_ref().and_then(|p| self.thumbnail_textures.get(p).map(|h| h.id()));
            let metadata = path.as_ref().map(|p| self.metadata_db.get(p));
            (display_idx, path, is_current, is_selected, tex_id, metadata)
        }).collect();
        
        for (display_idx, path, is_current, is_selected, tex_id, metadata) in thumb_data {
            if let Some(ref path) = path {
                let (response, painter) = ui.allocate_painter(
                    Vec2::splat(thumb_size),
                    egui::Sense::click()
                );
                
                let rect = response.rect;
                
                // Background and selection
                let bg_color = if is_current {
                    Color32::from_rgb(70, 130, 255)
                } else if is_selected {
                    Color32::from_rgb(50, 90, 180)
                } else if response.hovered() {
                    Color32::from_rgb(50, 50, 55)
                } else {
                    Color32::from_rgb(35, 35, 40)
                };
                
                painter.rect_filled(rect, Rounding::same(4.0), bg_color);
                
                // Thumbnail image
                if let Some(tex_id) = tex_id {
                    let inner_rect = rect.shrink(3.0);
                    painter.image(
                        tex_id,
                        inner_rect,
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    // Request thumbnail to be loaded if not already requested
                    if !self.thumbnail_requests.contains(path) {
                        self.request_thumbnail(path.clone(), ctx.clone());
                    }
                    
                    // Loading indicator - spinning animation
                    let time = ui.input(|i| i.time);
                    let angle = time * 2.0;
                    let spinner_char = match (angle as i32) % 4 {
                        0 => "◐",
                        1 => "◓",
                        2 => "◑",
                        _ => "◒",
                    };
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        spinner_char,
                        egui::FontId::proportional(18.0),
                        Color32::from_rgb(100, 100, 100),
                    );
                    // Request repaint for animation
                    ui.ctx().request_repaint();
                }
                
                // Rating stars (bottom left)
                if let Some(metadata) = &metadata {
                    if metadata.rating > 0 {
                        painter.text(
                            rect.left_bottom() + Vec2::new(3.0, -3.0),
                            egui::Align2::LEFT_BOTTOM,
                            "★".repeat(metadata.rating as usize),
                            egui::FontId::proportional(8.0),
                            Color32::from_rgb(255, 200, 50),
                        );
                    }
                    
                    // Color label dot (top right)
                    if metadata.color_label != ColorLabel::None {
                        painter.circle_filled(
                            rect.right_top() + Vec2::new(-6.0, 6.0),
                            4.0,
                            metadata.color_label.to_color(),
                        );
                    }
                }
                
                // Click handling
                if response.clicked() {
                    if ui.input(|i| i.modifiers.ctrl) {
                        // Multi-select
                        if self.selected_indices.contains(&display_idx) {
                            self.selected_indices.remove(&display_idx);
                        } else {
                            self.selected_indices.insert(display_idx);
                        }
                    } else if ui.input(|i| i.modifiers.shift) {
                        // Range select
                        let start = self.current_index.min(display_idx);
                        let end = self.current_index.max(display_idx);
                        for i in start..=end {
                            self.selected_indices.insert(i);
                        }
                    } else {
                        // Single select
                        self.selected_indices.clear();
                        self.current_index = display_idx;
                        self.load_current_image();
                    }
                }
                
                // Double-click for compare mode
                if response.double_clicked() {
                    self.compare_index = Some(display_idx);
                    self.view_mode = crate::app::ViewMode::Compare;
                }
                
                // Context menu
                response.context_menu(|ui| {
                    if ui.button("View").clicked() {
                        self.current_index = display_idx;
                        self.load_current_image();
                        ui.close_menu();
                    }
                    if ui.button("Compare with current").clicked() {
                        self.compare_index = Some(display_idx);
                        self.view_mode = crate::app::ViewMode::Compare;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Delete").clicked() {
                        self.current_index = display_idx;
                        self.delete_current_image();
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.menu_button("Rating", |ui| {
                        for r in 0..=5 {
                            let stars = if r == 0 { "None".to_string() } else { "★".repeat(r) };
                            if ui.button(stars).clicked() {
                                self.current_index = display_idx;
                                self.set_rating(r as u8);
                                ui.close_menu();
                            }
                        }
                    });
                });
            }
        }
    }
}
