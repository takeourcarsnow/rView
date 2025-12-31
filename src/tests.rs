#[cfg(test)]
mod unit_tests {
    use std::path::PathBuf;
    use crate::metadata::{MetadataDb, UndoHistory, FileOperation};
    use crate::settings::ColorLabel;


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

        let stats = CacheStats { cache_hit_count: 100, cache_miss_count: 25, cache_memory_usage: 1024 * 1024, thumbnail_memory_usage: 512 * 1024, ..Default::default() };

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
        if let Ok(proc) = crate::gpu::GpuProcessor::new() {
            let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(16, 16, image::Rgba([128, 128, 128, 255])));
            let adj = crate::image_loader::ImageAdjustments {
                exposure: 0.5,
                contrast: 1.1,
                brightness: 10.0,
                saturation: 1.0,
                highlights: 0.0,
                shadows: 0.0,
                temperature: 0.0,
                tint: 0.0,
                blacks: 0.0,
                whites: 0.0,
                sharpening: 0.0,
            };

            let out = proc.apply_adjustments(&img, &adj).expect("GPU adjustment failed");
            assert_eq!(out.len(), (16 * 16 * 4) as usize);
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
    fn test_logging_initialization() {
        // Ensure logging/tracing initialization runs without panic
        crate::logging::init_tracing(false);
    }
}

#[cfg(test)]
mod integration_tests {
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_image_loading_integration() {
        // Create a temporary directory for test files
        let temp_dir = TempDir::new().unwrap();
        let test_image_path = temp_dir.path().join("test.jpg");

        // Create a minimal valid JPEG file (this is a very basic test)
        // In a real scenario, you'd use a proper test image
        let jpeg_data = vec![
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
            0x01, 0x01, 0x00, 0x48, 0x00, 0x48, 0x00, 0x00, 0xFF, 0xC0, 0x00, 0x11,
            0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0x02, 0x11, 0x01,
            0x03, 0x11, 0x01, 0xFF, 0xC4, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x08, 0xFF, 0xC4, 0x00, 0x14, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,
            0xDA, 0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F,
            0x00, 0x00, 0xFF, 0xD9
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
        use std::time::{Duration, Instant};
        use image::{DynamicImage, RgbaImage, Rgba};

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
        assert!(duration < Duration::from_secs(1), "Cache operations took too long: {:?}", duration);

        // Verify cache stats
        let stats = cache.stats();
        assert!(stats.image_count > 0);
        assert!(stats.image_size_bytes > 0);
    }

    #[test]
    fn test_cache_eviction_by_memory() {
        use crate::image_cache::ImageCache;
        use std::path::PathBuf;
        use image::{DynamicImage, RgbaImage, Rgba};

        // 1 MB cache
        let cache = ImageCache::new(1);

        // Each image is ~512x512x4 = 1,048,576 bytes (~1MB)
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(512, 512, Rgba([10, 20, 30, 255])));

        cache.insert(PathBuf::from("img1.jpg"), img.clone());
        cache.insert(PathBuf::from("img2.jpg"), img.clone());

        let stats = cache.get_stats();

        // Ensure total tracked size does not exceed configured cache size
        assert!(stats.image_size_bytes <= 1 * 1024 * 1024, "Cache exceeded max size: {}", stats.image_size_bytes);
    }

    #[test]
    fn test_thumbnail_disk_persistence() {
        use crate::image_cache::ImageCache;
        use tempfile::TempDir;
        use image::{DynamicImage, RgbaImage, Rgba};

        let tmp = TempDir::new().unwrap();
        let key_path = tmp.path().join("test_image.jpg");

        let cache = ImageCache::new(10);

        let thumb = DynamicImage::ImageRgba8(RgbaImage::from_pixel(16, 16, Rgba([100, 150, 200, 255])));
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
        use tempfile::TempDir;
        use std::path::PathBuf;
        use std::thread;
        use std::time::{Duration, Instant};
        use image::{DynamicImage, RgbaImage, Rgba};

        let tmp = TempDir::new().unwrap();
        let mut paths = Vec::new();

        for i in 0..4 {
            let p = tmp.path().join(format!("f{}.png", i));
            let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(64, 64, Rgba([i as u8 * 10, 100, 120, 255])));
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
            if all_ok { return; }
            thread::sleep(Duration::from_millis(50));
        }

        // Final check
        for p in &paths_clone {
            assert!(cache.get_thumbnail(p).is_some(), "Thumbnail not generated for {:?}", p);
        }
    }

    #[test]
    fn debug_load_lightroom_dng() {
        use std::path::Path;
        let p = Path::new("testfiles/20251121-IMG_20251121_145826.dng");
        assert!(p.exists(), "Test DNG not found: {:?}", p);

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
}

#[cfg(test)]
mod benchmark_tests {
    // benching disabled: test crate/bench unsupported in stable

    #[ignore]
    #[allow(dead_code)]
    fn bench_cache_operations() {
        // bench disabled on stable; kept for reference
    }

    #[ignore]
    #[allow(dead_code)]
    fn bench_error_creation() {
        // bench disabled on stable; kept for reference
    }
}