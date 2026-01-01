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
                if let Some(path) = self.image_list.get(real_idx).cloned() {
                    // Avoid holding mutable and immutable borrows simultaneously by copying path
                    if !self.thumbnail_textures.contains_key(&path) {
                        self.ensure_thumbnail_requested(&path, ctx);
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
        // Add extra space for optional filename / resolution labels
        let bar_size = thumb_size + 34.0;
        
        match self.settings.thumbnail_position {
            ThumbnailPosition::Bottom => {
                if self.thumbnail_collapsed {
                    egui::TopBottomPanel::bottom("thumbnails")
                        .exact_height(24.0)
                        .frame(egui::Frame::none()
                            .fill(Color32::from_rgb(25, 25, 28))
                            .inner_margin(Margin::symmetric(4.0, 8.0)))
                        .show(ctx, |ui| {
                            ui.horizontal_centered(|ui| {
                                if ui.button("⊞").clicked() {
                                    self.thumbnail_collapsed = false;
                                }
                            });
                        });
                } else {
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
            }
            ThumbnailPosition::Top => {
                if self.thumbnail_collapsed {
                    egui::TopBottomPanel::top("thumbnails_top")
                        .exact_height(24.0)
                        .frame(egui::Frame::none()
                            .fill(Color32::from_rgb(25, 25, 28))
                            .inner_margin(Margin::symmetric(4.0, 8.0)))
                        .show(ctx, |ui| {
                            ui.horizontal_centered(|ui| {
                                if ui.button("⊞").clicked() {
                                    self.thumbnail_collapsed = false;
                                }
                            });
                        });
                } else {
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
            }
            ThumbnailPosition::Left => {
                if self.thumbnail_collapsed {
                    egui::SidePanel::left("thumbnails_left")
                        .exact_width(24.0)
                        .frame(egui::Frame::none()
                            .fill(Color32::from_rgb(25, 25, 28))
                            .inner_margin(Margin::symmetric(8.0, 4.0)))
                        .show(ctx, |ui| {
                            ui.vertical_centered(|ui| {
                                if ui.button("⊞").clicked() {
                                    self.thumbnail_collapsed = false;
                                }
                            });
                        });
                } else {
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
            }
            ThumbnailPosition::Right => {
                if self.thumbnail_collapsed {
                    egui::SidePanel::right("thumbnails_right")
                        .exact_width(24.0)
                        .frame(egui::Frame::none()
                            .fill(Color32::from_rgb(25, 25, 28))
                            .inner_margin(Margin::symmetric(8.0, 4.0)))
                        .show(ctx, |ui| {
                            ui.vertical_centered(|ui| {
                                if ui.button("⊞").clicked() {
                                    self.thumbnail_collapsed = false;
                                }
                            });
                        });
                } else {
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
    }
    
    fn render_thumbnail_contents(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, horizontal: bool) {
        let thumb_size = self.settings.thumbnail_size;
        let spacing = if horizontal { 4.0 } else { 4.0 };
        let extra_height = if self.settings.show_thumbnail_labels { 18.0 } else { 0.0 };
        let item_width = if horizontal { thumb_size + spacing } else { thumb_size };
        let item_height = thumb_size + extra_height + if horizontal { 0.0 } else { spacing };

        let total_items = self.filtered_list.len();
        if total_items == 0 {
            return;
        }

        if horizontal {
            let total_width = total_items as f32 * item_width;
            let content_size = Vec2::new(total_width, item_height);

            ui.vertical(|ui| {


                // Thumbnail row with collapse button and thumbnails scroll area
                ui.horizontal(|ui| {
                    if ui.small_button("✕").clicked() {
                        self.thumbnail_collapsed = true;
                    }

                    egui::ScrollArea::horizontal()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.allocate_space(content_size);
                            self.render_visible_thumbnails(ui, ctx, thumb_size, horizontal, spacing, extra_height, item_width, item_height);
                        });
                });
            });
        } else {
            let total_height = total_items as f32 * item_height;
            let content_size = Vec2::new(item_width, total_height);

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.small_button("✕").clicked() {
                        self.thumbnail_collapsed = true;
                    }
                });
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.allocate_space(content_size);
                        self.render_visible_thumbnails(ui, ctx, thumb_size, horizontal, spacing, extra_height, item_width, item_height);
                    });
            });
        }
    }
    
    fn render_visible_thumbnails(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, thumb_size: f32, horizontal: bool, spacing: f32, extra_height: f32, _item_width: f32, _item_height: f32) {
        let visible_rect = ui.clip_rect();
        let content_rect = ui.min_rect();

        let item_width = if horizontal { thumb_size + spacing } else { thumb_size };
        let item_height = thumb_size + extra_height + if horizontal { 0.0 } else { spacing };

        let (scroll_offset, visible_size) = if horizontal {
            (-content_rect.left(), visible_rect.width())
        } else {
            (-content_rect.top(), visible_rect.height())
        };

        let step = if horizontal { item_width } else { item_height };

        let start_idx = (scroll_offset / step).floor() as usize;
        let end_idx = ((scroll_offset + visible_size) / step).ceil() as usize;
        let end_idx = end_idx.min(self.filtered_list.len());

        // Request thumbnails and EXIF for visible items
        for display_idx in start_idx..end_idx {
            if let Some(&real_idx) = self.filtered_list.get(display_idx) {
                if let Some(path) = self.image_list.get(real_idx).cloned() {
                    if !self.thumbnail_textures.contains_key(&path) && !self.thumbnail_requests.contains(&path) {
                        self.ensure_thumbnail_requested(&path, ctx);
                    }
                    // Request EXIF if not cached and labels are enabled
                    if self.settings.show_thumbnail_labels && !self.compare_exifs.contains_key(&path) {
                        self.load_exif_data(&path);
                    }
                }
            }
        }

        // Render visible thumbnails
        for display_idx in start_idx..end_idx {
            if let Some(&real_idx) = self.filtered_list.get(display_idx) {
                if let Some(path) = self.image_list.get(real_idx).cloned() {
                    let pos = if horizontal {
                        egui::pos2(content_rect.left() + display_idx as f32 * item_width, content_rect.top())
                    } else {
                        egui::pos2(content_rect.left(), content_rect.top() + display_idx as f32 * item_height)
                    };

                    self.render_single_thumbnail(ui, ctx, thumb_size, extra_height, pos, display_idx, &path);
                }
            }
        }
    }
}

