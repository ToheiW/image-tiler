use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileRect {
    pub row: u32,
    pub col: u32,
    pub x: u32,
    pub y: u32,
    pub height: u32,
    pub width: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridPlan {
    pub source_width: u32,
    pub source_height: u32,
    pub columns: u32,
    pub rows: u32,
    pub tiles: Vec<TileRect>,
}

pub fn plan_grid(
    source_width: u32,
    source_height: u32,
    tile_width: NonZeroU32,
    tile_height: NonZeroU32,
) -> GridPlan {
    let tw = tile_width.get();
    let th = tile_height.get();

    let columns = source_width.div_ceil(tw);
    let rows = source_height.div_ceil(th);

    let mut tiles = Vec::with_capacity((columns as usize) * (rows as usize));

    for row in 0..rows{
        for col in 0..columns {
            let x = col * tw;
            let y = row * th;
            tiles.push(TileRect {
                row,
                col,
                x,
                y,
                height: th.min(source_height - y),
                width: tw.min(source_width - x),
            });
        }
    }

    GridPlan {
        source_width,
        source_height,
        columns,
        rows,
        tiles,
    }
}

#[cfg(test)]
mod tests;
