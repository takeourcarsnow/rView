use crate::app::ImageViewerApp;
use egui::{self, RichText};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct BatchOperation {
    pub operation_type: BatchOperationType,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum BatchOperationType {
    Resize { width: u32, height: u32, maintain_aspect: bool },
    ConvertFormat { format: String },
    ApplyAdjustments,
    Rename { pattern: String },
}

pub struct BatchProcessingDialog {
    pub open: bool,
    pub selected_files: Vec<PathBuf>,
    pub operations: Vec<BatchOperation>,
    pub output_directory: Option<PathBuf>,
    pub processing: bool,
    pub progress: f32,
    pub current_file: Option<String>,
}

impl Default for BatchProcessingDialog {
    fn default() -> Self {
        Self {
            open: false,
            selected_files: Vec::new(),
            operations: vec![
                BatchOperation {
                    operation_type: BatchOperationType::Resize {
                        width: 1920,
                        height: 1080,
                        maintain_aspect: true,
                    },
                    enabled: false,
                },
                BatchOperation {
                    operation_type: BatchOperationType::ConvertFormat {
                        format: "jpg".to_string(),
                    },
                    enabled: false,
                },
                BatchOperation {
                    operation_type: BatchOperationType::ApplyAdjustments,
                    enabled: false,
                },
                BatchOperation {
                    operation_type: BatchOperationType::Rename {
                        pattern: "{name}_processed.{ext}".to_string(),
                    },
                    enabled: false,
                },
            ],
            output_directory: None,
            processing: false,
            progress: 0.0,
            current_file: None,
        }
    }
}

impl ImageViewerApp {
    pub fn render_batch_processing_dialog(&mut self, ctx: &egui::Context) {
        if !self.batch_processing_dialog.open {
            return;
        }

        let mut open = self.batch_processing_dialog.open;
        egui::Window::new("Batch Processing")
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(600.0, 500.0))
            .show(ctx, |ui| {
                // File selection
                ui.group(|ui| {
                    ui.label(RichText::new("Selected Files").strong());
                    ui.label(format!("{} files selected", self.batch_processing_dialog.selected_files.len()));

                    ui.horizontal(|ui| {
                        if ui.button("Select Current Folder").clicked() {
                            if let Some(_folder) = &self.current_folder {
                                self.batch_processing_dialog.selected_files = self.image_list.clone();
                            }
                        }
                        if ui.button("Select All Filtered").clicked() {
                            self.batch_processing_dialog.selected_files = self.filtered_list.iter()
                                .filter_map(|&idx| self.image_list.get(idx))
                                .cloned()
                                .collect();
                        }
                        if ui.button("Clear Selection").clicked() {
                            self.batch_processing_dialog.selected_files.clear();
                        }
                    });
                });

                ui.add_space(10.0);

                // Output directory
                ui.group(|ui| {
                    ui.label(RichText::new("Output Directory").strong());
                    ui.horizontal(|ui| {
                        if let Some(dir) = &self.batch_processing_dialog.output_directory {
                            ui.label(dir.display().to_string());
                        } else {
                            ui.label("Same as source");
                        }
                        if ui.button("Choose...").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.batch_processing_dialog.output_directory = Some(path);
                            }
                        }
                        if ui.button("Reset").clicked() {
                            self.batch_processing_dialog.output_directory = None;
                        }
                    });
                });

                ui.add_space(10.0);

                // Operations
                ui.group(|ui| {
                    ui.label(RichText::new("Operations").strong());

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for operation in self.batch_processing_dialog.operations.iter_mut() {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut operation.enabled, "");

                                match &mut operation.operation_type {
                                    BatchOperationType::Resize { width, height, maintain_aspect } => {
                                        ui.label("Resize:");
                                        ui.add(egui::DragValue::new(width).range(1..=10000));
                                        ui.label("x");
                                        ui.add(egui::DragValue::new(height).range(1..=10000));
                                        ui.checkbox(maintain_aspect, "Maintain aspect ratio");
                                    }
                                    BatchOperationType::ConvertFormat { format } => {
                                        ui.label("Convert format:");
                                        egui::ComboBox::from_label("")
                                            .selected_text(format.clone())
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(format, "jpg".to_string(), "JPEG");
                                                ui.selectable_value(format, "png".to_string(), "PNG");
                                                ui.selectable_value(format, "webp".to_string(), "WebP");
                                                ui.selectable_value(format, "tiff".to_string(), "TIFF");
                                            });
                                    }
                                    BatchOperationType::ApplyAdjustments => {
                                        ui.label("Apply current adjustments");
                                    }
                                    BatchOperationType::Rename { pattern } => {
                                        ui.label("Rename:");
                                        ui.text_edit_singleline(pattern);
                                        ui.label("(Use {name}, {ext}, {index})");
                                    }
                                }
                            });
                        }
                    });
                });

                ui.add_space(10.0);

                // Progress
                if self.batch_processing_dialog.processing {
                    ui.group(|ui| {
                        ui.label(RichText::new("Processing...").strong());
                        ui.add(egui::ProgressBar::new(self.batch_processing_dialog.progress).show_percentage());

                        if let Some(file) = &self.batch_processing_dialog.current_file {
                            ui.label(format!("Current: {}", file));
                        }
                    });
                }

                // Buttons
                ui.horizontal(|ui| {
                    if self.batch_processing_dialog.processing {
                        if ui.button("Cancel").clicked() {
                            self.batch_processing_dialog.processing = false;
                            self.batch_processing_dialog.progress = 0.0;
                            self.batch_processing_dialog.current_file = None;
                        }
                    } else if ui.button("Start Processing").clicked() && !self.batch_processing_dialog.selected_files.is_empty() {
                        self.start_batch_processing();
                    }

                    if ui.button("Close").clicked() {
                        self.batch_processing_dialog.open = false;
                    }
                });
            });
        self.batch_processing_dialog.open = open;
    }

    fn start_batch_processing(&mut self) {
        self.batch_processing_dialog.processing = true;
        self.batch_processing_dialog.progress = 0.0;

        let operations: Vec<BatchOperation> = self.batch_processing_dialog.operations.iter()
            .filter(|op| op.enabled)
            .cloned()
            .collect();

        let files = self.batch_processing_dialog.selected_files.len();

        // In a real implementation, this would spawn a background task
        // For now, just show a message
        self.show_status(&format!("Batch processing {} files with {} operations",
            files, operations.len()));
    }
}