use std::io::Write;
use std::path::{Path, PathBuf};

use image::DynamicImage;
use printpdf::{
    Color, Line, LinePoint, Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Point, Pt, RawImage, Rgb,
    XObjectTransform,
};
use serde::{Deserialize, Serialize};

use super::error::TileError;
use super::layout::{
    LayoutPlan, LayoutSpec, MAX_PIXELS, PagePlan, oriented_image_dimensions, plan_layout,
};
use super::orientation::read_orientation;

#[derive(Debug, Clone, Deserialize)]
pub struct PdfRequest {
    pub input: PathBuf,
    pub output: PathBuf,
    pub spec: LayoutSpec,
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfReport {
    pub columns: u32,
    pub rows: u32,
    pub pages: usize,
    pub output: PathBuf,
    pub effective_dpi: f32,
    pub page_width_mm: f32,
    pub page_height_mm: f32,
}

pub fn tile_to_pdf(request: &PdfRequest) -> Result<PdfReport, TileError> {
    if !request.output.is_absolute() {
        return Err(TileError::OutputPathNotAbsolute(request.output.clone()));
    }
    if request.output.exists() && !request.overwrite {
        return Err(TileError::OutputExists(request.output.clone()));
    }

    // Header-only check first: a decompression bomb is rejected before its
    // pixels are ever allocated.
    let (header_width, header_height) = oriented_image_dimensions(&request.input)?;
    let pixels = header_width as u64 * header_height as u64;
    if pixels > MAX_PIXELS {
        return Err(TileError::ImageTooLarge {
            width: header_width,
            height: header_height,
            max: MAX_PIXELS,
        });
    }

    // with_guessed_format() sniffs the actual bytes, not just the extension.
    let source = image::ImageReader::open(&request.input)?
        .with_guessed_format()?
        .decode()?;

    // Orientation must be applied before planning: 90°/270° swap width and height.
    let source = read_orientation(&request.input).apply(source);
    let plan = plan_layout(&request.spec, source.width(), source.height())?;

    let title = request
        .input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("image");
    let mut doc = PdfDocument::new(&format!("{title} tiles"));
    let pages = render_pages(&mut doc, &source, &plan)?;
    let page_count = pages.len();
    doc.with_pages(pages);

    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    if !warnings.is_empty() {
        tracing::debug!(warnings = warnings.len(), "PDF saved with warnings");
    }

    write_output(&request.output, &bytes, request.overwrite)?;

    tracing::info!(
        input = %request.input.display(),
        output = %request.output.display(),
        mode = ?request.spec.mode,
        pages = page_count,
        "PDF written"
    );

    Ok(PdfReport {
        columns: plan.columns,
        rows: plan.rows,
        pages: page_count,
        output: request.output.clone(),
        effective_dpi: plan.effective_dpi,
        page_width_mm: plan.page_width_mm,
        page_height_mm: plan.page_height_mm,
    })
}

/// The PDF renderer and the canvas preview both consume this one plan.
fn render_pages(
    doc: &mut PdfDocument,
    source: &DynamicImage,
    plan: &LayoutPlan,
) -> Result<Vec<PdfPage>, TileError> {
    let mut pages = Vec::with_capacity(plan.pages.len());

    for page in &plan.pages {
        let mut ops = Vec::new();

        if page.source.width > 0 && page.source.height > 0 {
            let crop = source.crop_imm(
                page.source.x,
                page.source.y,
                page.source.width,
                page.source.height,
            );
            let image_id = doc.add_image(&raw_image(crop)?);

            ops.push(Op::UseXobject {
                id: image_id,
                transform: XObjectTransform {
                    translate_x: Some(Pt(mm_to_pt(page.dest.x))),
                    // PDF coordinates start at the bottom-left corner.
                    translate_y: Some(Pt(mm_to_pt(
                        page.page_height_mm - page.dest.y - page.dest.height,
                    ))),
                    // Natural size at this DPI matches the plan's scale exactly.
                    dpi: Some(plan.effective_dpi),
                    ..Default::default()
                },
            });
        }

        append_crop_marks(&mut ops, page, plan);
        pages.push(PdfPage::new(
            Mm(page.page_width_mm),
            Mm(page.page_height_mm),
            ops,
        ));
    }

    Ok(pages)
}

/// Short lines in the user margin, aligned with the core tile edges, so a
/// printed page can be trimmed back to exactly one tile. They stop at the
/// printer-safe boundary so no ink lands in the non-printable band.
fn append_crop_marks(ops: &mut Vec<Op>, page: &PagePlan, plan: &LayoutPlan) {
    if plan.pages.len() < 2 {
        return;
    }

    let safe = plan.safe_margin_mm;
    let page_width = page.page_width_mm;
    let page_height = page.page_height_mm;
    let core = page.core;

    let mut segments = Vec::with_capacity(8);
    for x in [core.x, core.x + core.width] {
        if core.y > safe {
            segments.push((x, safe, x, core.y));
        }
        let start = core.y + core.height;
        let end = page_height - safe;
        if end > start {
            segments.push((x, start, x, end));
        }
    }
    for y in [core.y, core.y + core.height] {
        if core.x > safe {
            segments.push((safe, y, core.x, y));
        }
        let start = core.x + core.width;
        let end = page_width - safe;
        if end > start {
            segments.push((start, y, end, y));
        }
    }
    if segments.is_empty() {
        return;
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
                            y: Pt(mm_to_pt(page_height - y1)),
                        },
                        bezier: false,
                    },
                    LinePoint {
                        p: Point {
                            x: Pt(mm_to_pt(x2)),
                            y: Pt(mm_to_pt(page_height - y2)),
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
    // Keep 8-bit RGB/RGBA as-is; only normalize formats printpdf can't embed.
    let image = match image {
        DynamicImage::ImageRgb8(_) | DynamicImage::ImageRgba8(_) => image,
        other => DynamicImage::ImageRgba8(other.into_rgba8()),
    };
    RawImage::from_dynamic_image(image)
        .map_err(|reason| TileError::Pdf(format!("unsupported image format: {reason}")))
}

fn write_output(path: &Path, bytes: &[u8], overwrite: bool) -> Result<(), TileError> {
    if !overwrite {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|source| {
                if source.kind() == std::io::ErrorKind::AlreadyExists {
                    TileError::OutputExists(path.to_path_buf())
                } else {
                    TileError::OutputWrite {
                        path: path.to_path_buf(),
                        source,
                    }
                }
            })?;
        file.write_all(bytes)
            .map_err(|source| TileError::OutputWrite {
                path: path.to_path_buf(),
                source,
            })?;
        file.sync_all().map_err(|source| TileError::OutputWrite {
            path: path.to_path_buf(),
            source,
        })?;
        return Ok(());
    }

    // Overwrite atomically: write a sibling temp file, then rename over the target.
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output.pdf");
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let temp = parent.join(format!(".{file_name}.tmp-{}-{nanos}", std::process::id()));

    let result = (|| -> Result<(), std::io::Error> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)?;
        Ok(())
    })();

    if let Err(source) = result {
        let _ = std::fs::remove_file(&temp);
        return Err(TileError::OutputWrite {
            path: path.to_path_buf(),
            source,
        });
    }
    Ok(())
}

fn mm_to_pt(mm: f32) -> f32 {
    mm * 72.0 / 25.4
}

#[cfg(test)]
mod tests;
