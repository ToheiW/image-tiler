use std::num::NonZeroU32;
use std::path::PathBuf;

use image::DynamicImage;
use printpdf::{
    Color, Line, LinePoint, Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Point, Pt, RawImage,
    Rgb, XObjectTransform,
};
use serde::{Deserialize, Serialize};

use super::error::TileError;
use super::grid::{plan_grid, TileRect};
use super::orientation::read_orientation;

pub const DEFAULT_DPI: u32 = 300;
pub const DEFAULT_MARGIN_MM: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfLayout {
    /// One PDF page per tile, sized so tiles can be printed and assembled.
    #[default]
    OneTilePerPage,
    /// All tiles placed on a single page reproducing the whole image.
    SinglePage,
}

fn default_dpi() -> u32 {
    DEFAULT_DPI
}

fn default_margin_mm() -> f32 {
    DEFAULT_MARGIN_MM
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfRequest {
    pub input: PathBuf,
    pub output: PathBuf,
    pub tile_width: u32,
    pub tile_height: u32,
    #[serde(default = "default_dpi")]
    pub dpi: u32,
    #[serde(default)]
    pub layout: PdfLayout,
    /// Repeated strip around each tile, for trimming. Only used by `OneTilePerPage`.
    #[serde(default)]
    pub overlap_mm: f32,
    /// White border kept clear of image content so printers don't clip edges.
    /// Only used by `OneTilePerPage`.
    #[serde(default = "default_margin_mm")]
    pub margin_mm: f32,
    /// When set, pages are exactly this size in millimetres and the image is
    /// scaled to fit the grid (contain). Only used by `OneTilePerPage`.
    #[serde(default)]
    pub page_size_mm: Option<[f32; 2]>,
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfReport {
    pub layout: PdfLayout,
    pub columns: u32,
    pub rows: u32,
    pub pages: usize,
    pub output: PathBuf,
}

pub fn tile_to_pdf(request: &PdfRequest) -> Result<PdfReport, TileError> {
    let tile_width = NonZeroU32::new(request.tile_width).ok_or(TileError::InvalidTileSize)?;
    let tile_height = NonZeroU32::new(request.tile_height).ok_or(TileError::InvalidTileSize)?;
    let dpi = NonZeroU32::new(request.dpi).ok_or(TileError::InvalidDpi)?;
    let overlap_px = mm_to_px(request.overlap_mm, dpi.get(), TileError::InvalidOverlap)?;
    let margin_px = mm_to_px(request.margin_mm, dpi.get(), TileError::InvalidMargin)?;

    let page_size = match request.page_size_mm {
        Some([width, height]) => {
            if !width.is_finite()
                || !height.is_finite()
                || width <= 0.0
                || height <= 0.0
            {
                return Err(TileError::InvalidPageSize);
            }
            Some((width, height))
        }
        None => None,
    };

    if request.output.exists() && !request.overwrite {
        return Err(TileError::OutputExists(request.output.clone()));
    }

    // with_guessed_format() sniffs the actual bytes, not just the extension.
    let source = image::ImageReader::open(&request.input)?
        .with_guessed_format()?
        .decode()?;

    // Orientation must be applied before planning: 90°/270° swap width and height.
    let source = read_orientation(&request.input).apply(source);

    let stem = request
        .input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| TileError::InvalidInputName(request.input.clone()))?;

    let plan = plan_grid(source.width(), source.height(), tile_width, tile_height);
    let mut doc = PdfDocument::new(&format!("{stem} tiles"));

    let pages = match request.layout {
        PdfLayout::OneTilePerPage => match page_size {
            Some((width, height)) => render_fixed_pages(
                &mut doc,
                &source,
                plan.columns,
                plan.rows,
                width,
                height,
                request.margin_mm,
                request.overlap_mm,
            )?,
            None => render_one_tile_per_page(
                &mut doc,
                &source,
                &plan.tiles,
                tile_width.get(),
                tile_height.get(),
                overlap_px,
                margin_px,
                dpi.get(),
            )?,
        },
        PdfLayout::SinglePage => render_single_page(&mut doc, &source, &plan.tiles, dpi.get())?,
    };

    let page_count = pages.len();
    doc.with_pages(pages);

    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    if !warnings.is_empty() {
        tracing::debug!(warnings = warnings.len(), "PDF saved with warnings");
    }

    std::fs::write(&request.output, bytes)?;

    tracing::info!(
        input = %request.input.display(),
        output = %request.output.display(),
        layout = ?request.layout,
        pages = page_count,
        "PDF written"
    );

    Ok(PdfReport {
        layout: request.layout,
        columns: plan.columns,
        rows: plan.rows,
        pages: page_count,
        output: request.output.clone(),
    })
}

fn render_one_tile_per_page(
    doc: &mut PdfDocument,
    source: &DynamicImage,
    tiles: &[TileRect],
    tile_width: u32,
    tile_height: u32,
    overlap_px: u32,
    margin_px: u32,
    dpi: u32,
) -> Result<Vec<PdfPage>, TileError> {
    // Every page is the same size: margin | overlap | tile | overlap | margin.
    let page_px_width = tile_width + 2 * overlap_px + 2 * margin_px;
    let page_px_height = tile_height + 2 * overlap_px + 2 * margin_px;

    let insets = (margin_px + overlap_px) as f32;
    let margin = margin_px as f32;

    let mut pages = Vec::with_capacity(tiles.len());
    for tile in tiles {
        let crop = crop_for_tile(tile, overlap_px, source.width(), source.height());
        let crop_image = source.crop_imm(crop.x, crop.y, crop.width, crop.height);

        let image_id = doc.add_image(&raw_image(crop_image)?);

        // Clamped overlap (edges of the source) shifts the image on the page,
        // so the core tile always lands at the same physical position.
        let content_x = insets - crop.left as f32;
        let content_y = insets - crop.bottom as f32;

        let mut ops = vec![Op::UseXobject {
            id: image_id,
            transform: XObjectTransform {
                translate_x: Some(Pt(px_to_pt_f32(content_x, dpi))),
                translate_y: Some(Pt(px_to_pt_f32(content_y, dpi))),
                dpi: Some(dpi as f32),
                ..Default::default()
            },
        }];

        append_crop_marks(
            &mut ops,
            px_to_mm(page_px_width, dpi),
            px_to_mm(page_px_height, dpi),
            px_to_mm_f32(margin, dpi),
            (
                px_to_mm_f32(insets, dpi),
                px_to_mm_f32(insets, dpi),
                px_to_mm_f32(tile.width as f32, dpi),
                px_to_mm_f32(tile.height as f32, dpi),
            ),
        );

        pages.push(PdfPage::new(
            Mm(px_to_mm(page_px_width, dpi)),
            Mm(px_to_mm(page_px_height, dpi)),
            ops,
        ));
    }
    Ok(pages)
}

fn render_single_page(
    doc: &mut PdfDocument,
    source: &DynamicImage,
    tiles: &[TileRect],
    dpi: u32,
) -> Result<Vec<PdfPage>, TileError> {
    let mut ops = Vec::with_capacity(tiles.len());
    for tile in tiles {
        let crop = source.crop_imm(tile.x, tile.y, tile.width, tile.height);
        let image_id = doc.add_image(&raw_image(crop)?);

        ops.push(Op::UseXobject {
            id: image_id,
            transform: XObjectTransform {
                translate_x: Some(Pt(px_to_pt(tile.x, dpi))),
                // PDF coordinates start at the bottom-left corner.
                translate_y: Some(Pt(px_to_pt(
                    source.height() - tile.y - tile.height,
                    dpi,
                ))),
                dpi: Some(dpi as f32),
                ..Default::default()
            },
        });
    }

    let page = PdfPage::new(
        Mm(px_to_mm(source.width(), dpi)),
        Mm(px_to_mm(source.height(), dpi)),
        ops,
    );
    Ok(vec![page])
}

/// Renders a grid of pages with a fixed physical size (e.g. A4), scaling the
/// whole image to fit inside the grid ("contain"), so nothing is cropped.
fn render_fixed_pages(
    doc: &mut PdfDocument,
    source: &DynamicImage,
    columns: u32,
    rows: u32,
    page_width_mm: f32,
    page_height_mm: f32,
    margin_mm: f32,
    overlap_mm: f32,
) -> Result<Vec<PdfPage>, TileError> {
    if columns == 0 || rows == 0 {
        return Ok(Vec::new());
    }

    let insets_mm = margin_mm + overlap_mm;
    let core_width_mm = page_width_mm - 2.0 * insets_mm;
    let core_height_mm = page_height_mm - 2.0 * insets_mm;
    if core_width_mm <= 0.0 || core_height_mm <= 0.0 {
        return Err(TileError::PageTooSmall);
    }

    let image_width = source.width() as f32;
    let image_height = source.height() as f32;
    let total_width_mm = core_width_mm * columns as f32;
    let total_height_mm = core_height_mm * rows as f32;

    // Uniform scale (mm per pixel) that fits the whole image inside the grid.
    let scale = (total_width_mm / image_width).min(total_height_mm / image_height);
    let offset_x = (total_width_mm - image_width * scale) / 2.0;
    let offset_y = (total_height_mm - image_height * scale) / 2.0;
    let effective_dpi = 25.4 / scale;

    let mut pages = Vec::with_capacity((columns * rows) as usize);
    for row in 0..rows {
        for col in 0..columns {
            // Window covered by this page, in grid millimetres from the top-left.
            let window_x = col as f32 * core_width_mm - overlap_mm;
            let window_y = row as f32 * core_height_mm - overlap_mm;
            let window_width_mm = core_width_mm + 2.0 * overlap_mm;
            let window_height_mm = core_height_mm + 2.0 * overlap_mm;

            // Same window in image pixels.
            let source_x = (window_x - offset_x) / scale;
            let source_y = (window_y - offset_y) / scale;
            let source_right = (window_x + window_width_mm - offset_x) / scale;
            let source_bottom = (window_y + window_height_mm - offset_y) / scale;

            // Clamp to the image; blank pages are possible at the outer bands.
            let x0 = source_x.clamp(0.0, image_width).round();
            let y0 = source_y.clamp(0.0, image_height).round();
            let x1 = source_right.clamp(0.0, image_width).round();
            let y1 = source_bottom.clamp(0.0, image_height).round();

            let mut ops = Vec::new();
            if x1 > x0 && y1 > y0 {
                let crop = source.crop_imm(
                    x0 as u32,
                    y0 as u32,
                    (x1 - x0) as u32,
                    (y1 - y0) as u32,
                );
                let image_id = doc.add_image(&raw_image(crop)?);

                let dest_left_mm = margin_mm + x0 * scale - (window_x - offset_x);
                let dest_top_mm = margin_mm + y0 * scale - (window_y - offset_y);
                let dest_width_mm = (x1 - x0) * scale;
                let dest_height_mm = (y1 - y0) * scale;
                let dest_bottom_mm = page_height_mm - dest_top_mm - dest_height_mm;

                ops.push(Op::UseXobject {
                    id: image_id,
                    transform: XObjectTransform {
                        translate_x: Some(Pt(mm_to_pt(dest_left_mm))),
                        translate_y: Some(Pt(mm_to_pt(dest_bottom_mm))),
                        // Natural size at this DPI matches the fit scale exactly.
                        dpi: Some(effective_dpi),
                        ..Default::default()
                    },
                });
            }

            append_crop_marks(
                &mut ops,
                page_width_mm,
                page_height_mm,
                margin_mm,
                (
                    insets_mm,
                    insets_mm,
                    core_width_mm,
                    core_height_mm,
                ),
            );

            pages.push(PdfPage::new(
                Mm(page_width_mm),
                Mm(page_height_mm),
                ops,
            ));
        }
    }

    Ok(pages)
}

/// Short lines in the page margin, aligned with the core tile edges, so the
/// printed page can be trimmed back to exactly one tile.
fn append_crop_marks(
    ops: &mut Vec<Op>,
    page_width_mm: f32,
    page_height_mm: f32,
    margin_mm: f32,
    core: (f32, f32, f32, f32),
) {
    if margin_mm <= 0.0 {
        return;
    }

    let (core_x, core_y, core_width, core_height) = core;

    let mut segments = Vec::with_capacity(8);
    for x in [core_x, core_x + core_width] {
        segments.push((x, 0.0, x, margin_mm));
        segments.push((x, page_height_mm - margin_mm, x, page_height_mm));
    }
    for y in [core_y, core_y + core_height] {
        segments.push((0.0, y, margin_mm, y));
        segments.push((page_width_mm - margin_mm, y, page_width_mm, y));
    }

    ops.push(Op::SetOutlineColor {
        col: Color::Rgb(Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            icc_profile: None,
        }),
    });
    ops.push(Op::SetOutlineThickness { pt: Pt(0.25) });

    for (x1, y1, x2, y2) in segments {
        ops.push(Op::DrawLine {
            line: Line {
                points: vec![
                    LinePoint {
                        p: Point {
                            x: Pt(mm_to_pt(x1)),
                            y: Pt(mm_to_pt(y1)),
                        },
                        bezier: false,
                    },
                    LinePoint {
                        p: Point {
                            x: Pt(mm_to_pt(x2)),
                            y: Pt(mm_to_pt(y2)),
                        },
                        bezier: false,
                    },
                ],
                is_closed: false,
            },
        });
    }
}

