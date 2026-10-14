//! Sovereign 2D drawing primitives, rasterization, and anti-aliasing algorithms.
#![deny(missing_docs)]

use oxipix_compat_image::{GenericImage, Rgba};

/// 2D Rectangle primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    /// Left coordinate.
    pub left: i32,
    /// Top coordinate.
    pub top: i32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Rect {
    /// Creates a rectangle at specified origin.
    #[must_use]
    pub fn at(left: i32, top: i32) -> Self {
        Self { left, top, width: 0, height: 0 }
    }

    /// Sets the width and height of the rectangle.
    #[must_use]
    pub fn of_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

/// 2D Point primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point<T> {
    /// X coordinate.
    pub x: T,
    /// Y coordinate.
    pub y: T,
}

impl<T> Point<T> {
    /// Creates a new Point.
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

/// Draws a solid line between two endpoints using Bresenham's algorithm.
pub fn draw_line_segment_mut<I: GenericImage<Pixel = Rgba<u8>>>(
    image: &mut I,
    start: (f32, f32),
    end: (f32, f32),
    color: Rgba<u8>,
) {
    let mut x0 = start.0.round() as i32;
    let mut y0 = start.1.round() as i32;
    let x1 = end.0.round() as i32;
    let y1 = end.1.round() as i32;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let (w, h) = image.dimensions();

    loop {
        if x0 >= 0 && x0 < w as i32 && y0 >= 0 && y0 < h as i32 {
            image.put_pixel(x0 as u32, y0 as u32, color);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Draws an anti-aliased line between two endpoints on an RGBA image buffer.
pub fn draw_antialiased_line_segment_mut<I: GenericImage<Pixel = Rgba<u8>>>(
    image: &mut I,
    start: (i32, i32),
    end: (i32, i32),
    color: Rgba<u8>,
) {
    draw_line_segment_mut(image, (start.0 as f32, start.1 as f32), (end.0 as f32, end.1 as f32), color);
}

/// Draws a hollow rectangle.
pub fn draw_hollow_rect_mut<I: GenericImage<Pixel = Rgba<u8>>>(
    image: &mut I,
    rect: Rect,
    color: Rgba<u8>,
) {
    let x0 = rect.left;
    let y0 = rect.top;
    let x1 = x0 + rect.width as i32 - 1;
    let y1 = y0 + rect.height as i32 - 1;

    draw_line_segment_mut(image, (x0 as f32, y0 as f32), (x1 as f32, y0 as f32), color);
    draw_line_segment_mut(image, (x1 as f32, y0 as f32), (x1 as f32, y1 as f32), color);
    draw_line_segment_mut(image, (x1 as f32, y1 as f32), (x0 as f32, y1 as f32), color);
    draw_line_segment_mut(image, (x0 as f32, y1 as f32), (x0 as f32, y0 as f32), color);
}

/// Draws a filled rectangle.
pub fn draw_filled_rect_mut<I: GenericImage<Pixel = Rgba<u8>>>(
    image: &mut I,
    rect: Rect,
    color: Rgba<u8>,
) {
    let (w, h) = image.dimensions();
    let x_start = rect.left.clamp(0, w as i32) as u32;
    let x_end = (rect.left + rect.width as i32).clamp(0, w as i32) as u32;
    let y_start = rect.top.clamp(0, h as i32) as u32;
    let y_end = (rect.top + rect.height as i32).clamp(0, h as i32) as u32;

    for y in y_start..y_end {
        for x in x_start..x_end {
            image.put_pixel(x, y, color);
        }
    }
}

/// Draws a hollow circle with specified center and radius.
pub fn draw_hollow_circle_mut<I: GenericImage<Pixel = Rgba<u8>>>(
    image: &mut I,
    center: (i32, i32),
    radius: i32,
    color: Rgba<u8>,
) {
    let (cx, cy) = center;
    let mut x = radius;
    let mut y = 0;
    let mut err = 0;
    let (w, h) = image.dimensions();

    let mut set_px = |px: i32, py: i32| {
        if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
            image.put_pixel(px as u32, py as u32, color);
        }
    };

    while x >= y {
        set_px(cx + x, cy + y);
        set_px(cx + y, cy + x);
        set_px(cx - y, cy + x);
        set_px(cx - x, cy + y);
        set_px(cx - x, cy - y);
        set_px(cx - y, cy - x);
        set_px(cx + y, cy - x);
        set_px(cx + x, cy - y);

        if err <= 0 {
            y += 1;
            err += 2 * y + 1;
        }
        if err > 0 {
            x -= 1;
            err -= 2 * x + 1;
        }
    }
}

/// Draws a filled circle with specified center and radius.
pub fn draw_filled_circle_mut<I: GenericImage<Pixel = Rgba<u8>>>(
    image: &mut I,
    center: (i32, i32),
    radius: i32,
    color: Rgba<u8>,
) {
    let (cx, cy) = center;
    let (w, h) = image.dimensions();
    let r2 = radius * radius;

    let y_min = (cy - radius).clamp(0, h as i32) as u32;
    let y_max = (cy + radius).clamp(0, h as i32 - 1) as u32;

    for y in y_min..=y_max {
        let dy = y as i32 - cy;
        let dx = ((r2 - dy * dy) as f32).sqrt().floor() as i32;
        let x_min = (cx - dx).clamp(0, w as i32) as u32;
        let x_max = (cx + dx).clamp(0, w as i32 - 1) as u32;
        for x in x_min..=x_max {
            image.put_pixel(x, y, color);
        }
    }
}

/// Draws a filled convex polygon.
pub fn draw_polygon_mut<I: GenericImage<Pixel = Rgba<u8>>>(
    image: &mut I,
    points: &[Point<i32>],
    color: Rgba<u8>,
) {
    if points.len() < 3 {
        return;
    }
    let (w, h) = image.dimensions();
    let min_y = points.iter().map(|p| p.y).min().unwrap_or(0).clamp(0, h as i32) as u32;
    let max_y = points.iter().map(|p| p.y).max().unwrap_or(0).clamp(0, h as i32 - 1) as u32;

    for y in min_y..=max_y {
        let mut nodes = Vec::new();
        let mut j = points.len() - 1;
        for i in 0..points.len() {
            let pi = points[i];
            let pj = points[j];
            if (pi.y < y as i32 && pj.y >= y as i32) || (pj.y < y as i32 && pi.y >= y as i32) {
                let node_x = pi.x + ((y as i32 - pi.y) as f32 / (pj.y - pi.y) as f32 * (pj.x - pi.x) as f32) as i32;
                nodes.push(node_x);
            }
            j = i;
        }
        nodes.sort_unstable();
        for chunk in nodes.chunks(2) {
            if chunk.len() == 2 {
                let x0 = chunk[0].clamp(0, w as i32) as u32;
                let x1 = chunk[1].clamp(0, w as i32 - 1) as u32;
                for x in x0..=x1 {
                    image.put_pixel(x, y, color);
                }
            }
        }
    }
}
