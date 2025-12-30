use thiserror::Error;

#[derive(Error, Debug)]
pub enum ViewerError {
    #[error("Failed to load image: {0}")]
    ImageLoadError(String),
    
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    
    #[error("RAW processing error: {0}")]
    RawProcessingError(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Image decoding error: {0}")]
    DecodingError(String),
    
    #[error("Export error: {0}")]
    ExportError(String),
}

pub type Result<T> = std::result::Result<T, ViewerError>;
