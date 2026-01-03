use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ViewerError {
    #[error("Failed to load image '{path}': {message}")]
    ImageLoadError { path: PathBuf, message: String },

    #[error("Unsupported image format: {format}")]
    UnsupportedFormat { format: String },

    #[error("RAW processing error for '{path}': {message}")]
    RawProcessingError { path: PathBuf, message: String },

    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Disk full or out of space")]
    DiskFull,

    #[error("Network error: {message}")]
    NetworkError { message: String },

    #[error("Image decoding error for '{path}': {message}")]
    DecodingError { path: PathBuf, message: String },

    #[error("Export error for '{path}': {message}")]
    ExportError { path: PathBuf, message: String },

    #[error("Cache error: {message}")]
    CacheError { message: String },

    #[error("Settings error: {message}")]
    SettingsError { message: String },

    #[error("Metadata error: {message}")]
    MetadataError { message: String },

    #[error("Corrupted image file '{path}': {details}")]
    CorruptedImage { path: PathBuf, details: String },

    #[error("GPU processing error: {message}")]
    GpuError { message: String },

    #[error("Thread pool error: {message}")]
    ThreadPoolError { message: String },

    #[error("IO error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    #[error("JSON parsing error: {source}")]
    JsonError {
        #[from]
        source: serde_json::Error,
    },

    #[error("Image processing error: {message}")]
    ImageProcessingError { message: String },

    #[error("Operation cancelled")]
    Cancelled,

    #[error("Timeout error: {operation}")]
    Timeout { operation: String },

    #[error("Invalid operation: {message}")]
    InvalidOperation { message: String },
}

pub type Result<T> = std::result::Result<T, ViewerError>;

#[allow(dead_code)]
impl ViewerError {
    /// Returns true if this error is recoverable (user can retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            ViewerError::FileNotFound { .. }
                | ViewerError::PermissionDenied { .. }
                | ViewerError::NetworkError { .. }
                | ViewerError::Timeout { .. }
                | ViewerError::IoError { .. }
                | ViewerError::GpuError { .. }
                | ViewerError::ThreadPoolError { .. }
        )
    }

    /// Returns a user-friendly error message with recovery suggestions
    pub fn user_message(&self) -> String {
        let base_message = self.to_string();
        let suggestion = match self {
            ViewerError::FileNotFound { .. } => "Check if the file exists and you have permission to access it.",
            ViewerError::PermissionDenied { .. } => "Check file permissions or run the application with elevated privileges.",
            ViewerError::DiskFull => "Free up disk space and try again.",
            ViewerError::NetworkError { .. } => "Check your internet connection and try again.",
            ViewerError::UnsupportedFormat { .. } => "This image format is not supported. Try converting it to a common format like JPEG or PNG.",
            ViewerError::ImageLoadError { .. } | ViewerError::DecodingError { .. } => "The image file may be corrupted. Try opening it in another viewer.",
            ViewerError::CorruptedImage { .. } => "The image file appears to be corrupted. Try repairing it with image recovery software or re-downloading if applicable.",
            ViewerError::RawProcessingError { .. } => "RAW file processing failed. The file may be corrupted or use an unsupported format.",
            ViewerError::ExportError { .. } => "Export failed. Check if you have write permissions in the target directory.",
            ViewerError::Timeout { .. } => "Operation took too long. Try again or check system resources.",
            ViewerError::IoError { .. } => "File system error occurred. Check disk space and permissions.",
            ViewerError::GpuError { .. } => "GPU processing failed. The operation will fall back to CPU processing, which may be slower.",
            ViewerError::ThreadPoolError { .. } => "Background processing failed. Try restarting the application.",
            _ => "An unexpected error occurred.",
        };

        format!("{}\n\n{}", base_message, suggestion)
    }

    /// Returns an error code for programmatic handling
    pub fn error_code(&self) -> &'static str {
        match self {
            ViewerError::ImageLoadError { .. } => "IMAGE_LOAD_ERROR",
            ViewerError::UnsupportedFormat { .. } => "UNSUPPORTED_FORMAT",
            ViewerError::RawProcessingError { .. } => "RAW_PROCESSING_ERROR",
            ViewerError::FileNotFound { .. } => "FILE_NOT_FOUND",
            ViewerError::PermissionDenied { .. } => "PERMISSION_DENIED",
            ViewerError::DiskFull => "DISK_FULL",
            ViewerError::NetworkError { .. } => "NETWORK_ERROR",
            ViewerError::DecodingError { .. } => "DECODING_ERROR",
            ViewerError::ExportError { .. } => "EXPORT_ERROR",
            ViewerError::CacheError { .. } => "CACHE_ERROR",
            ViewerError::SettingsError { .. } => "SETTINGS_ERROR",
            ViewerError::MetadataError { .. } => "METADATA_ERROR",
            ViewerError::CorruptedImage { .. } => "CORRUPTED_IMAGE",
            ViewerError::GpuError { .. } => "GPU_ERROR",
            ViewerError::ThreadPoolError { .. } => "THREAD_POOL_ERROR",
            ViewerError::IoError { .. } => "IO_ERROR",
            ViewerError::JsonError { .. } => "JSON_ERROR",
            ViewerError::ImageProcessingError { .. } => "IMAGE_PROCESSING_ERROR",
            ViewerError::Cancelled => "CANCELLED",
            ViewerError::Timeout { .. } => "TIMEOUT",
            ViewerError::InvalidOperation { .. } => "INVALID_OPERATION",
        }
    }

    /// Logs the error and optionally reports it for crash analysis
    pub fn log_and_report(&self) {
        let error_code = self.error_code();
        let message = self.to_string();

        // Log to stderr for debugging
        eprintln!("Error [{}]: {}", error_code, message);

        // For critical errors, we could send crash reports here
        // This is a placeholder for future crash reporting implementation
        if matches!(
            self,
            ViewerError::CorruptedImage { .. }
                | ViewerError::GpuError { .. }
                | ViewerError::ThreadPoolError { .. }
        ) {
            // In a real implementation, this could send anonymized crash reports
            eprintln!(
                "Critical error detected. Consider checking system resources or updating drivers."
            );
        }
    }
}
