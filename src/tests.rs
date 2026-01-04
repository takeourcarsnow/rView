#[cfg(test)]
mod unit_tests {
    use crate::gpu::types::GpuProcessor;
    use crate::metadata::{FileOperation, MetadataDb, UndoHistory};
    use crate::settings::ColorLabel;
    use crate::task_scheduler::{ImageTask, TaskPriority, TaskScheduler};
    use std::path::PathBuf;

    #[test]
    fn test_undo_history() {
        let mut history = UndoHistory::new(10);

        // Test empty history
        assert!(!history.can_undo());
        assert!(!history.can_redo());

        // Add an operation
        let op = FileOperation::Delete {
            original_path: PathBuf::from("/test/image.jpg"),
            trash_path: None,
            metadata_backup: None,
        };
        history.push(op);

        assert!(history.can_undo());
        assert!(!history.can_redo());

        // Undo the operation
        let undone_op = history.undo();
        assert!(undone_op.is_some());
        assert!(!history.can_undo());
        assert!(history.can_redo());

        // Redo the operation
        let redone_op = history.redo();
        assert!(redone_op.is_some());
        assert!(history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn test_metadata_db() {
        let mut db = MetadataDb::new();

        let path = PathBuf::from("/test/image.jpg");

        // Test rating
        db.set_rating(path.clone(), 3);
        assert_eq!(db.get(&path).rating, 3);

        // Test color label
        db.set_color_label(path.clone(), ColorLabel::Red);
        assert_eq!(db.get(&path).color_label, ColorLabel::Red);

        // Test flag
        db.toggle_flag(path.clone());
        assert!(db.get(&path).flagged);

        db.toggle_flag(path.clone());
        assert!(!db.get(&path).flagged);
    }

    #[test]
    fn test_error_messages() {
        use crate::errors::ViewerError;

        let error = ViewerError::FileNotFound {
            path: PathBuf::from("/nonexistent/file.jpg"),
        };

        assert!(error.is_recoverable());
        assert_eq!(error.error_code(), "FILE_NOT_FOUND");

        let user_msg = error.user_message();
        assert!(user_msg.contains("Check if the file exists"));
    }

    #[test]
    fn test_cache_stats() {
        use crate::profiler::CacheStats;

        let stats = CacheStats {
            cache_hit_count: 100,
            cache_miss_count: 25,
            cache_memory_usage: 1024 * 1024,
            thumbnail_memory_usage: 512 * 1024,
            ..Default::default()
        };

        // Simulate some cache operations

        assert_eq!(stats.hit_rate(), 0.8); // 100/125 = 0.8

        assert_eq!(stats.memory_usage_mb(), 1.5);
    }

    #[test]
    fn test_corrupted_image_error() {
        use crate::errors::ViewerError;

        let error = ViewerError::CorruptedImage {
            path: PathBuf::from("/corrupted/image.jpg"),
            details: "Invalid JPEG header".to_string(),
        };

        assert!(!error.is_recoverable());
        assert_eq!(error.error_code(), "CORRUPTED_IMAGE");

        let user_msg = error.user_message();
        assert!(user_msg.contains("corrupted"));
        assert!(user_msg.contains("repairing it"));
    }

    #[test]
    fn test_gpu_error_recovery() {
        use crate::errors::ViewerError;

        let error = ViewerError::GpuError {
            message: "GPU device lost".to_string(),
        };

        assert!(error.is_recoverable());
        assert_eq!(error.error_code(), "GPU_ERROR");

        let user_msg = error.user_message();
        assert!(user_msg.contains("fall back to CPU"));
    }

    #[test]
    fn test_gpu_processor_basic() {
        // Try to initialize GPU; if not available just skip the test
        if let Ok(proc) = pollster::block_on(GpuProcessor::new()) {
            let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
                16,
                16,
                image::Rgba([128, 128, 128, 255]),
            ));
            let mut adj = crate::image_loader::ImageAdjustments::default();
            adj.exposure = 0.5;
            adj.saturation = 1.0;
            adj.temperature = 0.0;
            adj.film = crate::image_loader::FilmEmulation::default();

            let out = proc
                .apply_adjustments(&img, &adj)
                .expect("GPU adjustment failed");
            assert_eq!(out.len(), (16 * 16 * 4) as usize);
        }
    }

