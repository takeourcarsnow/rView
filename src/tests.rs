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
}

#[cfg(test)]
mod benchmark_tests {
    // benching disabled: test crate/bench unsupported in stable

    #[ignore]
    fn bench_cache_operations() {
        // bench disabled on stable; kept for reference
    }

    #[ignore]
    fn bench_error_creation() {
        // bench disabled on stable; kept for reference
    }
}