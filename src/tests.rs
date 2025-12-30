#[cfg(test)]
mod tests {
    use super::*;
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
}