impl ImageViewerApp {
    fn render_single_thumbnail(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, thumb_size: f32, extra_height: f32, pos: egui::Pos2, display_idx: usize, path: &std::path::PathBuf) {
        let is_current = display_idx == self.current_index;
        let is_selected = self.selected_indices.contains(&display_idx);
        let tex_id = self.thumbnail_textures.get(path).map(|h| h.id());
        let metadata = self.metadata_db.get(path);

        let rect = Rect::from_min_size(pos, Vec2::new(thumb_size, thumb_size + extra_height));
        let image_area = Rect::from_min_size(pos, Vec2::new(thumb_size, thumb_size));

        // Check if this thumbnail is visible
        if !ui.clip_rect().intersects(rect) {
            return;
        }

        let painter = ui.painter();

        // Background and selection (applies only to the image area)
        let bg_color = if is_current {
            Color32::from_rgb(70, 130, 255)
        } else if is_selected {
            Color32::from_rgb(50, 90, 180)
        } else if rect.contains(ui.input(|i| i.pointer.hover_pos().unwrap_or_default())) {
            Color32::from_rgb(50, 50, 55)
        } else {
            Color32::from_rgb(35, 35, 40)
        };

        painter.rect_filled(image_area, Rounding::same(4.0), bg_color);

        // Thumbnail image (preserve original aspect ratio) inside the reserved image area
        if let Some(tex_id) = tex_id {
            let inner_rect = image_area.shrink(3.0);
            // Determine texture pixel size and scale to fit inside inner_rect while preserving aspect ratio
            let tex_size = self.texture_size_from_id(tex_id);
            let scale = (inner_rect.width() / tex_size.x).min(inner_rect.height() / tex_size.y);
            let display_size = tex_size * scale;
            let image_rect = Rect::from_center_size(inner_rect.center(), display_size);
            painter.image(
                tex_id,
                image_rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            // Loading indicator - spinning animation (in image area)
            let spinner_char = self.spinner_char(ui);
            painter.text(
                image_area.center(),
                egui::Align2::CENTER_CENTER,
                spinner_char,
                egui::FontId::proportional(18.0),
                Color32::from_rgb(100, 100, 100),
            );
            // Request repaint for animation
            ui.ctx().request_repaint();
        }

        // Rating stars (bottom left)
        if metadata.rating > 0 {
            painter.text(
                image_area.left_bottom() + Vec2::new(3.0, -3.0),
                egui::Align2::LEFT_BOTTOM,
                "★".repeat(metadata.rating as usize),
                egui::FontId::proportional(8.0),
                Color32::from_rgb(255, 200, 50),
            );
        }

        // Color label dot (top right)
        if metadata.color_label != ColorLabel::None {
            painter.circle_filled(
                image_area.right_top() + Vec2::new(-6.0, 6.0),
                4.0,
                metadata.color_label.to_color(),
            );
        }

        // Filename and resolution label under thumbnail (optional)
        if self.settings.show_thumbnail_labels {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                let mut info = String::new();
                // Prefer cached EXIF dimensions if available, else texture size
                if let Some(exif) = self.compare_exifs.get(path) {
                    if let Some(dim) = exif.dimensions.clone() {
                        info = dim;
                    }
                } else if let Some(tex_id) = tex_id {
                    let tex_size = self.texture_size_from_id(tex_id);
                    info = format!("{} × {}", tex_size.x as i32, tex_size.y as i32);
                }

                let label = if info.is_empty() { file_name.to_string() } else { format!("{} • {}", file_name, info) };
                let label_pos = egui::pos2(rect.center().x, image_area.bottom() + 2.0);
                painter.text(
                    label_pos,
                    egui::Align2::CENTER_TOP,
                    label,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(200, 200, 200),
                );
            }
        }

        // Handle interactions
        let response = ui.interact(rect, egui::Id::new(format!("thumb_{}", display_idx)), egui::Sense::click());

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

        // Double-click: open image
        if response.double_clicked() {
            self.current_index = display_idx;
            self.load_current_image();
        }

        // Context menu
        response.context_menu(|ui| {
            if ui.button("View").clicked() {
                self.current_index = display_idx;
                self.load_current_image();
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
