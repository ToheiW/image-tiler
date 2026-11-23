use super::*;
use crate::tiling::{LayoutSpec, Margins, PageOrientation, PageSize, PdfLayout, SizeMode};
use image::Rgba;
use printpdf::PdfParseOptions;

fn source_image(path: &std::path::Path) {
    let mut img = image::RgbaImage::new(4, 2);
    for y in 0..2 {
        for x in 0..4 {
            let color = if x < 2 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            };
            img.put_pixel(x, y, Rgba(color));
        }
    }
    DynamicImage::ImageRgba8(img).save(path).unwrap();
}

fn tile_spec(tile_width: u32, tile_height: u32) -> LayoutSpec {
    LayoutSpec {
        mode: SizeMode::Tile,
        layout: PdfLayout::OneTilePerPage,
        tile_width,
        tile_height,
        dpi: 300,
        margins_mm: Margins {
            top: 10.0,
            right: 10.0,
            bottom: 10.0,
            left: 10.0,
        },
        overlap_mm: 0.0,
        ..Default::default()
    }
}

fn request(input: PathBuf, output: PathBuf) -> PdfRequest {
    PdfRequest {
        input,
        output,
        spec: tile_spec(2, 2),
        overwrite: false,
    }
}

fn page_count(path: &std::path::Path) -> usize {
    let bytes = std::fs::read(path).unwrap();
    let mut warnings = Vec::new();
    PdfDocument::parse(&bytes, &PdfParseOptions::default(), &mut warnings)
        .unwrap()
        .page_count()
}

#[test]
fn one_tile_per_page_writes_one_page_per_tile() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    source_image(&input);
    let output = dir.path().join("tiles.pdf");

    let report = tile_to_pdf(&request(input, output.clone())).unwrap();

    assert_eq!((report.columns, report.rows, report.pages), (2, 1, 2));
    let bytes = std::fs::read(&output).unwrap();
    assert!(bytes.starts_with(b"%PDF"));
    assert_eq!(page_count(&output), 2);
}

#[test]
fn single_page_writes_one_page() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    source_image(&input);
    let output = dir.path().join("tiles.pdf");

    let mut req = request(input, output.clone());
    req.spec.layout = PdfLayout::SinglePage;

    let report = tile_to_pdf(&req).unwrap();

    assert_eq!(report.pages, 1);
    assert_eq!(page_count(&output), 1);
}

#[test]
fn fixed_page_mode_writes_one_page_per_grid_cell() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    source_image(&input);
    let output = dir.path().join("tiles.pdf");

    let mut req = request(input, output.clone());
    req.spec = LayoutSpec {
        mode: SizeMode::Pages,
        layout: PdfLayout::OneTilePerPage,
        page_size: PageSize::A4,
        orientation: PageOrientation::Portrait,
        columns: 2,
        rows: 1,
        margins_mm: Margins {
            top: 10.0,
            right: 10.0,
            bottom: 10.0,
            left: 10.0,
        },
        overlap_mm: 5.0,
        ..Default::default()
    };

    let report = tile_to_pdf(&req).unwrap();

    assert_eq!((report.columns, report.rows, report.pages), (2, 1, 2));
    assert_eq!(report.page_width_mm, 210.0);
    assert_eq!(report.page_height_mm, 297.0);
    assert_eq!(page_count(&output), 2);
}

#[test]
fn a3_landscape_page_uses_independent_orientation() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    source_image(&input);
    let output = dir.path().join("tiles.pdf");

    let mut req = request(input, output);
    req.spec = LayoutSpec {
        mode: SizeMode::Pages,
        layout: PdfLayout::SinglePage,
        page_size: PageSize::A3,
        orientation: PageOrientation::Landscape,
        margins_mm: Margins::default(),
        overlap_mm: 0.0,
        ..Default::default()
    };

    let report = tile_to_pdf(&req).unwrap();

    assert_eq!(report.pages, 1);
    assert_eq!(report.page_width_mm, 420.0);
    assert_eq!(report.page_height_mm, 297.0);
}

#[test]
fn rejects_invalid_settings() {
    // Planning needs real dimensions, so use a tiny source image.
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    source_image(&input);
    let output = dir.path().join("out.pdf");

    let mut req = request(input.clone(), output.clone());
    req.spec.dpi = 0;
    assert!(matches!(tile_to_pdf(&req), Err(TileError::InvalidDpi)));

    let mut req = request(input.clone(), output.clone());
    req.spec.tile_width = 0;
    assert!(matches!(tile_to_pdf(&req), Err(TileError::InvalidTileSize)));

    let mut req = request(input.clone(), output.clone());
    req.spec.margins_mm.left = -1.0;
    assert!(matches!(tile_to_pdf(&req), Err(TileError::InvalidMargin)));

    let mut req = request(input, output);
    req.spec.overlap_mm = f32::NAN;
    assert!(matches!(tile_to_pdf(&req), Err(TileError::InvalidOverlap)));
}

#[test]
fn rejects_relative_output_paths() {
    let req = request(PathBuf::from("missing.png"), PathBuf::from("out.pdf"));
    assert!(matches!(
        tile_to_pdf(&req),
        Err(TileError::OutputPathNotAbsolute(_))
    ));
}

#[test]
fn refuses_to_overwrite_unless_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    source_image(&input);
    let output = dir.path().join("tiles.pdf");

    tile_to_pdf(&request(input.clone(), output.clone())).unwrap();

    let err = tile_to_pdf(&request(input.clone(), output.clone())).unwrap_err();
    assert!(matches!(err, TileError::OutputExists(_)));

    let mut allowed = request(input, output);
    allowed.overwrite = true;
    assert!(tile_to_pdf(&allowed).is_ok());
}

#[test]
fn write_errors_carry_the_target_path() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    source_image(&input);
    // Renaming a file over a directory fails, exercising the error path.
    let output = dir.path().to_path_buf();

    let mut req = request(input, output.clone());
    req.overwrite = true;
    let err = tile_to_pdf(&req).unwrap_err();
    let TileError::OutputWrite { path, .. } = err else {
        panic!("expected OutputWrite, got {err}");
    };
    assert_eq!(path, output);
}
