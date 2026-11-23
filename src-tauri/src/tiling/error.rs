use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TileError {
    #[error("I/O failure")]
    Io(#[from] std::io::Error),

    #[error("image codec failure")]
    Image(#[from] image::ImageError),

    #[error("tile width and height must be at least 1 pixel")]
    InvalidTileSize,

    #[error("DPI must be at least 1")]
    InvalidDpi,

    #[error("overlap must be a non-negative number of millimetres")]
    InvalidOverlap,

    #[error("safe margin must be a non-negative number of millimetres")]
    InvalidMargin,

    #[error("page size must be two positive numbers in millimetres")]
    InvalidPageSize,

    #[error("page is too small for the safe margin and overlap")]
    PageTooSmall,

    #[error("PDF generation failed: {0}")]
    Pdf(String),

    #[error("output file already exists: {}", .0.display())]
    OutputExists(PathBuf),

    #[error("input path has no usable file name: {}", .0.display())]
    InvalidInputName(PathBuf),
}
