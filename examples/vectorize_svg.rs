//! Traces raster shapes into SVG paths.
//!
//! Run with `cargo run --example vectorize_svg`. The input is drawn in memory.

use image::{Rgba, RgbaImage};
use imageproc::{drawing, rect::Rect};
use oxipix::vectorize::{VectorizeConfig, vectorize_rgba_to_svg};

fn main() {
    // A black rectangle and a black disc on white paper.
    let black = Rgba([0, 0, 0, 255]);
    let mut scan = RgbaImage::from_pixel(120, 80, Rgba([255, 255, 255, 255]));
    drawing::draw_filled_rect_mut(&mut scan, Rect::at(10, 15).of_size(40, 50), black);
    drawing::draw_filled_circle_mut(&mut scan, (86, 40), 24, black);

    // `grid_step` samples every pixel; `rdp_epsilon` is the simplification tolerance in pixels.
    let config = VectorizeConfig {
        grid_step: 1.0,
        rdp_epsilon: 1.5,
    };
    let graphic = vectorize_rgba_to_svg(&scan, config);

    println!("traced {} closed paths", graphic.paths.len());
    for path in &graphic.paths {
        println!("  {} points", path.points.len());
    }
    println!("{}", graphic.to_svg_string());
}
