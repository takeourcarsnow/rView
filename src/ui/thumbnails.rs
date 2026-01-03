use crate::app::ImageViewerApp;
use crate::settings::{ColorLabel, ThumbnailPosition};
use egui::{self, Color32, CornerRadius, Margin, Rect, Vec2};
use std::path::PathBuf;

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
                egui::TopBottomPanel::bottom("thumbnails")
                    .resizable(false)
                    .exact_height(bar_size)
                    .frame(
                        egui::Frame::NONE
                            .fill(Color32::from_rgb(25, 25, 28))
                            .inner_margin(Margin::symmetric(4, 8)),
                    )
                    .show(ctx, |ui| {
                        self.render_thumbnail_contents(ui, ctx, true);
                    });
            }
            ThumbnailPosition::Top => {
                egui::TopBottomPanel::top("thumbnails_top")
                    .resizable(false)
                    .exact_height(bar_size)
                    .frame(
                        egui::Frame::NONE
                            .fill(Color32::from_rgb(25, 25, 28))
                            .inner_margin(Margin::symmetric(4, 8)),
                    )
                    .show(ctx, |ui| {
                        self.render_thumbnail_contents(ui, ctx, true);
                    });
            }
            ThumbnailPosition::Left => {
                egui::SidePanel::left("thumbnails_left")
                    .resizable(false)
                    .exact_width(bar_size)
                    .frame(
                        egui::Frame::NONE
                            .fill(Color32::from_rgb(25, 25, 28))
                            .inner_margin(Margin::symmetric(8, 4)),
                    )
                    .show(ctx, |ui| {
                        self.render_thumbnail_contents(ui, ctx, false);
                    });
            }
            ThumbnailPosition::Right => {
                egui::SidePanel::right("thumbnails_right")
                    .resizable(false)
                    .exact_width(bar_size)
                    .frame(
                        egui::Frame::NONE
                            .fill(Color32::from_rgb(25, 25, 28))
                            .inner_margin(Margin::symmetric(8, 4)),
                    )
                    .show(ctx, |ui| {
                        self.render_thumbnail_contents(ui, ctx, false);
                    });
            }
        }
    }

    fn render_thumbnail_contents(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        horizontal: bool,
    ) {
        let thumb_size = self.settings.thumbnail_size;
        let spacing = 4.0; // Same for both orientations
        let extra_height = if self.settings.show_thumbnail_labels {
            18.0
        } else {
            0.0
        };
        let item_width = if horizontal {
            thumb_size + spacing
        } else {
            thumb_size
        };
        let item_height = thumb_size + extra_height + if horizontal { 0.0 } else { spacing };

        let total_items = self.filtered_list.len();
        if total_items == 0 {
            return;
        }

        if horizontal {
            let total_width = total_items as f32 * item_width;
            let content_size = Vec2::new(total_width, item_height);

            // Handle vertical mouse wheel for horizontal scrolling
            let scroll_delta = ui.input(|i| i.raw_scroll_delta);
            if scroll_delta.y != 0.0 {
                let hover_pos = ui.input(|i| i.pointer.hover_pos());
                if let Some(pos) = hover_pos {
                    if ui.max_rect().contains(pos) {
                        self.thumbnail_scroll_offset.x += -scroll_delta.y * 20.0;
                    }
                }
            }

            ui.horizontal(|ui| {
                let output = egui::ScrollArea::horizontal()
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                    .scroll_offset(self.thumbnail_scroll_offset)
                    .show(ui, |ui| {
                        ui.allocate_space(content_size);
                        self.render_visible_thumbnails(
                            ui,
                            ctx,
                            thumb_size,
                            horizontal,
                            spacing,
                            extra_height,
                            item_width,
                            item_height,
                        );
                    });
                self.thumbnail_scroll_offset = output.state.offset;
            });
        } else {
            let total_height = total_items as f32 * item_height;
            let content_size = Vec2::new(item_width, total_height);

            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                    .show(ui, |ui| {
                        ui.allocate_space(content_size);
                        self.render_visible_thumbnails(
                            ui,
                            ctx,
                            thumb_size,
                            horizontal,
                            spacing,
                            extra_height,
                            item_width,
                            item_height,
                        );
                    });
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_visible_thumbnails(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        thumb_size: f32,
        horizontal: bool,
        spacing: f32,
        extra_height: f32,
        _item_width: f32,
        _item_height: f32,
    ) {
        let visible_rect = ui.clip_rect();
        let content_rect = ui.min_rect();

        let item_width = if horizontal {
            thumb_size + spacing
        } else {
            thumb_size
        };
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

        // Request thumbnails with priority levels
        let mut priority_requests = Vec::new(); // Current image
        let mut high_priority_requests = Vec::new(); // Adjacent images (±2)
        let mut medium_priority_requests = Vec::new(); // Visible but not adjacent
        let mut low_priority_requests = Vec::new(); // Not currently visible

        for display_idx in start_idx..end_idx {
            if let Some(&real_idx) = self.filtered_list.get(display_idx) {
                if let Some(path) = self.image_list.get(real_idx).cloned() {
                    if !self.thumbnail_textures.contains_key(&path)
                        && !self.thumbnail_requests.contains(&path)
                    {
                        let distance = (display_idx as isize - self.current_index as isize).abs();
                        match distance {
                            0 => priority_requests.push(path.clone()), // Current image - highest priority
                            1..=2 => high_priority_requests.push(path.clone()), // Adjacent images
                            _ => medium_priority_requests.push(path.clone()), // Visible but not adjacent
                        }
                    }
                    // Request EXIF if not cached and labels are enabled
                    if self.settings.show_thumbnail_labels
                        && !self.compare_exifs.contains_key(&path)
                    {
                        self.load_exif_data(&path);
                    }
                }
            }
        }

        // Also request thumbnails for a few images outside the visible area (background loading)
        let preload_count = 10;
        for offset in 1..=preload_count {
            // Before visible area
            if let Some(display_idx) = start_idx.checked_sub(offset) {
                if let Some(&real_idx) = self.filtered_list.get(display_idx) {
                    if let Some(path) = self.image_list.get(real_idx).cloned() {
                        if !self.thumbnail_textures.contains_key(&path)
                            && !self.thumbnail_requests.contains(&path)
                        {
                            low_priority_requests.push(path);
                        }
                    }
                }
            }
            // After visible area
            let display_idx = end_idx + offset - 1;
            if display_idx < self.filtered_list.len() {
                if let Some(&real_idx) = self.filtered_list.get(display_idx) {
                    if let Some(path) = self.image_list.get(real_idx).cloned() {
                        if !self.thumbnail_textures.contains_key(&path)
                            && !self.thumbnail_requests.contains(&path)
                        {
                            low_priority_requests.push(path);
                        }
                    }
                }
            }
        }

        // Process requests in priority order
        for path in priority_requests {
            self.ensure_thumbnail_requested(&path, ctx);
        }
        for path in high_priority_requests {
            self.ensure_thumbnail_requested(&path, ctx);
        }
        for path in medium_priority_requests {
            self.ensure_thumbnail_requested(&path, ctx);
        }
        // Limit low priority requests to prevent overwhelming the loader
        for path in low_priority_requests.into_iter().take(5) {
            self.ensure_thumbnail_requested(&path, ctx);
        }

        // Render visible thumbnails
        for display_idx in start_idx..end_idx {
            if let Some(&real_idx) = self.filtered_list.get(display_idx) {
                if let Some(path) = self.image_list.get(real_idx).cloned() {
                    let pos = if horizontal {
                        egui::pos2(
                            content_rect.left() + display_idx as f32 * item_width,
                            content_rect.top(),
                        )
                    } else {
                        egui::pos2(
                            content_rect.left(),
                            content_rect.top() + display_idx as f32 * item_height,
                        )
                    };

                    self.render_single_thumbnail(
                        ui,
                        ctx,
                        thumb_size,
                        extra_height,
                        pos,
                        display_idx,
                        &path,
                    );
                }
            }
        }
    }
}

impl ImageViewerApp {
    #[allow(clippy::too_many_arguments)]
    fn render_single_thumbnail(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        thumb_size: f32,
        extra_height: f32,
        pos: egui::Pos2,
        display_idx: usize,
        path: &std::path::PathBuf,
    ) {
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

        painter.rect_filled(image_area, CornerRadius::same(4), bg_color);

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

                let label = if info.is_empty() {
                    file_name.to_string()
                } else {
                    format!("{} • {}", file_name, info)
                };
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
        let response = ui.interact(
            rect,
            egui::Id::new(format!("thumb_{}", display_idx)),
            egui::Sense::click_and_drag(),
        );

        // Drag source for drag-and-drop to collections
        response.dnd_set_drag_payload(path.clone());

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
                // Single select - use go_to_index to properly save/load adjustments
                self.selected_indices.clear();
                self.go_to_index(display_idx);
            }
        }

        // Double-click: open image
        if response.double_clicked() {
            self.go_to_index(display_idx);
        }

        // Context menu
        response.context_menu(|ui| {
            if ui.button("View").clicked() {
                self.go_to_index(display_idx);
                ui.close_menu();
            }

            ui.separator();
            if ui.button("Delete").clicked() {
                self.current_index = display_idx;
                self.delete_current_image();
                ui.close_menu();
            }
            ui.separator();

            // Add to Collection submenu
            if let Some(ref catalog_db) = self.catalog_db {
                if let Ok(collections) = catalog_db.get_collections() {
                    if !collections.is_empty() {
                        ui.menu_button("Add to Collection", |ui| {
                            for collection in collections {
                                let label = format!("📁 {}", collection.name);
                                if ui.button(&label).clicked() {
                                    // Add all selected images to collection, or current image if none selected
                                    if !self.selected_indices.is_empty() {
                                        let mut added_count = 0;
                                        // Collect paths first to avoid borrowing issues
                                        let paths_to_add: Vec<PathBuf> = self
                                            .selected_indices
                                            .iter()
                                            .filter_map(|&selected_idx| {
                                                self.filtered_list.get(selected_idx).and_then(
                                                    |&real_idx| {
                                                        self.image_list.get(real_idx).cloned()
                                                    },
                                                )
                                            })
                                            .collect();

                                        for path in paths_to_add {
                                            if self
                                                .add_path_to_collection(path, collection.id)
                                                .is_ok()
                                            {
                                                added_count += 1;
                                            }
                                        }
                                        if added_count > 0 {
                                            self.set_status_message(format!(
                                                "Added {} images to collection",
                                                added_count
                                            ));
                                        }
                                    } else {
                                        self.current_index = display_idx;
                                        self.add_current_to_collection(collection.id);
                                    }
                                    ui.close_menu();
                                }
                            }
                        });
                        ui.separator();
                    }
                }
            }

            ui.menu_button("Rating", |ui| {
                for r in 0..=5 {
                    let stars = if r == 0 {
                        "None".to_string()
                    } else {
                        "★".repeat(r)
                    };
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
