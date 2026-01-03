use crate::app::{ImageViewerApp, ViewMode};
use crate::gpu::types::GpuProcessor;

impl eframe::App for ImageViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::profiler::with_profiler(|p| p.start_timer("ui_update"));

        self.ctx = Some(ctx.clone());

        // Initialize GPU processor synchronously on first update
        if !self.gpu_initialization_attempted {
            self.gpu_initialization_attempted = true;
            // Use pollster to block on the async GPU initialization
            // This is acceptable since GPU init is typically fast and only happens once
            match pollster::block_on(GpuProcessor::new()) {
                Ok(processor) => {
                    self.gpu_processor = Some(std::sync::Arc::new(processor));
                    self.set_status_message("GPU acceleration enabled".to_string());
                }
                Err(e) => {
                    log::error!("Failed to initialize GPU processor: {}", e);
                    self.set_status_message(format!("GPU initialization failed: {}", e));
                }
            }
        }

        // Process async messages
        self.process_loader_messages(ctx);

        // Periodic cleanup of unused textures (every 100 frames)
        static mut FRAME_COUNTER: u32 = 0;
        unsafe {
            FRAME_COUNTER += 1;
            if FRAME_COUNTER.is_multiple_of(100) {
                self.cleanup_unused_textures();
            }
        }

        // Handle keyboard input
        self.handle_keyboard(ctx);

        // Update slideshow
        self.update_slideshow(ctx);

        // Animate zoom/pan
        self.animate_view(ctx);

        // Apply theme
        crate::ui::main::theme::apply_theme(ctx, &self.settings);

        // Menu bar removed per user request

        // Handle dropped files
        self.handle_dropped_files(ctx);

        // Render dialogs
        self.render_dialogs(ctx);

        // Tabs disabled — do not render tab bar

        // Render UI based on view mode
        match self.view_mode {
            ViewMode::Single => {
                if self.settings.show_toolbar {
                    self.render_toolbar(ctx);
                }
                if self.settings.show_statusbar {
                    self.render_statusbar(ctx);
                }
                if !self.panels_hidden {
                    // Render thumbnail bar before side panels so it spans full width
                    self.render_thumbnail_bar(ctx);
                    self.render_navigator_left_panel(ctx);
                    self.render_sidebar(ctx);
                }
                self.render_main_view(ctx);
            }
            ViewMode::Lightbox => {
                if self.settings.show_toolbar {
                    self.render_toolbar(ctx);
                }
                self.render_lightbox(ctx);
            }
            ViewMode::Compare => {
                if self.settings.show_toolbar {
                    self.render_toolbar(ctx);
                }
                if self.settings.show_statusbar {
                    self.render_statusbar(ctx);
                }
                if !self.panels_hidden {
                    self.render_navigator_left_panel(ctx);
                    self.render_sidebar(ctx);
                }
                // Call the public wrapper
                self.render_compare_view_public(ctx);
            }
        }

        // Process pending navigation actions (deferred to avoid UI blocking)
        if self.pending_navigate_prev {
            self.previous_image();
        }
        if self.pending_navigate_next {
            self.next_image();
        }
        if self.pending_navigate_first {
            self.go_to_first();
        }
        if self.pending_navigate_last {
            self.go_to_last();
        }
        if self.pending_navigate_page_up {
            for _ in 0..10 {
                self.previous_image();
            }
        }
        if self.pending_navigate_page_down {
            for _ in 0..10 {
                self.next_image();
            }
        }
        if self.pending_fit_to_window {
            self.fit_to_window_internal();
        }

        // Reset pending flags
        self.pending_navigate_prev = false;
        self.pending_navigate_next = false;
        self.pending_navigate_first = false;
        self.pending_navigate_last = false;
        self.pending_navigate_page_up = false;
        self.pending_navigate_page_down = false;
        self.pending_fit_to_window = false;

        // Process any pending adjustment changes (deferred for smoother UI)
        self.refresh_adjustments_if_dirty();

        crate::profiler::with_profiler(|p| {
            p.end_timer("ui_update");
            p.increment_counter("ui_updates");
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings.save();
        self.metadata_db.save();
    }
}
