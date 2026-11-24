//! Straightens a tilted receipt into an upright rectangle.
//!
//! Run with `cargo run --example unwarp_receipt`. The photo is generated in memory, so no input file is needed.

use image::{Rgba, RgbaImage};
use oxipix::mosaic::unwarp_quad_rgba;

fn main() -> Result<(), oxipix::mosaic::MosaicError> {
    // A 400x300 "photo": a smooth colour gradient standing in for a receipt on a desk.
    let photo = RgbaImage::from_fn(400, 300, |x, y| {
        Rgba([(x / 2) as u8, (y / 2) as u8, 128, 255])
    });

    // The receipt's corners in the photo: top-left, top-right, bottom-right, bottom-left.
    let corners = [(40.0, 30.0), (360.0, 50.0), (340.0, 270.0), (20.0, 240.0)];

    // Warp the quadrilateral into a 320x240 upright image.
    let page = unwarp_quad_rgba(&photo, &corners, 320, 240)?;

    println!(
        "unwarped {}x{} photo into {}x{}",
        photo.width(),
        photo.height(),
        page.width(),
        page.height()
    );
    println!("top-left pixel of the page:  {:?}", page.get_pixel(0, 0));
    println!(
        "centre pixel of the page:    {:?}",
        page.get_pixel(160, 120)
    );
    Ok(())
}
