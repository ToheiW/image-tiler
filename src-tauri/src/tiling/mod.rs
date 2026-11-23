pub mod error;
pub mod grid;
pub mod layout;
pub mod orientation;
pub mod pdf;

pub use error::TileError;
pub use grid::{GridPlan, TileRect, plan_grid};
pub use layout::{
    LayoutPlan, LayoutSpec, MAX_PAGES, MAX_PIXELS, Margins, PRINTER_SAFE_MARGIN_MM,
    PageOrientation, PagePlan, PageSize, PdfLayout, RectMm, RectPx, SizeMode,
    oriented_image_dimensions, plan_layout,
};
pub use orientation::{Orientation, read_orientation};
pub use pdf::{PdfReport, PdfRequest, tile_to_pdf};