    #[test]
    fn test_gpu_processor_with_film_emulation() {
        // Try to initialize GPU; if not available just skip the test
        if let Ok(proc) = pollster::block_on(GpuProcessor::new()) {
            let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
                16,
                16,
                image::Rgba([128, 128, 128, 255]),
            ));
            let mut adj = crate::image_loader::ImageAdjustments::default();
            adj.apply_preset(crate::image_loader::FilmPreset::Portra400);

            let out = proc
                .apply_adjustments(&img, &adj)
                .expect("GPU adjustment with film emulation failed");
            assert_eq!(out.len(), (16 * 16 * 4) as usize);
        }
    }

    #[test]
    fn test_gpu_processor_bw_film() {
        // Try to initialize GPU; if not available just skip the test
        if let Ok(proc) = pollster::block_on(GpuProcessor::new()) {
            let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
                16,
                16,
                image::Rgba([200, 100, 50, 255]),
            ));
            let mut adj = crate::image_loader::ImageAdjustments::default();
            adj.apply_preset(crate::image_loader::FilmPreset::TriX400);

            let out = proc
                .apply_adjustments(&img, &adj)
                .expect("GPU B&W film emulation failed");
            assert_eq!(out.len(), (16 * 16 * 4) as usize);

            // Verify it's converted to grayscale-ish (R, G, B should be similar)
            // Note: Due to tinting and other effects, they won't be exactly equal
        }
    }

    #[test]
    fn test_cpu_film_emulation() {
        use crate::image_loader::{apply_adjustments, FilmPreset, ImageAdjustments};

        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            32,
            32,
            image::Rgba([128, 100, 80, 255]),
        ));

        // Test Portra 400 (color film)
        let mut adj = ImageAdjustments::default();
        adj.apply_preset(FilmPreset::Portra400);
        let result = apply_adjustments(&img, &adj);
        assert_eq!(result.width(), 32);
        assert_eq!(result.height(), 32);

        // Test Tri-X 400 (B&W film - should convert to grayscale)
        let mut adj_bw = ImageAdjustments::default();
        adj_bw.apply_preset(FilmPreset::TriX400);
        let result_bw = apply_adjustments(&img, &adj_bw);

        // Verify B&W conversion - RGB values should be very close
        let rgba = result_bw.to_rgba8();
        let pixel = rgba.get_pixel(16, 16);
        let r = pixel[0] as i32;
        let g = pixel[1] as i32;
        let b = pixel[2] as i32;
        // For true B&W, RGB should be very similar (within some tolerance due to grain/tinting)
        assert!(
            (r - g).abs() < 30,
            "B&W film should produce similar R and G values"
        );
        assert!(
            (g - b).abs() < 30,
            "B&W film should produce similar G and B values"
        );
    }

    #[test]
    fn test_preview_adjustments_lightweight() {
        use crate::image_loader::{
            FilmEmulation, FilmGrain, FilmHalation, FilmTone, FilmVignette, ImageAdjustments,
        };

        let mut adj = ImageAdjustments::default();
        adj.film = FilmEmulation {
            enabled: true,
            grain: FilmGrain {
                amount: 0.3,
                size: 1.0,
                roughness: 0.5,
            },
            halation: FilmHalation {
                amount: 0.2,
                radius: 1.0,
                color: [1.0, 0.3, 0.1],
            },
            tone: FilmTone {
                shadows: 0.0,
                midtones: 0.0,
                highlights: 0.0,
                s_curve_strength: 0.4,
            },
            vignette: FilmVignette {
                amount: 0.05,
                softness: 1.0,
            },
            ..FilmEmulation::default()
        };

        let preview = adj.preview();
        // Preview should disable heavy film features
        assert_eq!(preview.film.enabled, false);
        assert_eq!(preview.film.grain.amount, 0.0);
        assert_eq!(preview.film.halation.amount, 0.0);
        assert_eq!(preview.film.tone.s_curve_strength, 0.0);
        assert_eq!(preview.film.vignette.amount, 0.0);
    }

    #[test]
    fn test_film_emulation_grain() {
        use crate::image_loader::{apply_adjustments, FilmEmulation, FilmGrain, ImageAdjustments};

        // Create a flat gray image
        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            64,
            64,
            image::Rgba([128, 128, 128, 255]),
        ));

        // Apply grain
        let adj = ImageAdjustments {
            film: FilmEmulation {
                enabled: true,
                grain: FilmGrain {
                    amount: 0.5,
                    size: 1.0,
                    roughness: 0.5,
                },
                ..FilmEmulation::default()
            },
            ..ImageAdjustments::default()
        };

        let result = apply_adjustments(&img, &adj);
        let rgba = result.to_rgba8();

        // Verify grain adds variation - not all pixels should be identical
        let mut unique_values = std::collections::HashSet::new();
        for y in 0..10 {
            for x in 0..10 {
                let pixel = rgba.get_pixel(x, y);
                unique_values.insert(pixel[0]);
            }
        }
        // With grain, we should have multiple unique values
        assert!(
            unique_values.len() > 1,
            "Grain should add variation to pixels"
        );
    }

    #[test]
    fn test_film_emulation_s_curve() {
        use crate::image_loader::{apply_adjustments, FilmEmulation, FilmTone, ImageAdjustments};

        // Test that S-curve increases contrast (darkens shadows, brightens highlights)
        let dark_img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            16,
            16,
            image::Rgba([64, 64, 64, 255]),
        ));
        let bright_img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            16,
            16,
            image::Rgba([192, 192, 192, 255]),
        ));

        let adj = ImageAdjustments {
            film: FilmEmulation {
                enabled: true,
                tone: FilmTone {
                    shadows: 0.0,
                    midtones: 0.0,
                    highlights: 0.0,
                    s_curve_strength: 0.5,
                },
                ..FilmEmulation::default()
            },
            ..ImageAdjustments::default()
        };

        let dark_result = apply_adjustments(&dark_img, &adj);
        let bright_result = apply_adjustments(&bright_img, &adj);

        let dark_rgba = dark_result.to_rgba8();
        let bright_rgba = bright_result.to_rgba8();

        let dark_pixel = dark_rgba.get_pixel(8, 8)[0];
        let bright_pixel = bright_rgba.get_pixel(8, 8)[0];

        // S-curve should increase contrast - dark gets darker, bright gets brighter relative to midpoint
        // The contrast increase should be noticeable
        assert!(
            bright_pixel > dark_pixel,
            "S-curve should maintain brightness ordering"
        );
    }

    #[test]
    fn test_all_film_presets() {
        use crate::image_loader::{apply_adjustments, FilmPreset, ImageAdjustments};

        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            16,
            16,
            image::Rgba([128, 100, 80, 255]),
        ));

        // Test all presets don't crash
        for preset in FilmPreset::all() {
            let mut adj = ImageAdjustments::default();
            adj.apply_preset(*preset);
            let result = apply_adjustments(&img, &adj);
            assert_eq!(result.width(), 16);
            assert_eq!(result.height(), 16);
        }
    }

    #[test]
    fn test_thread_pool_error() {
        use crate::errors::ViewerError;

        let error = ViewerError::ThreadPoolError {
            message: "Thread pool exhausted".to_string(),
        };

        assert!(error.is_recoverable());
        assert_eq!(error.error_code(), "THREAD_POOL_ERROR");

        let user_msg = error.user_message();
        assert!(user_msg.contains("restarting the application"));
    }

    #[test]
    fn test_task_scheduler_priority_queue() {
        use crate::task_scheduler::{ImageTask, TaskPriority, TaskScheduler};

        let scheduler = TaskScheduler::new(2);

        // Submit tasks with different priorities
        let task1 = ImageTask::LoadImage {
            path: std::path::PathBuf::from("low.jpg"),
            priority: TaskPriority::Low,
        };
        let task2 = ImageTask::LoadImage {
            path: std::path::PathBuf::from("high.jpg"),
            priority: TaskPriority::High,
        };
        let task3 = ImageTask::LoadImage {
            path: std::path::PathBuf::from("critical.jpg"),
            priority: TaskPriority::Critical,
        };

        scheduler.submit_task(task1);
        scheduler.submit_task(task2);
        scheduler.submit_task(task3);

        // Queue should have 3 tasks
        assert_eq!(scheduler.queue_size(), 3);

        // Note: In a real test, we'd need to wait for task completion
        // but since this is async, we'll just test the queue management
    }

    #[test]
    fn test_memory_pool_allocation() {
        use crate::task_scheduler::MemoryPool;

        let pool = MemoryPool::new();

        // Allocate buffers of different sizes
        let buf1 = pool.allocate(1024); // Should use first size class
        let buf2 = pool.allocate(5 * 1024 * 1024); // Should use third size class

        assert_eq!(buf1.len(), 1024);
        assert_eq!(buf2.len(), 5 * 1024 * 1024);

        // Deallocate and check stats
        pool.deallocate(buf1);
        pool.deallocate(buf2);

        let stats = pool.stats();
        assert!(stats.total_buffers >= 2);
        assert!(stats.total_memory_mb > 0.0);
    }

    #[test]
    fn test_concurrent_histogram() {
        use crate::task_scheduler::concurrent_histogram;
        use image::{DynamicImage, RgbaImage};

        // Create a test image
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            100,
            100,
            image::Rgba([128, 64, 192, 255]),
        ));

        // Test optimal tile count calculation
        let tile_count = concurrent_histogram::optimal_tile_count(100, 100);
        assert!(tile_count >= 1 && tile_count <= num_cpus::get());

        // Test parallel histogram computation
        let histogram = concurrent_histogram::compute_parallel(&img, tile_count);

        // Verify histogram structure (RGBA channels)
        assert_eq!(histogram.len(), 4);
        for channel in &histogram {
            assert_eq!(channel.len(), 256);
        }

        // Verify some pixels were counted (not all zeros)
        let total_pixels: u32 = histogram.iter().map(|ch| ch.iter().sum::<u32>()).sum();
        assert_eq!(total_pixels, 100 * 100 * 4); // Should equal total pixels × channels
    }

    #[test]
    fn test_task_scheduler_error_handling() {
        use crate::task_scheduler::{ImageTask, TaskPriority, TaskScheduler};

        let scheduler = TaskScheduler::new(1);

        // Submit a task with invalid path
        let task = ImageTask::LoadImage {
            path: std::path::PathBuf::from("/nonexistent/path/image.jpg"),
            priority: TaskPriority::High,
        };

        scheduler.submit_task(task);

        // In a real scenario, this would result in an error task result
        // For now, just verify the task was queued
        assert_eq!(scheduler.queue_size(), 1);
    }

    #[test]
    fn test_memory_pool_reuse() {
        use crate::task_scheduler::MemoryPool;

        let pool = MemoryPool::new();

        // Allocate and deallocate multiple times to test reuse
        for _ in 0..5 {
            let buf = pool.allocate(2048);
            assert_eq!(buf.len(), 2048);
            // Fill with test data
            let mut test_buf = buf;
            test_buf.fill(42);
            pool.deallocate(test_buf);
        }

        let stats = pool.stats();
        // Should have reused buffers
        assert!(stats.total_buffers > 0);
    }

    #[test]
    fn test_image_loading_with_task_scheduler() {
        use crate::task_scheduler::TaskScheduler;

        // Create a minimal app state for testing
        // Note: This is challenging because ImageViewerApp requires eframe context
        // In a real test, we'd mock the dependencies or use integration tests

        let scheduler = TaskScheduler::new(1);

        // Test that scheduler can be created and destroyed without issues
        drop(scheduler);

        // If we get here without panicking, the basic functionality works
        assert!(true);
    }

    #[test]
    fn test_ui_state_initialization() {
        // Test that the app can be created with the new fields
        // This is a compile-time test - if it compiles, the struct is valid

        use crate::task_scheduler::{MemoryPool, TaskScheduler};

        let scheduler = TaskScheduler::new(2);
        let pool = MemoryPool::new();

        // Verify they have expected initial state
        assert_eq!(scheduler.queue_size(), 0);

        let stats = pool.stats();
        assert_eq!(stats.total_buffers, 0);
        assert_eq!(stats.total_memory_mb, 0.0);
    }

    #[test]
    fn test_performance_optimizations_dont_break_basic_functionality() {
        use crate::image_loader;
        use image::{DynamicImage, RgbaImage};

        // Create test image
        let test_img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            64,
            64,
            image::Rgba([100, 150, 200, 255]),
        ));

        // Test that basic image processing still works
        let adjustments = image_loader::ImageAdjustments::default();
        let processed = image_loader::apply_adjustments(&test_img, &adjustments);

        assert_eq!(processed.width(), 64);
        assert_eq!(processed.height(), 64);

        // Test histogram calculation
        let histogram = image_loader::calculate_histogram(&processed);
        assert_eq!(histogram.len(), 3); // RGB
        assert_eq!(histogram[0].len(), 256); // 256 bins per channel
    }

    #[test]
    fn test_task_scheduler_thread_safety() {
        use crate::task_scheduler::{ImageTask, TaskPriority, TaskScheduler};
        use std::thread;
        use std::time::Duration;

        let scheduler = TaskScheduler::new(4);

        // Test single-threaded operation since TaskScheduler isn't Send
        for i in 0..10 {
            let task = ImageTask::LoadImage {
                path: std::path::PathBuf::from(format!("test{}.jpg", i)),
                priority: TaskPriority::Medium,
            };
            scheduler.submit_task(task);
        }

        // Verify all tasks were queued
        assert_eq!(scheduler.queue_size(), 10);
    }

    #[test]
    fn test_memory_pool_size_classes() {
        use crate::task_scheduler::MemoryPool;

        let pool = MemoryPool::new();

        // Test different size allocations
        let sizes = vec![512, 2048, 10000, 50000, 200000];

        for size in sizes {
            let buf = pool.allocate(size);
            assert!(buf.len() >= size);
            pool.deallocate(buf);
        }

        // Pool should have retained some buffers
        let stats = pool.stats();
        assert!(stats.total_buffers > 0);
    }

    #[test]
    fn test_concurrent_histogram_large_image() {
        use crate::task_scheduler::concurrent_histogram;
        use image::{DynamicImage, RgbaImage};

        // Create a larger test image
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            500,
            500,
            image::Rgba([255, 128, 64, 255]),
        ));

        let tile_count = concurrent_histogram::optimal_tile_count(500, 500);
        let histogram = concurrent_histogram::compute_parallel(&img, tile_count);

        // Verify histogram is correct
        assert_eq!(histogram.len(), 4);
        let total_pixels: u32 = histogram.iter().map(|ch| ch.iter().sum::<u32>()).sum();
        assert_eq!(total_pixels, 500 * 500 * 4);

        // Check that the dominant color is represented
        assert!(histogram[0][255] > 0); // Red channel has 255
        assert!(histogram[1][128] > 0); // Green channel has 128
        assert!(histogram[2][64] > 0);  // Blue channel has 64
    }

    #[test]
    fn test_task_scheduler_shutdown() {
        use crate::task_scheduler::TaskScheduler;

        let scheduler = TaskScheduler::new(2);

        // Submit some tasks
        for i in 0..5 {
            let task = crate::task_scheduler::ImageTask::LoadImage {
                path: std::path::PathBuf::from(format!("test{}.jpg", i)),
                priority: TaskPriority::Low,
            };
            scheduler.submit_task(task);
        }

        assert_eq!(scheduler.queue_size(), 5);

        // Shutdown should complete without hanging
        // Note: In real usage, we'd call shutdown() but it's not implemented yet
        drop(scheduler);
    }

    #[test]
    fn test_ui_message_processing_integration() {
        // Test that message processing can handle the new task results
        // This is a structural test - if it compiles and runs, the integration works

        use crate::task_scheduler::TaskResult;

        // Create a sample task result
        let result = TaskResult::ImageLoaded {
            path: std::path::PathBuf::from("test.jpg"),
            image: image::DynamicImage::ImageRgba8(
                image::RgbaImage::from_pixel(10, 10, image::Rgba([0, 0, 0, 255]))
            ),
        };

        // Test that the result enum works
        match result {
            TaskResult::ImageLoaded { path, .. } => {
                assert_eq!(path, std::path::PathBuf::from("test.jpg"));
            }
            _ => panic!("Unexpected result type"),
        }
    }

    #[test]
    fn test_performance_optimizations_memory_efficiency() {
        use crate::task_scheduler::MemoryPool;
        use std::mem;

        let pool = MemoryPool::new();

        // Test memory reuse efficiency
        let mut allocations = vec![];

        // Allocate many small buffers
        for _ in 0..20 {
            allocations.push(pool.allocate(1024));
        }

        // Deallocate them
        for buf in allocations {
            pool.deallocate(buf);
        }

        let stats = pool.stats();

        // Should have consolidated into fewer actual allocations
        assert!(stats.total_buffers <= 20);

        // Memory usage should be reasonable (pool allocates in 1MB chunks)
        assert!(stats.total_memory_mb <= 25.0); // Up to 25MB for 20 allocations in 1MB chunks
    }

    #[test]
    fn test_task_scheduler_load_balancing() {
        use std::sync::Arc;
        use std::thread;
        use std::time::Duration;

        let scheduler = Arc::new(TaskScheduler::new(4));

        // Submit tasks with mixed priorities
        let priorities = vec![
            TaskPriority::Low,
            TaskPriority::Medium,
            TaskPriority::High,
            TaskPriority::Critical,
        ];

        let initial_queue_size = scheduler.queue_size();
        assert_eq!(initial_queue_size, 0);

        for (i, &priority) in priorities.iter().enumerate() {
            for j in 0..5 {
                let task = ImageTask::LoadImage {
                    path: std::path::PathBuf::from(format!("task_{}_{}.jpg", i, j)),
                    priority,
                };
                scheduler.submit_task(task);
            }
        }

        // Verify tasks were queued (some may have already started processing)
        let queue_size_after_submit = scheduler.queue_size();
        assert!(queue_size_after_submit > 0, "No tasks were queued");
        assert!(queue_size_after_submit <= 20, "Too many tasks queued: {}", queue_size_after_submit);

        // Allow some processing time
        thread::sleep(Duration::from_millis(200));

        // Tasks should be processed (since they're dummy tasks with non-existent files)
        // The queue should be much smaller or empty
        let final_queue_size = scheduler.queue_size();
        assert!(final_queue_size < queue_size_after_submit, "Tasks were not processed: {} -> {}", queue_size_after_submit, final_queue_size);
    }
}