fn raw_image(image: DynamicImage) -> Result<RawImage, TileError> {
    // printpdf only accepts 8-bit images; normalize everything else.
    let rgba = DynamicImage::ImageRgba8(image.into_rgba8());
    RawImage::from_dynamic_image(rgba).map_err(TileError::Pdf)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TileCrop {
    /// Crop origin and size in source pixels.
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    /// Pixels of overlap actually available on the left/bottom side.
    left: u32,
    bottom: u32,
}

/// Grows a tile by the overlap strip on all four sides, clamped to the source.
fn crop_for_tile(
    tile: &TileRect,
    overlap_px: u32,
    source_width: u32,
    source_height: u32,
) -> TileCrop {
    let left = overlap_px.min(tile.x);
    let top = overlap_px.min(tile.y);
    let right = overlap_px.min(source_width - (tile.x + tile.width));
    let bottom = overlap_px.min(source_height - (tile.y + tile.height));

    TileCrop {
        x: tile.x - left,
        y: tile.y - top,
        width: tile.width + left + right,
        height: tile.height + top + bottom,
        left,
        bottom,
    }
}

fn mm_to_px(mm: f32, dpi: u32, error: TileError) -> Result<u32, TileError> {
    if !mm.is_finite() || mm < 0.0 {
        return Err(error);
    }
    Ok((mm * dpi as f32 / 25.4).round() as u32)
}

fn px_to_mm(px: u32, dpi: u32) -> f32 {
    px as f32 * 25.4 / dpi as f32
}

fn px_to_mm_f32(px: f32, dpi: u32) -> f32 {
    px * 25.4 / dpi as f32
}

fn mm_to_pt(mm: f32) -> f32 {
    mm * 72.0 / 25.4
}

fn px_to_pt(px: u32, dpi: u32) -> f32 {
    px_to_pt_f32(px as f32, dpi)
}

fn px_to_pt_f32(px: f32, dpi: u32) -> f32 {
    px * 72.0 / dpi as f32
}

#[cfg(test)]
mod tests;
