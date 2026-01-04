use eframe::egui::{self, Ui, Color32, RichText, Stroke, Vec2};

use crate::app::ImageViewerApp;
use crate::catalog::CollectionType;

const LR_TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 140);

impl ImageViewerApp {
    /// Render the catalog panel (sidebar)
    pub fn render_catalog_panel(&mut self, ui: &mut Ui) {
        // Compact catalog panel header: removed the large "Catalog" heading per user request
        ui.add_space(4.0);

        // Library section
        ui.label("LIBRARY");

        // All Photos
        if ui
            .selectable_label(
                self.catalog_view_active && self.catalog_show_all_photos,
                "📷 All Photos",
            )
            .clicked()
        {
            self.catalog_view_active = true;
            self.catalog_show_all_photos = true;
            self.catalog_selected_collection = None;
            self.load_catalog_all_photos();
        }

        ui.separator();
        // New collection button on its own line
        ui.horizontal(|ui| {
            if ui.button("➕ New Collection").clicked() {
                self.catalog_show_new_collection_dialog = true;
            }
        });

        // Collapsible triangle toggle below the New Collection button — symbol only, no text or frame
        ui.horizontal(|ui| {
            let symbol = if self.catalog_collections_open {
                "v"
            } else {
                ">"
            };
            let triangle = egui::Button::new(RichText::new(symbol).size(8.0).color(LR_TEXT_SECONDARY))
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .min_size(Vec2::new(14.0, 14.0));
            if ui.add(triangle).clicked() {
                self.catalog_collections_open = !self.catalog_collections_open;
            }
        });

        // List collections (when expanded) - collect actions to perform after iteration
        let mut collection_to_delete: Option<i64> = None;
        let mut collection_to_select: Option<i64> = None;
        let mut paths_to_add: Vec<(std::path::PathBuf, i64)> = Vec::new();

        if self.catalog_collections_open {
            if let Some(ref catalog_db) = self.catalog_db {
                if let Ok(collections) = catalog_db.get_collections() {
                    for collection in collections {
                        let is_selected = self.catalog_selected_collection == Some(collection.id);
                        let label = format!("📁 {} ({})", collection.name, collection.image_count);
                        let collection_id = collection.id;

                        // Make collection droppable for drag-and-drop from thumbnails
                        let (drop_response, dropped_payload) = ui
                            .dnd_drop_zone::<std::path::PathBuf, _>(
                                egui::Frame::NONE,
                                |ui: &mut egui::Ui| ui.selectable_label(is_selected, &label),
                            );

                        if let Some(payload) = dropped_payload {
                            // Queue path to add to collection
                            paths_to_add.push(((*payload).clone(), collection_id));
                        }

                        // Use .inner to get the selectable_label response
                        if drop_response.inner.clicked() {
                            collection_to_select = Some(collection_id);
                        }

                        // Context menu for collection actions
                        drop_response.inner.context_menu(|ui| {
                            if ui.button("🗑 Delete Collection").clicked() {
                                collection_to_delete = Some(collection_id);
                                ui.close_menu();
                            }
                        });
                    }
                }
            }
        }

        // Process queued actions after iteration
        for (path, collection_id) in paths_to_add {
            let _ = self.add_path_to_collection(path, collection_id);
        }

        if let Some(collection_id) = collection_to_select {
            self.catalog_view_active = true;
            self.catalog_show_all_photos = false;
            self.catalog_selected_collection = Some(collection_id);
            self.load_catalog_collection(collection_id);
        }

        if let Some(collection_id) = collection_to_delete {
            self.delete_catalog_collection(collection_id);
        }

        ui.separator();

        // Import button
        if ui.button("📥 Import Folder").clicked() {
            self.catalog_show_import_dialog = true;
        }

        // Catalog stats
        if let Some(ref catalog_db) = self.catalog_db {
            if let Ok(count) = catalog_db.get_image_count() {
                ui.separator();
                ui.label(format!("Total Images: {}", count));
            }
        }
    }

