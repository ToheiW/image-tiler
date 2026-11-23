use super::*;
use image::{DynamicImage, Rgba, RgbaImage};

#[test]
fn exif_values_map_to_the_right_variant() {
    assert_eq!(Orientation::from_exif_value(1), Orientation::Normal);
    assert_eq!(Orientation::from_exif_value(6), Orientation::Rotate90);
    assert_eq!(Orientation::from_exif_value(8), Orientation::Rotate270);
    assert_eq!(Orientation::from_exif_value(5), Orientation::Transpose);
    assert_eq!(Orientation::from_exif_value(7), Orientation::Transverse);
    // garbage values must not panic
    assert_eq!(Orientation::from_exif_value(0), Orientation::Normal);
    assert_eq!(Orientation::from_exif_value(99), Orientation::Normal);
}

#[test]
fn rotate_90_swaps_dimensions() {
    let out = Orientation::Rotate90.apply(DynamicImage::new_rgb8(4, 2));
    assert_eq!((out.width(), out.height()), (2, 4));
}

#[test]
fn flip_horizontal_moves_the_left_pixel_to_the_right() {
    let mut img = RgbaImage::new(2, 1);
    img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));

    let out = Orientation::FlipH
        .apply(DynamicImage::ImageRgba8(img))
        .to_rgba8();

    assert_eq!(out.get_pixel(1, 0).0, [255, 0, 0, 255]);
    assert_eq!(out.get_pixel(0, 0).0, [0, 0, 0, 0]);
}

#[test]
fn missing_exif_is_normal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plain.png");
    DynamicImage::new_rgb8(2, 2).save(&path).unwrap();

    assert_eq!(read_orientation(&path), Orientation::Normal);
}
