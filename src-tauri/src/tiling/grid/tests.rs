use super::*;

fn n(v: u32) -> NonZeroU32 {
    NonZeroU32::new(v).unwrap()
}

#[test]
fn exact_multiple_produces_full_tiles() {
    let plan = plan_grid(10, 10, n(5), n(5));
    assert_eq!((plan.columns, plan.rows), (2, 2));
    assert_eq!(plan.tiles.len(), 4);
    assert!(plan.tiles.iter().all(|t| (t.width, t.height) == (5, 5)));
}

#[test]
fn remainder_produces_partial_edge_tiles() {
    let plan = plan_grid(12, 10, n(5), n(5));
    assert_eq!((plan.columns, plan.rows), (3, 2));
    assert_eq!(plan.tiles.len(), 6);

    let last_of_first_row = &plan.tiles[2];
    assert_eq!(
        (
            last_of_first_row.col,
            last_of_first_row.x,
            last_of_first_row.width
        ),
        (2, 10, 2)
    );
    assert_eq!(
        plan.tiles.iter().map(|t| t.width * t.height).sum::<u32>(),
        120
    );
}

#[test]
fn tile_larger_than_source_yields_single_tile() {
    let plan = plan_grid(3, 2, n(5), n(5));
    assert_eq!((plan.columns, plan.rows), (1, 1));
    assert_eq!(plan.tiles[0].width, 3);
    assert_eq!(plan.tiles[0].height, 2);
}

#[test]
fn tiles_are_row_major_and_contiguous() {
    let plan = plan_grid(12, 10, n(5), n(5));
    for (i, t) in plan.tiles.iter().enumerate() {
        assert_eq!(i, (t.row * plan.columns + t.col) as usize);
    }
}
