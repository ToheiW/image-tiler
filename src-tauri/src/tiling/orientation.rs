use std::io::BufReader;
use std::path::Path;

use exif::{In, Reader, Tag};
use image::DynamicImage;

/// EXIF orientation: a transform to apply, not a rotation angle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Normal,
    FlipH,
    Rotate180,
    FlipV,
    Transpose,
    Rotate90,
    Transverse,
    Rotate270,
}

impl Orientation {
    /// Maps the raw EXIF tag value (1-8). Unknown/missing values mean Normal.
    pub fn from_exif_value(value: u32) -> Self {
        match value {
            2 => Orientation::FlipH,
            3 => Orientation::Rotate180,
            4 => Orientation::FlipV,
            5 => Orientation::Transpose,
            6 => Orientation::Rotate90,
            7 => Orientation::Transverse,
            8 => Orientation::Rotate270,
            _ => Orientation::Normal,
        }
    }

    /// True when applying this orientation exchanges width and height.
    pub fn swaps_dimensions(self) -> bool {
        matches!(
            self,
            Orientation::Transpose
                | Orientation::Rotate90
                | Orientation::Transverse
                | Orientation::Rotate270
        )
    }

    pub fn apply(self, image: DynamicImage) -> DynamicImage {
        match self {
            Orientation::Normal => image,
            Orientation::FlipH => image.fliph(),
            Orientation::Rotate180 => image.rotate180(),
            Orientation::FlipV => image.flipv(),
            // transpose = 90° CW then mirror horizontally
            Orientation::Transpose => image.rotate90().fliph(),
            Orientation::Rotate90 => image.rotate90(),
            // transverse = mirror horizontally, then 90° CW
            Orientation::Transverse => image.fliph().rotate90(),
            Orientation::Rotate270 => image.rotate270(),
        }
    }
}

/// Reads the orientation tag. A missing or unreadable EXIF block is normal
/// (most PNGs have none), so this always succeeds and defaults to `Normal`.
pub fn read_orientation(path: &Path) -> Orientation {
    match read_exif_orientation(path) {
        Ok(Some(value)) => Orientation::from_exif_value(value),
        Ok(None) => Orientation::Normal,
        Err(err) => {
            tracing::debug!(%err, "no EXIF orientation available; assuming normal");
            Orientation::Normal
        }
    }
}

fn read_exif_orientation(path: &Path) -> Result<Option<u32>, exif::Error> {
    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let exif = Reader::new().read_from_container(&mut reader)?;
    let value = exif
        .get_field(Tag::Orientation, In::PRIMARY)
        .and_then(|field| field.value.get_uint(0));
    Ok(value)
}

#[cfg(test)]
mod tests;
