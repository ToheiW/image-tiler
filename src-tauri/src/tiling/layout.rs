use std::path::Path;

use serde::{Deserialize, Serialize};

use super::error::TileError;
use super::orientation::read_orientation;

/// Reserved non-printable band on every page edge. Content never enters it.
pub const PRINTER_SAFE_MARGIN_MM: f32 = 5.0;
/// Hard ceiling on generated pages, to keep a typo from hanging the app.
pub const MAX_PAGES: u32 = 400;
/// Hard ceiling on decoded pixels, to keep a decompression bomb from aborting.
pub const MAX_PIXELS: u64 = 100_000_000;

const MIN_CORE_MM: f32 = 10.0;
const MAX_MARGIN_MM: f32 = 100.0;
const MAX_OVERLAP_MM: f32 = 100.0;
const MAX_TILE_PX: u32 = 100_000;
const MAX_DPI: u32 = 2_400;
/// PDF viewers reject pages larger than 14 400 pt (5 080 mm).
const MAX_PAGE_MM: f32 = 5_080.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PdfLayout {
    #[default]
    OneTilePerPage,
    SinglePage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SizeMode {
    #[default]
    Pages,
    Tile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PageSize {
    #[default]
    A4,
    A3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PageOrientation {
    #[default]
    Portrait,
    Landscape,
}

impl PageSize {
    pub fn dimensions(self, orientation: PageOrientation) -> (f32, f32) {
        let (width, height) = match self {
            PageSize::A4 => (210.0, 297.0),
            PageSize::A3 => (297.0, 420.0),
        };
        match orientation {
            PageOrientation::Portrait => (width, height),
            PageOrientation::Landscape => (height, width),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Margins {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            top: 10.0,
            right: 10.0,
            bottom: 10.0,
            left: 10.0,
        }
    }
}

impl Margins {
    fn sides(self) -> [f32; 4] {
        [self.top, self.right, self.bottom, self.left]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSpec {
    #[serde(default)]
    pub mode: SizeMode,
    #[serde(default)]
    pub layout: PdfLayout,
    #[serde(default)]
    pub page_size: PageSize,
    #[serde(default)]
    pub orientation: PageOrientation,
    #[serde(default = "default_columns")]
    pub columns: u32,
    #[serde(default = "default_rows")]
    pub rows: u32,
    #[serde(default = "default_dpi")]
    pub dpi: u32,
    #[serde(default = "default_tile_size")]
    pub tile_width: u32,
    #[serde(default = "default_tile_size")]
    pub tile_height: u32,
    #[serde(default)]
    pub margins_mm: Margins,
    #[serde(default)]
    pub overlap_mm: f32,
}

fn default_columns() -> u32 {
    2
}

fn default_rows() -> u32 {
    2
}

fn default_dpi() -> u32 {
    300
}

fn default_tile_size() -> u32 {
    1181
}

impl Default for LayoutSpec {
    fn default() -> Self {
        Self {
            mode: SizeMode::Pages,
            layout: PdfLayout::OneTilePerPage,
            page_size: PageSize::A4,
            orientation: PageOrientation::Portrait,
            columns: default_columns(),
            rows: default_rows(),
            dpi: default_dpi(),
            tile_width: default_tile_size(),
            tile_height: default_tile_size(),
            margins_mm: Margins::default(),
            overlap_mm: 10.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RectMm {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RectPx {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// One printed page. All millimetre coordinates are measured from the
/// page's top-left corner, matching how the canvas preview draws them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagePlan {
    pub row: u32,
    pub col: u32,
    pub page_width_mm: f32,
    pub page_height_mm: f32,
    /// The printable core cell on this page. Crop marks sit on these edges.
    pub core: RectMm,
    /// Where the whole image would sit on this page (may extend off-page).
    pub image: RectMm,
    /// The source pixels this page prints (clamped to the image).
    pub source: RectPx,
    /// Where `source` is drawn on the page.
    pub dest: RectMm,
}

/// The single layout calculation shared by the PDF writer and the preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPlan {
    pub image_width: u32,
    pub image_height: u32,
    pub page_width_mm: f32,
    pub page_height_mm: f32,
    pub columns: u32,
    pub rows: u32,
    pub scale_mm_per_px: f32,
    pub effective_dpi: f32,
    pub safe_margin_mm: f32,
    pub margins: Margins,
    pub overlap_mm: f32,
    pub pages: Vec<PagePlan>,
}

/// Reads the file header and applies EXIF orientation, so the preview and the
/// PDF writer agree on the image's upright dimensions without a full decode.
pub fn oriented_image_dimensions(path: &Path) -> Result<(u32, u32), TileError> {
    let reader = image::ImageReader::open(path)?.with_guessed_format()?;
    let (width, height) = reader.into_dimensions()?;
    let orientation = read_orientation(path);
    Ok(if orientation.swaps_dimensions() {
        (height, width)
    } else {
        (width, height)
    })
}

pub fn plan_layout(
    spec: &LayoutSpec,
    image_width: u32,
    image_height: u32,
) -> Result<LayoutPlan, TileError> {
    if image_width == 0 || image_height == 0 {
        return Err(TileError::EmptyImage);
    }
    validate_margins(spec)?;

    let (page_width_mm, page_height_mm, core_width_mm, core_height_mm, columns, rows, scale) =
        match spec.mode {
            SizeMode::Pages => {
                let (page_width, page_height) = spec.page_size.dimensions(spec.orientation);
                let core_width = page_width
                    - 2.0 * PRINTER_SAFE_MARGIN_MM
                    - spec.margins_mm.left
                    - spec.margins_mm.right;
                let core_height = page_height
                    - 2.0 * PRINTER_SAFE_MARGIN_MM
                    - spec.margins_mm.top
                    - spec.margins_mm.bottom;
                if core_width < MIN_CORE_MM || core_height < MIN_CORE_MM {
                    return Err(TileError::PageTooSmall);
                }

                let (columns, rows) = if spec.layout == PdfLayout::SinglePage {
                    (1, 1)
                } else {
                    if spec.columns == 0 || spec.rows == 0 {
                        return Err(TileError::InvalidGrid);
                    }
                    (spec.columns, spec.rows)
                };
                check_page_count(columns, rows)?;

                let scale = (core_width * columns as f32 / image_width as f32)
                    .min(core_height * rows as f32 / image_height as f32);
                if !scale.is_finite() || scale <= 0.0 {
                    return Err(TileError::PageTooSmall);
                }

                (
                    page_width,
                    page_height,
                    core_width,
                    core_height,
                    columns,
                    rows,
                    scale,
                )
            }
            SizeMode::Tile => {
                if spec.dpi == 0 || spec.dpi > MAX_DPI {
                    return Err(TileError::InvalidDpi);
                }
                if spec.layout == PdfLayout::SinglePage {
                    let scale = 25.4 / spec.dpi as f32;
                    let core_width = image_width as f32 * scale;
                    let core_height = image_height as f32 * scale;
                    let page_width = core_width
                        + 2.0 * PRINTER_SAFE_MARGIN_MM
                        + spec.margins_mm.left
                        + spec.margins_mm.right;
                    let page_height = core_height
                        + 2.0 * PRINTER_SAFE_MARGIN_MM
                        + spec.margins_mm.top
                        + spec.margins_mm.bottom;
                    check_page_size(page_width, page_height)?;
                    (
                        page_width,
                        page_height,
                        core_width,
                        core_height,
                        1,
                        1,
                        scale,
                    )
                } else {
                    if spec.tile_width == 0
                        || spec.tile_height == 0
                        || spec.tile_width > MAX_TILE_PX
                        || spec.tile_height > MAX_TILE_PX
                    {
                        return Err(TileError::InvalidTileSize);
                    }
                    let scale = 25.4 / spec.dpi as f32;
                    let columns = image_width.div_ceil(spec.tile_width);
                    let rows = image_height.div_ceil(spec.tile_height);
                    check_page_count(columns, rows)?;

                    let core_width = spec.tile_width as f32 * scale;
                    let core_height = spec.tile_height as f32 * scale;
                    let page_width = core_width
                        + 2.0 * PRINTER_SAFE_MARGIN_MM
                        + spec.margins_mm.left
                        + spec.margins_mm.right;
                    let page_height = core_height
                        + 2.0 * PRINTER_SAFE_MARGIN_MM
                        + spec.margins_mm.top
                        + spec.margins_mm.bottom;
                    check_page_size(page_width, page_height)?;
                    (
                        page_width,
                        page_height,
                        core_width,
                        core_height,
                        columns,
                        rows,
                        scale,
                    )
                }
            }
        };

    let grid_width_mm = core_width_mm * columns as f32;
    let grid_height_mm = core_height_mm * rows as f32;
    let image_width_mm = image_width as f32 * scale;
    let image_height_mm = image_height as f32 * scale;
    let offset_x_mm = (grid_width_mm - image_width_mm) / 2.0;
    let offset_y_mm = (grid_height_mm - image_height_mm) / 2.0;

    let core_x_mm = PRINTER_SAFE_MARGIN_MM + spec.margins_mm.left;
    let core_y_mm = PRINTER_SAFE_MARGIN_MM + spec.margins_mm.top;
    // Overlap exists to match adjacent tiles; a single sheet has no neighbours.
    let multi_page = columns * rows > 1;
    let left_pad = if multi_page {
        spec.overlap_mm.min(spec.margins_mm.left)
    } else {
        0.0
    };
    let right_pad = if multi_page {
        spec.overlap_mm.min(spec.margins_mm.right)
    } else {
        0.0
    };
    let top_pad = if multi_page {
        spec.overlap_mm.min(spec.margins_mm.top)
    } else {
        0.0
    };
    let bottom_pad = if multi_page {
        spec.overlap_mm.min(spec.margins_mm.bottom)
    } else {
        0.0
    };

    let mut pages = Vec::with_capacity(columns as usize * rows as usize);
    for row in 0..rows {
        for col in 0..columns {
            let image_rect = RectMm {
                x: core_x_mm + offset_x_mm - col as f32 * core_width_mm,
                y: core_y_mm + offset_y_mm - row as f32 * core_height_mm,
                width: image_width_mm,
                height: image_height_mm,
            };

            let grid_x0 = col as f32 * core_width_mm - left_pad;
            let grid_x1 = col as f32 * core_width_mm + core_width_mm + right_pad;
            let grid_y0 = row as f32 * core_height_mm - top_pad;
            let grid_y1 = row as f32 * core_height_mm + core_height_mm + bottom_pad;

            let (x0, x1) = pixel_span(grid_x0, grid_x1, offset_x_mm, scale, image_width);
            let (y0, y1) = pixel_span(grid_y0, grid_y1, offset_y_mm, scale, image_height);

            let source = RectPx {
                x: x0,
                y: y0,
                width: x1 - x0,
                height: y1 - y0,
            };
            let dest = RectMm {
                x: image_rect.x + x0 as f32 * scale,
                y: image_rect.y + y0 as f32 * scale,
                width: source.width as f32 * scale,
                height: source.height as f32 * scale,
            };

            pages.push(PagePlan {
                row,
                col,
                page_width_mm,
                page_height_mm,
                core: RectMm {
                    x: core_x_mm,
                    y: core_y_mm,
                    width: core_width_mm,
                    height: core_height_mm,
                },
                image: image_rect,
                source,
                dest,
            });
        }
    }

    Ok(LayoutPlan {
        image_width,
        image_height,
        page_width_mm,
        page_height_mm,
        columns,
        rows,
        scale_mm_per_px: scale,
        effective_dpi: 25.4 / scale,
        safe_margin_mm: PRINTER_SAFE_MARGIN_MM,
        margins: spec.margins_mm,
        overlap_mm: spec.overlap_mm,
        pages,
    })
}

fn validate_margins(spec: &LayoutSpec) -> Result<(), TileError> {
    for margin in spec.margins_mm.sides() {
        if !margin.is_finite() || !(0.0..=MAX_MARGIN_MM).contains(&margin) {
            return Err(TileError::InvalidMargin);
        }
    }
    let overlap = spec.overlap_mm;
    if !overlap.is_finite() || !(0.0..=MAX_OVERLAP_MM).contains(&overlap) {
        return Err(TileError::InvalidOverlap);
    }
    Ok(())
}

fn check_page_count(columns: u32, rows: u32) -> Result<(), TileError> {
    let pages = columns.checked_mul(rows).ok_or(TileError::TooManyPages {
        pages: u32::MAX,
        max: MAX_PAGES,
    })?;
    if pages > MAX_PAGES {
        return Err(TileError::TooManyPages {
            pages,
            max: MAX_PAGES,
        });
    }
    Ok(())
}

fn check_page_size(width: f32, height: f32) -> Result<(), TileError> {
    if width > MAX_PAGE_MM || height > MAX_PAGE_MM {
        return Err(TileError::PageTooLarge { width, height });
    }
    Ok(())
}

fn pixel_span(start_mm: f32, end_mm: f32, offset_mm: f32, scale: f32, limit: u32) -> (u32, u32) {
    let limit = limit as f32;
    let start = ((start_mm - offset_mm) / scale).round().clamp(0.0, limit) as u32;
    let end = ((end_mm - offset_mm) / scale).round().clamp(0.0, limit) as u32;
    if end < start {
        (start, start)
    } else {
        (start, end)
    }
}

#[cfg(test)]
mod tests;