    /// Render import dialog
    pub fn render_catalog_import_dialog(&mut self, ctx: &egui::Context) {
        if !self.catalog_show_import_dialog {
            return;
        }

        let mut open = true;
        egui::Window::new("Import Folder to Catalog")
            .open(&mut open)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Folder:");
                    ui.text_edit_singleline(&mut self.catalog_import_path);
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.catalog_import_path = path.to_string_lossy().to_string();
                        }
                    }
                });

                ui.checkbox(&mut self.catalog_import_recursive, "Include subfolders");

                ui.separator();

                // Show progress if importing
                // TODO: Add progress tracking
                if ui.button("Import").clicked() {
                    self.start_catalog_import();
                }
            });

        if !open {
            self.catalog_show_import_dialog = false;
        }
    }

    /// Render new collection dialog
    pub fn render_new_collection_dialog(&mut self, ctx: &egui::Context) {
        if !self.catalog_show_new_collection_dialog {
            return;
        }

        let mut open = true;
        egui::Window::new("New Collection")
            .open(&mut open)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.label("Collection Name:");
                ui.text_edit_singleline(&mut self.catalog_new_collection_name);

                ui.separator();

                ui.label("Type:");
                ui.radio_value(
                    &mut self.catalog_new_collection_type,
                    CollectionType::Regular,
                    "Regular Collection (manual)",
                );
                ui.radio_value(
                    &mut self.catalog_new_collection_type,
                    CollectionType::Smart,
                    "Smart Collection (auto-populated)",
                );

                ui.separator();

                // Show info about selected images
                let selected_count = self.selected_indices.len();
                if selected_count > 0 {
                    ui.label(format!(
                        "📸 {} selected image(s) will be added to this collection",
                        selected_count
                    ));
                    ui.separator();
                }

                if ui.button("Create").clicked() {
                    self.create_catalog_collection();
                    self.catalog_show_new_collection_dialog = false;
                }
            });

        if !open {
            self.catalog_show_new_collection_dialog = false;
        }
    }

    /// Load all photos from catalog
    pub fn load_catalog_all_photos(&mut self) {
        if let Some(ref catalog_db) = self.catalog_db {
            if let Ok(images) = catalog_db.get_all_images() {
                self.image_list = images.iter().map(|img| img.file_path.clone()).collect();
                self.filtered_list = (0..self.image_list.len()).collect();
                self.current_index = 0;
                if !self.image_list.is_empty() {
                    self.load_current_image();
                }
            }
        }
    }

    /// Load collection images
    pub fn load_catalog_collection(&mut self, collection_id: i64) {
        if let Some(ref catalog_db) = self.catalog_db {
            if let Ok(images) = catalog_db.get_collection_images(collection_id) {
                self.image_list = images.iter().map(|img| img.file_path.clone()).collect();
                self.filtered_list = (0..self.image_list.len()).collect();
                self.current_index = 0;
                if !self.image_list.is_empty() {
                    self.load_current_image();
                }
            }
        }
    }

    /// Start catalog import process
    pub fn start_catalog_import(&mut self) {
        let import_path = self.catalog_import_path.clone();
        let recursive = self.catalog_import_recursive;

        if import_path.is_empty() {
            return;
        }

        if let Some(ref mut catalog_db) = self.catalog_db {
            let options = crate::catalog::import::ImportOptions {
                recursive,
                skip_existing: true,
                extract_exif: false,
            };

            // For now, do a simple blocking import
            // In production, this should be async
            match catalog_db.import_folder(&import_path, options, None) {
                Ok(count) => {
                    self.set_status_message(format!("Imported {} images", count));
                    self.catalog_show_import_dialog = false;
                    self.load_catalog_all_photos();
                }
                Err(e) => {
                    self.set_status_message(format!("Import failed: {}", e));
                }
            }
        }
    }

    /// Create a new collection
    pub fn create_catalog_collection(&mut self) {
        let name = self.catalog_new_collection_name.clone();
        let collection_type = self.catalog_new_collection_type.clone();

        if name.is_empty() {
            return;
        }

        if let Some(ref mut catalog_db) = self.catalog_db {
            match catalog_db.create_collection(&name, collection_type, None, "") {
                Ok(id) => {
                    self.set_status_message(format!("Created collection: {}", name));
                    self.catalog_new_collection_name.clear();
                    self.catalog_selected_collection = Some(id);

                    // Auto-add selected images to the new collection
                    let selected_count = self.selected_indices.len();
                    if selected_count > 0 {
                        // Collect paths first to avoid borrowing issues
                        let selected_paths: Vec<std::path::PathBuf> = self
                            .selected_indices
                            .iter()
                            .filter_map(|&display_idx| {
                                self.filtered_list
                                    .get(display_idx)
                                    .and_then(|&real_idx| self.image_list.get(real_idx))
                                    .cloned()
                            })
                            .collect();

                        let mut added_count = 0;
                        for path in selected_paths {
                            if self.add_path_to_collection(path, id).is_ok() {
                                added_count += 1;
                            }
                        }
                        if added_count > 0 {
                            self.set_status_message(format!(
                                "Created collection '{}' with {} images",
                                name, added_count
                            ));
                            // Refresh the collection view
                            self.load_catalog_collection(id);
                        }
                    }
                }
                Err(e) => {
                    self.set_status_message(format!("Failed to create collection: {}", e));
                }
            }
        }
    }

    /// Add current image to a collection
    pub fn add_current_to_collection(&mut self, collection_id: i64) {
        if let Some(current_path) = self.get_current_path() {
            let _ = self.add_path_to_collection(current_path, collection_id);
        }
    }

    /// Add a specific image path to a collection
    pub fn add_path_to_collection(
        &mut self,
        path: std::path::PathBuf,
        collection_id: i64,
    ) -> Result<(), ()> {
        if let Some(ref mut catalog_db) = self.catalog_db {
            // Get or import image
            let image_id = if let Ok(Some(img)) = catalog_db.get_image(&path) {
                img.id
            } else {
                match catalog_db.import_image(&path) {
                    Ok(id) => id,
                    Err(e) => {
                        self.set_status_message(format!("Failed to add to collection: {}", e));
                        return Err(());
                    }
                }
            };

            match catalog_db.add_to_collection(collection_id, image_id) {
                Ok(_) => {
                    self.set_status_message("Added image to collection".to_string());
                    // Refresh collection count if currently viewing this collection
                    if self.catalog_selected_collection == Some(collection_id) {
                        self.load_catalog_collection(collection_id);
                    }
                    Ok(())
                }
                Err(e) => {
                    self.set_status_message(format!("Failed to add to collection: {}", e));
                    Err(())
                }
            }
        } else {
            Err(())
        }
    }

    /// Delete a collection from the catalog
    pub fn delete_catalog_collection(&mut self, collection_id: i64) {
        if let Some(ref mut catalog_db) = self.catalog_db {
            match catalog_db.delete_collection(collection_id) {
                Ok(_) => {
                    self.set_status_message("Collection deleted".to_string());

                    // If we were viewing the deleted collection, clear the selection
                    if self.catalog_selected_collection == Some(collection_id) {
                        self.catalog_selected_collection = None;
                        self.catalog_view_active = false;
                        // Clear the image list
                        self.image_list.clear();
                        self.filtered_list.clear();
                    }
                }
                Err(e) => {
                    self.set_status_message(format!("Failed to delete collection: {}", e));
                }
            }
        }
    }
}
