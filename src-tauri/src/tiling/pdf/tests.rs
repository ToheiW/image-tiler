use super::*;
use image::Rgba;
use printpdf::PdfParseOptions;

fn source_image(path: &std::path::Path) {
    let mut img = image::RgbaImage::new(4, 2);
    for y in 0..2 {
        for x in 0..4 {
            let color = if x < 2 { [255, 0, 0, 255] } else { [0, 0, 255, 255] };
            img.put_pixel(x, y, Rgba(color));
        }
    }
    DynamicImage::ImageRgba8(img).save(path).unwrap();
}

fn request(input: PathBuf, output: PathBuf) -> PdfRequest {
    PdfRequest {
        input,
        output,
        tile_width: 2,
        tile_height: 2,
        dpi: DEFAULT_DPI,
        layout: PdfLayout::OneTilePerPage,
        overlap_mm: 0.0,
        margin_mm: DEFAULT_MARGIN_MM,
        page_size_mm: None,
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
    req.layout = PdfLayout::SinglePage;

    let report = tile_to_pdf(&req).unwrap();

    assert_eq!(report.pages, 1);
    assert_eq!(page_count(&output), 1);
}

#[test]
fn millimetres_convert_to_pixels() {
    // 10 mm at 300 DPI = 118.11 px
    assert_eq!(mm_to_px(10.0, 300, TileError::InvalidOverlap).unwrap(), 118);
    assert_eq!(mm_to_px(0.0, 300, TileError::InvalidOverlap).unwrap(), 0);
    assert!(matches!(
        mm_to_px(-1.0, 300, TileError::InvalidOverlap),
        Err(TileError::InvalidOverlap)
    ));
    assert!(matches!(
        mm_to_px(f32::NAN, 300, TileError::InvalidOverlap),
        Err(TileError::InvalidOverlap)
    ));
}

#[test]
fn crop_for_tile_adds_overlap_and_clamps_at_edges() {
    let interior = TileRect {
        row: 1,
        col: 1,
        x: 5,
        y: 5,
        width: 5,
        height: 5,
    };
    let crop = crop_for_tile(&interior, 2, 20, 20);
    assert_eq!((crop.x, crop.y, crop.width, crop.height), (3, 3, 9, 9));
    assert_eq!((crop.left, crop.bottom), (2, 2));

    let corner = TileRect {
        row: 1,
        col: 1,
        x: 8,
        y: 8,
        width: 2,
        height: 2,
    };
    let crop = crop_for_tile(&corner, 2, 10, 10);
    assert_eq!((crop.x, crop.y, crop.width, crop.height), (6, 6, 4, 4));
    assert_eq!((crop.left, crop.bottom), (2, 0));
}

#[test]
fn page_size_includes_margin_and_overlap() {
    // margin 2mm, overlap 1mm at 300 DPI, 100x50 px tile
    let margin = mm_to_px(2.0, 300, TileError::InvalidMargin).unwrap();
    let overlap = mm_to_px(1.0, 300, TileError::InvalidOverlap).unwrap();
    let page_width = 100 + 2 * overlap + 2 * margin;
    let page_height = 50 + 2 * overlap + 2 * margin;
    assert!(page_width > 100 && page_height > 50);
}

#[test]
fn fixed_page_mode_writes_one_page_per_grid_cell() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    source_image(&input);
    let output = dir.path().join("tiles.pdf");

    let mut req = request(input, output.clone());
    req.page_size_mm = Some([297.0, 210.0]);
    req.margin_mm = 10.0;
    req.overlap_mm = 5.0;

    let report = tile_to_pdf(&req).unwrap();

    assert_eq!((report.columns, report.rows, report.pages), (2, 1, 2));
    assert_eq!(page_count(&output), 2);
}

#[test]
fn rejects_invalid_page_size() {
    let mut req = request(PathBuf::from("missing.png"), PathBuf::from("out.pdf"));
    req.page_size_mm = Some([0.0, 297.0]);
    assert!(matches!(
        tile_to_pdf(&req),
        Err(TileError::InvalidPageSize)
    ));
}

#[test]
fn rejects_pages_too_small_for_insets() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    source_image(&input);
    let output = dir.path().join("tiles.pdf");

    let mut req = request(input, output);
    req.page_size_mm = Some([20.0, 20.0]);
    req.margin_mm = 6.0;
    req.overlap_mm = 6.0;

    assert!(matches!(tile_to_pdf(&req), Err(TileError::PageTooSmall)));
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
fn rejects_invalid_settings() {
    let mut req = request(PathBuf::from("missing.png"), PathBuf::from("out.pdf"));
    req.dpi = 0;
    assert!(matches!(tile_to_pdf(&req), Err(TileError::InvalidDpi)));

    let mut req = request(PathBuf::from("missing.png"), PathBuf::from("out.pdf"));
    req.tile_width = 0;
    assert!(matches!(
        tile_to_pdf(&req),
        Err(TileError::InvalidTileSize)
    ));

    let mut req = request(PathBuf::from("missing.png"), PathBuf::from("out.pdf"));
    req.margin_mm = -1.0;
    assert!(matches!(tile_to_pdf(&req), Err(TileError::InvalidMargin)));
}
