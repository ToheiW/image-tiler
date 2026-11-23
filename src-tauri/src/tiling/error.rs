use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TileError {
    #[error("I/O failure: {0}")]
    Io(#[from] std::io::Error),

    #[error("image codec failure: {0}")]
    Image(#[from] image::ImageError),

    #[error("tile width and height must be at least 1 pixel")]
    InvalidTileSize,

    #[error("DPI must be between 1 and 2400")]
    InvalidDpi,

    #[error("overlap must be between 0 and 100 millimetres")]
    InvalidOverlap,

    #[error("each margin must be between 0 and 100 millimetres")]
    InvalidMargin,

    #[error("columns and rows must each be at least 1")]
    InvalidGrid,

    #[error("the printer-safe margin and user margins leave no printable area")]
    PageTooSmall,

    #[error("image is empty")]
    EmptyImage,

    #[error("image is too large ({width} x {height} px); the limit is {max} pixels")]
    ImageTooLarge { width: u32, height: u32, max: u64 },

    #[error("too many pages ({pages}); the limit is {max}")]
    TooManyPages { pages: u32, max: u32 },

    #[error("page is too large for a PDF ({width:.1} x {height:.1} mm); the limit is 5080 mm")]
    PageTooLarge { width: f32, height: f32 },

    #[error("output path must be absolute: {}", .0.display())]
    OutputPathNotAbsolute(PathBuf),

    #[error("PDF generation failed: {0}")]
    Pdf(String),

    #[error("output file already exists: {}", .0.display())]
    OutputExists(PathBuf),

    #[error("cannot write output {}: {source}", path.display())]
    OutputWrite {
        path: PathBuf,
        source: std::io::Error,
    },
}
