#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::metadata::{MetadataDb, UndoHistory, FileOperation};
    use crate::settings::ColorLabel;

    extern crate test;

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

        db.toggle_flag(path);
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

        let mut stats = CacheStats::default();

        // Simulate some cache operations
        stats.cache_hit_count = 100;
        stats.cache_miss_count = 25;

        assert_eq!(stats.hit_rate(), 0.8); // 100/125 = 0.8

        stats.cache_memory_usage = 1024 * 1024; // 1 MB
        stats.thumbnail_memory_usage = 512 * 1024; // 0.5 MB

        assert_eq!(stats.memory_usage_mb(), 1.5);
    }

    #[test]
    fn test_rename_pattern() {
        use crate::metadata::RenamePattern;

        let mut pattern = RenamePattern::default();
        pattern.prefix = "IMG_".to_string();
        pattern.counter_start = 1;
        pattern.counter_digits = 3;
        pattern.use_original_name = false;

        let result = pattern.apply("test.jpg", 0);
        assert_eq!(result, "IMG_001.jpg");

        pattern.use_original_name = true;
        pattern.find = "test".to_string();
        pattern.replace = "photo".to_string();

        let result = pattern.apply("test_image.jpg", 5);
        assert_eq!(result, "IMG_photo_image006.jpg");
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
    use super::*;
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

        let mut cache = ImageCache::new(10 * 1024 * 1024); // 10MB cache

        // Benchmark cache operations
        let start = Instant::now();

        // Simulate cache operations
        for i in 0..1000 {
            let key = format!("test_key_{}", i);
            let data = vec![0u8; 1024]; // 1KB of dummy data

            cache.put(key.clone(), data.clone());
            let _retrieved = cache.get(&key);
        }

        let duration = start.elapsed();

        // Cache operations should be reasonably fast
        assert!(duration < Duration::from_secs(1), "Cache operations took too long: {:?}", duration);

        // Verify cache stats
        let stats = cache.stats();
        assert!(stats.total_entries > 0);
        assert!(stats.hit_rate() >= 0.0 && stats.hit_rate() <= 1.0);
    }
}

#[cfg(test)]
mod benchmark_tests {
    use super::*;
    use std::time::{Duration, Instant};
    use test::Bencher;

    #[bench]
    fn bench_cache_operations(b: &mut Bencher) {
        use crate::image_cache::ImageCache;

        let mut cache = ImageCache::new(50 * 1024 * 1024); // 50MB cache

        b.iter(|| {
            let key = "bench_key";
            let data = vec![0u8; 10 * 1024]; // 10KB data

            cache.put(key.to_string(), data.clone());
            let _ = cache.get(key);
        });
    }

    #[bench]
    fn bench_error_creation(b: &mut Bencher) {
        use crate::errors::ViewerError;

        b.iter(|| {
            let error = ViewerError::ImageLoadError {
                path: std::path::PathBuf::from("/test/path.jpg"),
                message: "Test error message".to_string(),
            };

            let _code = error.error_code();
            let _msg = error.user_message();
        });
    }
}