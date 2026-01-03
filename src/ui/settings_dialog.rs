use crate::app::ImageViewerApp;
use crate::settings::{BackgroundColor, FocusPeakingColor, GridType, Theme, ThumbnailPosition};
use egui::{self, Color32, RichText, Vec2};

impl ImageViewerApp {
    pub fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_settings_dialog {
            return;
        }

        // Close on escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_settings_dialog = false;
            return;
        }

        let screen_rect = ctx.screen_rect();
        let max_height = (screen_rect.height() - 100.0).max(300.0);

        egui::Window::new("⚙ Settings")
            .collapsible(false)
            .resizable(true)
            .default_width(480.0)
            .max_height(max_height)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(max_height - 80.0)
                    .show(ui, |ui| {
                        self.render_appearance_settings(ui);
                        self.render_view_settings(ui);
                        self.render_photography_tools_settings(ui);
                        self.render_slideshow_settings(ui);
                        self.render_cache_settings(ui);
                        self.render_performance_settings(ui);
                        self.render_gpu_info(ui);
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui
                        .add_sized(Vec2::new(80.0, 28.0), egui::Button::new("✓ Close"))
                        .clicked()
                    {
                        self.show_settings_dialog = false;
                    }
                    ui.add_space(8.0);
                    if ui
                        .add_sized(
                            Vec2::new(120.0, 28.0),
                            egui::Button::new("↺ Reset Defaults"),
                        )
                        .clicked()
                    {
                        self.settings = crate::settings::Settings::default();
                    }
                });
            });
    }

    fn render_appearance_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Appearance");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Theme:");
            egui::ComboBox::from_id_salt("theme_combo")
                .selected_text(format!("{:?}", self.settings.theme))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.settings.theme, Theme::Dark, "Dark");
                    ui.selectable_value(&mut self.settings.theme, Theme::Light, "Light");
                    ui.selectable_value(&mut self.settings.theme, Theme::Oled, "OLED Black");
                    ui.selectable_value(&mut self.settings.theme, Theme::System, "System");
                });
        });

        ui.horizontal(|ui| {
            ui.label("Background:");
            egui::ComboBox::from_id_salt("bg_combo")
                .selected_text(format!("{:?}", self.settings.background_color))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.background_color,
                        BackgroundColor::Black,
                        "Black",
                    );
                    ui.selectable_value(
                        &mut self.settings.background_color,
                        BackgroundColor::Dark,
                        "Dark",
                    );
                    ui.selectable_value(
                        &mut self.settings.background_color,
                        BackgroundColor::Gray,
                        "Gray",
                    );
                    ui.selectable_value(
                        &mut self.settings.background_color,
                        BackgroundColor::Light,
                        "Light",
                    );
                    ui.selectable_value(
                        &mut self.settings.background_color,
                        BackgroundColor::Checkered,
                        "Checkered",
                    );
                });
        });

        ui.add_space(12.0);
        ui.heading("Thumbnails");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Position:");
            egui::ComboBox::from_id_salt("thumb_pos")
                .selected_text(format!("{:?}", self.settings.thumbnail_position))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.thumbnail_position,
                        ThumbnailPosition::Bottom,
                        "Bottom",
                    );
                    ui.selectable_value(
                        &mut self.settings.thumbnail_position,
                        ThumbnailPosition::Top,
                        "Top",
                    );
                    ui.selectable_value(
                        &mut self.settings.thumbnail_position,
                        ThumbnailPosition::Left,
                        "Left",
                    );
                    ui.selectable_value(
                        &mut self.settings.thumbnail_position,
                        ThumbnailPosition::Right,
                        "Right",
                    );
                });
        });

        ui.horizontal(|ui| {
            ui.label("Size:");
            ui.add(egui::Slider::new(&mut self.settings.thumbnail_size, 50.0..=200.0).suffix("px"));
        });

        ui.add_space(12.0);
        ui.heading("Panels");
        ui.add_space(4.0);

        ui.checkbox(&mut self.settings.show_sidebar, "Show sidebar");
        ui.checkbox(&mut self.settings.show_thumbnails, "Show thumbnails");
        ui.checkbox(&mut self.settings.show_exif, "Show EXIF panel");
        ui.checkbox(&mut self.settings.show_histogram, "Show histogram");
        ui.checkbox(
            &mut self.settings.show_adjustments,
            "Show adjustments panel",
        );
        ui.checkbox(&mut self.settings.show_toolbar, "Show toolbar");
        ui.checkbox(&mut self.settings.show_statusbar, "Show status bar");
    }

    fn render_view_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("View");
        ui.add_space(4.0);

        ui.checkbox(&mut self.settings.smooth_zoom, "Smooth zoom animation");
        ui.checkbox(
            &mut self.settings.maintain_zoom_on_navigate,
            "Keep zoom when navigating",
        );
        ui.checkbox(
            &mut self.settings.maintain_pan_on_navigate,
            "Keep pan position when navigating",
        );
        ui.checkbox(
            &mut self.settings.auto_rotate_exif,
            "Auto-rotate based on EXIF",
        );

        ui.horizontal(|ui| {
            ui.label("Grid overlay:");
            egui::ComboBox::from_id_salt("grid_type")
                .selected_text(format!("{:?}", self.settings.grid_type))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.settings.grid_type, GridType::Off, "Off");
                    ui.selectable_value(
                        &mut self.settings.grid_type,
                        GridType::RuleOfThirds,
                        "Rule of Thirds",
                    );
                    ui.selectable_value(
                        &mut self.settings.grid_type,
                        GridType::GoldenRatio,
                        "Golden Ratio",
                    );
                    ui.selectable_value(
                        &mut self.settings.grid_type,
                        GridType::Diagonal,
                        "Diagonal",
                    );
                    ui.selectable_value(&mut self.settings.grid_type, GridType::Center, "Center");
                });
        });

        // RAW loading option: use embedded previews only to avoid heavy RAW decoding
        ui.checkbox(&mut self.settings.load_raw_full_size, "Load full-size RAW files (decode to full resolution). If unchecked, only embedded JPEG previews are used");
    }

    fn render_photography_tools_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Photography Tools");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Focus peaking color:");
            egui::ComboBox::from_id_salt("focus_color")
                .selected_text(format!("{:?}", self.settings.focus_peaking_color))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.focus_peaking_color,
                        FocusPeakingColor::Red,
                        "Red",
                    );
                    ui.selectable_value(
                        &mut self.settings.focus_peaking_color,
                        FocusPeakingColor::Green,
                        "Green",
                    );
                    ui.selectable_value(
                        &mut self.settings.focus_peaking_color,
                        FocusPeakingColor::Blue,
                        "Blue",
                    );
                    ui.selectable_value(
                        &mut self.settings.focus_peaking_color,
                        FocusPeakingColor::Yellow,
                        "Yellow",
                    );
                    ui.selectable_value(
                        &mut self.settings.focus_peaking_color,
                        FocusPeakingColor::White,
                        "White",
                    );
                });
        });

        ui.horizontal(|ui| {
            ui.label("Focus peaking threshold:");
            ui.add(egui::Slider::new(
                &mut self.settings.focus_peaking_threshold,
                10.0..=100.0,
            ));
        });

        ui.horizontal(|ui| {
            ui.label("Zebra high threshold:");
            ui.add(egui::Slider::new(
                &mut self.settings.zebra_high_threshold,
                200..=255,
            ));
        });
    }

    fn render_slideshow_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Slideshow");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Interval:");
            ui.add(
                egui::Slider::new(&mut self.settings.slideshow_interval, 0.5..=30.0).suffix("s"),
            );
        });

        ui.checkbox(&mut self.settings.slideshow_loop, "Loop slideshow");
    }

    fn render_cache_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Cache");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Cache size:");
            ui.add(egui::Slider::new(&mut self.settings.cache_size_mb, 100..=4096).suffix(" MB"));
        });

        ui.horizontal(|ui| {
            ui.label("Preload ahead:");
            ui.add(
                egui::Slider::new(&mut self.settings.preload_adjacent, 0..=10).suffix(" images"),
            );
        });

        // Cache stats
        let stats = self.image_cache.get_stats();
        ui.label(format!(
            "Cache: {} images ({:.1} MB)",
            stats.image_count,
            stats.image_size_bytes as f64 / 1_048_576.0
        ));

        if ui.button("Clear Cache").clicked() {
            self.image_cache.clear();
        }
    }

    fn render_performance_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Performance & Diagnostics");
        ui.add_space(4.0);

        ui.checkbox(&mut self.profiler_enabled, "Enable performance profiling");
        ui.checkbox(
            &mut self.settings.gpu_enabled,
            "Enable GPU acceleration (experimental)",
        );

        if self.profiler_enabled {
            ui.add_space(8.0);

            // Cache statistics
            ui.label(RichText::new("Cache Statistics").strong());
            ui.add_space(2.0);

            let cache_stats = &self.cache_stats;
            ui.label(format!(
                "Images cached: {} / {} ({:.1}%)",
                cache_stats.cached_images,
                cache_stats.total_images,
                if cache_stats.total_images > 0 {
                    (cache_stats.cached_images as f64 / cache_stats.total_images as f64) * 100.0
                } else {
                    0.0
                }
            ));
            ui.label(format!(
                "Cache hit rate: {:.1}%",
                cache_stats.hit_rate() * 100.0
            ));
            ui.label(format!(
                "Memory usage: {:.1} MB",
                cache_stats.memory_usage_mb()
            ));
            ui.label(format!("Evictions: {}", cache_stats.eviction_count));

            ui.add_space(8.0);

            // Loading diagnostics
            ui.label(RichText::new("Loading Performance").strong());
            ui.add_space(2.0);

            let diag = &self.loading_diagnostics;
            ui.label(format!(
                "Total load time: {:.2}s",
                diag.total_load_time.as_secs_f64()
            ));
            ui.label(format!(
                "Average load time: {:.2}s",
                diag.average_load_time().as_secs_f64()
            ));
            ui.label(format!("Images loaded: {}", diag.images_loaded));
            ui.label(format!(
                "Thumbnails generated: {}",
                diag.thumbnails_generated
            ));
            ui.label(format!("Errors encountered: {}", diag.errors_encountered));

            if !diag.bottlenecks.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new("Bottlenecks:").color(Color32::YELLOW));
                for bottleneck in &diag.bottlenecks {
                    ui.label(format!("• {}", bottleneck));
                }
            }

            ui.add_space(8.0);

            // Profiler stats
            ui.label(RichText::new("Performance Timers").strong());
            ui.add_space(2.0);

            let profiler_stats = crate::profiler::with_profiler(|p| p.get_stats());
            for (name, stats) in &profiler_stats.measurements {
                ui.label(format!(
                    "{}: {:.2}ms avg ({} samples)",
                    name,
                    stats.average_time.as_millis(),
                    stats.count
                ));
            }

            for (name, count) in &profiler_stats.counters {
                ui.label(format!("{}: {} times", name, count));
            }

            if ui.button("Reset Profiler").clicked() {
                crate::profiler::with_profiler(|p| p.reset());
            }
        }
    }

    fn render_gpu_info(&mut self, ui: &mut egui::Ui) {
        // GPU Information
        if let Some(ref gpu) = self.gpu_processor {
            ui.add_space(8.0);
            ui.label(RichText::new("GPU Information").strong());
            ui.add_space(2.0);

            let perf_info = gpu.get_performance_info();
            ui.label(format!("Adapter: {}", perf_info.adapter_name));
            ui.label(format!("Backend: {}", perf_info.backend));
            ui.label(format!("Device Type: {}", perf_info.device_type));
            ui.label(format!(
                "Texture Operations: {}",
                if perf_info.supports_texture_operations {
                    "Supported"
                } else {
                    "Not Supported"
                }
            ));
            ui.label(format!(
                "RAW Demosaic: {}",
                if perf_info.supports_raw_demosaic {
                    "Supported"
                } else {
                    "Not Supported"
                }
            ));

            let adapter_info = gpu.adapter_info();
            ui.label(format!("Driver: {}", adapter_info.driver));
            ui.label(format!("Driver Info: {}", adapter_info.driver_info));
            ui.label(format!("Vendor ID: {}", adapter_info.vendor));
            ui.label(format!("Device ID: {}", adapter_info.device));
        } else {
            ui.add_space(8.0);
            ui.label(RichText::new("GPU Information").strong());
            ui.add_space(2.0);
            ui.label(RichText::new("GPU acceleration not available").color(Color32::YELLOW));
        }
    }
}
