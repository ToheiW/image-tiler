use super::*;

fn margins(all: f32) -> Margins {
    Margins {
        top: all,
        right: all,
        bottom: all,
        left: all,
    }
}

fn pages_spec(columns: u32, rows: u32) -> LayoutSpec {
    LayoutSpec {
        mode: SizeMode::Pages,
        layout: PdfLayout::OneTilePerPage,
        page_size: PageSize::A4,
        orientation: PageOrientation::Portrait,
        columns,
        rows,
        margins_mm: margins(10.0),
        overlap_mm: 0.0,
        ..Default::default()
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.01,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn a4_pages_contain_the_whole_image_without_cropping() {
    let plan = plan_layout(&pages_spec(2, 1), 1000, 500).unwrap();

    assert_eq!((plan.columns, plan.rows), (2, 1));
    assert_eq!(plan.pages.len(), 2);
    assert_close(plan.page_width_mm, 210.0);
    assert_close(plan.page_height_mm, 297.0);
    assert_close(plan.scale_mm_per_px, 0.36);
    assert_close(plan.effective_dpi, 25.4 / 0.36);

    assert_eq!(plan.pages[0].source.x, 0);
    assert_eq!(plan.pages[0].source.width, 500);
    assert_eq!(plan.pages[1].source.x, 500);
    assert_eq!(plan.pages[1].source.width, 500);
    assert_eq!(
        plan.pages[0].source.width + plan.pages[1].source.width,
        1000
    );

    // The two page windows are one core-width apart, so they reassemble.
    assert_close(
        plan.pages[1].image.x - plan.pages[0].image.x,
        -plan.pages[0].core.width,
    );
}

#[test]
fn per_side_margins_shift_the_core_independently() {
    let mut spec = pages_spec(1, 1);
    spec.margins_mm = Margins {
        top: 20.0,
        right: 10.0,
        bottom: 5.0,
        left: 30.0,
    };

    let plan = plan_layout(&spec, 100, 100).unwrap();
    let core = plan.pages[0].core;

    assert_close(core.x, 5.0 + 30.0);
    assert_close(core.y, 5.0 + 20.0);
    assert_close(core.width, 210.0 - 10.0 - 30.0 - 10.0);
    assert_close(core.height, 297.0 - 10.0 - 20.0 - 5.0);
    // Content never enters the printer-safe band.
    assert!(core.x + core.width <= 210.0 - 5.0);
    assert!(core.y + core.height <= 297.0 - 5.0);
}

#[test]
fn a3_landscape_is_selectable_independently() {
    let mut spec = pages_spec(1, 1);
    spec.page_size = PageSize::A3;
    spec.orientation = PageOrientation::Landscape;

    let plan = plan_layout(&spec, 100, 100).unwrap();

    assert_close(plan.page_width_mm, 420.0);
    assert_close(plan.page_height_mm, 297.0);
}

#[test]
fn overlap_is_limited_by_the_user_margin() {
    let mut spec = pages_spec(2, 1);
    spec.margins_mm = margins(3.0);
    spec.overlap_mm = 10.0;

    let plan = plan_layout(&spec, 1000, 500).unwrap();
    let first = &plan.pages[0];
    let second = &plan.pages[1];

    assert!(first.source.width < 1000);
    assert!(first.source.x + first.source.width > second.source.x);
    // The overlap strip still stops at the printable edge. Pixel rounding can
    // overshoot by at most half a source pixel.
    assert_close(first.dest.x, 5.0 + 3.0);
    let half_pixel = plan.scale_mm_per_px / 2.0;
    assert!(first.dest.x + first.dest.width <= 210.0 - 5.0 + half_pixel);
}

#[test]
fn tile_mode_pages_tile_every_pixel_exactly() {
    let spec = LayoutSpec {
        mode: SizeMode::Tile,
        layout: PdfLayout::OneTilePerPage,
        tile_width: 4,
        tile_height: 3,
        dpi: 300,
        margins_mm: margins(10.0),
        overlap_mm: 0.0,
        ..Default::default()
    };

    let plan = plan_layout(&spec, 10, 6).unwrap();
    assert_eq!((plan.columns, plan.rows, plan.pages.len()), (3, 2, 6));

    let row_zero: Vec<_> = plan.pages.iter().filter(|page| page.row == 0).collect();
    let covered: u32 = row_zero.iter().map(|page| page.source.width).sum();
    assert_eq!(covered, 10);

    // Sorted left-to-right, the sources are adjacent with no gap.
    let mut x = 0;
    for page in row_zero {
        assert_eq!(page.source.x, x);
        x += page.source.width;
    }
}

#[test]
fn fixed_page_pages_are_all_the_same_size() {
    let plan = plan_layout(&pages_spec(2, 2), 4000, 3000).unwrap();
    for page in &plan.pages {
        assert_close(page.page_width_mm, 210.0);
        assert_close(page.page_height_mm, 297.0);
    }
    assert_eq!(plan.pages.len(), 4);
}

#[test]
fn single_page_contains_the_whole_image_on_one_sheet() {
    let mut spec = pages_spec(2, 2);
    spec.layout = PdfLayout::SinglePage;

    let plan = plan_layout(&spec, 1000, 500).unwrap();
    assert_eq!((plan.columns, plan.rows, plan.pages.len()), (1, 1, 1));

    let page = &plan.pages[0];
    assert_eq!((page.source.x, page.source.y), (0, 0));
    assert_eq!((page.source.width, page.source.height), (1000, 500));
    assert!(page.image.width <= page.core.width + 0.01);
    assert!(page.image.height <= page.core.height + 0.01);
}

#[test]
fn rejects_too_many_pages() {
    let spec = pages_spec(100, 100);
    assert!(matches!(
        plan_layout(&spec, 100, 100),
        Err(TileError::TooManyPages { .. })
    ));
}

#[test]
fn rejects_zero_grid() {
    let spec = pages_spec(0, 1);
    assert!(matches!(
        plan_layout(&spec, 100, 100),
        Err(TileError::InvalidGrid)
    ));
}

#[test]
fn rejects_margins_that_leave_no_printable_area() {
    let mut spec = pages_spec(1, 1);
    spec.margins_mm.left = 100.0;
    spec.margins_mm.right = 100.0;
    assert!(matches!(
        plan_layout(&spec, 100, 100),
        Err(TileError::PageTooSmall)
    ));
}

#[test]
fn rejects_pages_beyond_the_pdf_limit() {
    let spec = LayoutSpec {
        mode: SizeMode::Tile,
        layout: PdfLayout::SinglePage,
        dpi: 300,
        margins_mm: margins(10.0),
        ..Default::default()
    };
    assert!(matches!(
        plan_layout(&spec, 100_000, 1_000),
        Err(TileError::PageTooLarge { .. })
    ));
}

#[test]
fn rejects_empty_images() {
    let spec = pages_spec(1, 1);
    assert!(matches!(
        plan_layout(&spec, 0, 10),
        Err(TileError::EmptyImage)
    ));
}