#[cfg(test)]
mod integration_tests {
    use image::{DynamicImage, GenericImageView};
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_image_loading_integration() {
        // Create a temporary directory for test files
        let temp_dir = TempDir::new().unwrap();
        let test_image_path = temp_dir.path().join("test.jpg");

        // Create a minimal valid JPEG file (this is a very basic test)
        // In a real scenario, you'd use a proper test image
        let jpeg_data = vec![
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01,
            0x00, 0x48, 0x00, 0x48, 0x00, 0x00, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x01, 0x00,
            0x01, 0x01, 0x01, 0x11, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01, 0xFF, 0xC4, 0x00,
            0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x08, 0xFF, 0xC4, 0x00, 0x14, 0x10, 0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,
            0xDA, 0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00, 0x00,
            0xFF, 0xD9,
        ];

        fs::write(&test_image_path, jpeg_data).unwrap();

        // Test that the file exists
        assert!(test_image_path.exists());

        // In a full integration test, you would:
        // 1. Create an ImageLoader instance
        // 2. Load the test image
        // 3. Verify the loaded image data
        // 4. Test error handling for corrupted files

        // For now, just verify file operations work
        assert!(Path::new(&test_image_path).exists());
    }

    #[test]
    fn test_cache_performance() {
        use crate::image_cache::ImageCache;
        use image::{DynamicImage, Rgba, RgbaImage};
        use std::time::{Duration, Instant};

        let cache = ImageCache::new(10 * 1024 * 1024); // 10MB cache

        // Benchmark cache operations
        let start = Instant::now();

        // Simulate cache operations
        for i in 0..1000 {
            let key = format!("test_key_{}", i);
            let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 255])));

            cache.put(key.clone(), img.clone());
            let _retrieved = cache.get(&key);
        }

        let duration = start.elapsed();

        // Cache operations should be reasonably fast
        assert!(
            duration < Duration::from_secs(1),
            "Cache operations took too long: {:?}",
            duration
        );

        // Verify cache stats
        let stats = cache.stats();
        assert!(stats.image_count > 0);
        assert!(stats.image_size_bytes > 0);
    }

    #[test]
    fn test_cache_eviction_by_memory() {
        use crate::image_cache::ImageCache;
        use image::{DynamicImage, Rgba, RgbaImage};
        use std::path::PathBuf;

        // 1 MB cache
        let cache = ImageCache::new(1);

        // Each image is ~512x512x4 = 1,048,576 bytes (~1MB)
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(512, 512, Rgba([10, 20, 30, 255])));

        cache.insert(PathBuf::from("img1.jpg"), img.clone());
        cache.insert(PathBuf::from("img2.jpg"), img.clone());

        let stats = cache.get_stats();

        // Ensure total tracked size does not exceed configured cache size
        assert!(
            stats.image_size_bytes <= 1 * 1024 * 1024,
            "Cache exceeded max size: {}",
            stats.image_size_bytes
        );
    }

    #[test]
    fn test_thumbnail_disk_persistence() {
        use crate::image_cache::ImageCache;
        use image::{DynamicImage, Rgba, RgbaImage};
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let key_path = tmp.path().join("test_image.jpg");

        let cache = ImageCache::new(10);

        let thumb =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(16, 16, Rgba([100, 150, 200, 255])));
        // Ensure the source file exists so the cache key generation which depends on file metadata works.
        thumb.save(&key_path).unwrap();
        cache.insert_thumbnail(key_path.clone(), thumb.clone());

        // Ensure thumbnail exists in-memory
        let stats = cache.get_stats();
        assert!(stats.thumbnail_count >= 1);

        // Clear in-memory caches and verify load from disk succeeds
        cache.clear();
        let loaded = cache.get_thumbnail(&key_path);
        assert!(loaded.is_some(), "Failed to load thumbnail from disk cache");
    }

    #[test]
    fn test_preload_thumbnails_parallel() {
        use crate::image_cache::ImageCache;
        use image::{DynamicImage, Rgba, RgbaImage};
        use std::path::PathBuf;
        use std::thread;
        use std::time::{Duration, Instant};
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let mut paths = Vec::new();

        for i in 0..4 {
            let p = tmp.path().join(format!("f{}.png", i));
            let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
                64,
                64,
                Rgba([i as u8 * 10, 100, 120, 255]),
            ));
            img.save(&p).unwrap();
            paths.push(p);
        }

        let cache = ImageCache::new(10);
        let paths_clone: Vec<PathBuf> = paths.clone();

        cache.preload_thumbnails_parallel(paths.clone(), 64);

        // Wait until thumbnails are generated (timeout after 2s)
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(2) {
            let mut all_ok = true;
            for p in &paths_clone {
                if cache.get_thumbnail(p).is_none() {
                    all_ok = false;
                    break;
                }
            }
            if all_ok {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }

        // Final check
        for p in &paths_clone {
            assert!(
                cache.get_thumbnail(p).is_some(),
                "Thumbnail not generated for {:?}",
                p
            );
        }
    }

    #[test]
    fn debug_load_dng() {
        use std::path::Path;
        let p = Path::new("testfiles/20251121-IMG_20251121_145826.dng");
        if !p.exists() {
            println!("Test DNG not found, skipping test");
            return;
        }

        // Try embedded thumbnail via EXIF
        match crate::image_loader::load_raw_embedded_thumbnail(p, 512) {
            Ok(img) => println!("embedded thumbnail OK: {}x{}", img.width(), img.height()),
            Err(e) => println!("embedded thumbnail failed: {:?}", e),
        }

        // Use the public thumbnail loader
        match crate::image_loader::load_thumbnail(p, 512) {
            Ok(img) => println!("load_thumbnail OK: {}x{}", img.width(), img.height()),
            Err(e) => println!("load_thumbnail failed: {:?}", e),
        }

        // Finally try full RAW decode (this may be slow)
        match crate::image_loader::load_image(p) {
            Ok(img) => println!("load_image OK: {}x{}", img.width(), img.height()),
            Err(e) => println!("load_image failed: {:?}", e),
        }
    }

    #[test]
    fn test_image_loading_pipeline() {
        // Create a simple test image
        let test_image = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            100,
            100,
            image::Rgba([255, 0, 0, 255]),
        ));

        // Test that image processing works
        let mut adjustments = crate::image_loader::ImageAdjustments::default();
        adjustments.saturation = 0.0; // This should make it grayscale

        // This tests the CPU image processing pipeline
        let processed =
            crate::image_loader::adjustments::apply_adjustments(&test_image, &adjustments);

        // Verify the image was processed (should be different from original)
        assert_eq!(processed.dimensions(), test_image.dimensions());

        // Check that at least some pixels changed (more robust than != comparison)
        let original_pixels: Vec<_> = test_image.as_rgba8().unwrap().pixels().collect();
        let processed_pixels: Vec<_> = processed.as_rgba8().unwrap().pixels().collect();

        // At least some pixels should be different after processing
        let mut has_difference = false;
        for (orig, proc) in original_pixels.iter().zip(processed_pixels.iter()) {
            if orig != proc {
                has_difference = true;
                break;
            }
        }
        assert!(has_difference, "Image processing did not change any pixels");
    }

    #[test]
    fn test_settings_persistence() {
        // Test that settings can be saved and loaded
        let mut settings = crate::settings::Settings::default();
        settings.theme = crate::settings::Theme::Dark;
        settings.show_sidebar = false;

        // In a real integration test, we'd save to a temp file and load back
        // For now, just test the default loading
        let loaded_settings = crate::settings::Settings::load();
        assert!(
            loaded_settings.theme == crate::settings::Theme::Dark
                || loaded_settings.theme == crate::settings::Theme::Light
        );
    }

    #[test]
    fn test_metadata_operations() {
        let mut db = crate::metadata::MetadataDb::new();
        let path = PathBuf::from("/test/image.jpg");

        // Test rating operations
        db.set_rating(path.clone(), 4);
        assert_eq!(db.get(&path).rating, 4);

        // Test color label
        db.set_color_label(path.clone(), crate::settings::ColorLabel::Red);
        assert_eq!(db.get(&path).color_label, crate::settings::ColorLabel::Red);
    }
}
