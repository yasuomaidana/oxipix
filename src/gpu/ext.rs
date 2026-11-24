//! Extension trait wiring the GPU kernels into `image` buffers.

use std::ops::Deref;

use image::{ImageBuffer, Rgba, RgbaImage};

use super::{GpuContext, cpu};

const GAUSSIAN_3X3: [f32; 9] = [
    1.0 / 16.0,
    2.0 / 16.0,
    1.0 / 16.0,
    2.0 / 16.0,
    4.0 / 16.0,
    2.0 / 16.0,
    1.0 / 16.0,
    2.0 / 16.0,
    1.0 / 16.0,
];

/// GPU-backed operations on RGBA8 image buffers, with automatic CPU fallback.
pub trait GpuImageExt {
    /// 3x3 convolution over RGB with clamp-to-edge borders; alpha is preserved.
    fn gpu_filter3x3(&self, kernel: &[f32; 9]) -> RgbaImage;

    /// 3x3 Gaussian blur.
    fn gpu_blur3x3(&self) -> RgbaImage {
        self.gpu_filter3x3(&GAUSSIAN_3X3)
    }

    /// Bilinear perspective warp into a `dst_w` x `dst_h` image. `inv_h` maps destination
    /// pixel coordinates to source coordinates (row-major 3x3); uncovered pixels are white.
    fn gpu_warp_perspective(&self, inv_h: &[f32; 9], dst_w: u32, dst_h: u32) -> RgbaImage;
}

impl<C: Deref<Target = [u8]>> GpuImageExt for ImageBuffer<Rgba<u8>, C> {
    fn gpu_filter3x3(&self, kernel: &[f32; 9]) -> RgbaImage {
        let (w, h) = self.dimensions();
        if w == 0 || h == 0 {
            return RgbaImage::new(w, h);
        }
        let raw: &[u8] = self.as_raw();
        let out = GpuContext::global()
            .and_then(|ctx| ctx.convolve3x3(raw, w, h, kernel).ok())
            .unwrap_or_else(|| cpu::convolve3x3(raw, w, h, kernel));
        RgbaImage::from_raw(w, h, out).expect("output length matches dimensions")
    }

    fn gpu_warp_perspective(&self, inv_h: &[f32; 9], dst_w: u32, dst_h: u32) -> RgbaImage {
        let (sw, sh) = self.dimensions();
        if sw == 0 || sh == 0 || dst_w == 0 || dst_h == 0 {
            return RgbaImage::new(dst_w, dst_h);
        }
        let raw: &[u8] = self.as_raw();
        let out = GpuContext::global()
            .and_then(|ctx| ctx.warp_perspective(raw, sw, sh, dst_w, dst_h, inv_h).ok())
            .unwrap_or_else(|| cpu::warp_perspective(raw, sw, sh, dst_w, dst_h, inv_h));
        RgbaImage::from_raw(dst_w, dst_h, out).expect("output length matches dimensions")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| {
            Rgba([
                (x * 255 / w) as u8,
                (y * 255 / h) as u8,
                ((x + y) % 256) as u8,
                200,
            ])
        })
    }

    fn close(a: &[u8], b: &[u8], tol: i16) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(x, y)| (i16::from(*x) - i16::from(*y)).abs() <= tol)
    }

    #[test]
    fn blur_of_flat_image_is_flat() {
        let img = RgbaImage::from_pixel(16, 16, Rgba([128, 128, 128, 255]));
        let out = img.gpu_blur3x3();
        assert!(close(out.as_raw(), img.as_raw(), 1));
    }

    #[test]
    fn dispatch_matches_cpu_reference_for_filter() {
        let img = gradient(61, 37);
        let k = [0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0];
        let got = img.gpu_filter3x3(&k);
        assert!(close(
            got.as_raw(),
            &cpu::convolve3x3(img.as_raw(), 61, 37, &k),
            1
        ));
    }

    #[test]
    fn identity_warp_preserves_image() {
        let img = gradient(40, 30);
        let id = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let out = img.gpu_warp_perspective(&id, 40, 30);
        assert!(close(out.as_raw(), img.as_raw(), 1));
    }

    #[test]
    fn gpu_warp_matches_cpu_reference() {
        let img = gradient(64, 48);
        let h = [0.9, 0.1, 3.0, -0.05, 0.95, 2.0, 0.0004, 0.0002, 1.0];
        let out = img.gpu_warp_perspective(&h, 70, 50);
        let reference = cpu::warp_perspective(img.as_raw(), 64, 48, 70, 50, &h);
        // GPU and CPU float rounding can disagree on whether a pixel sitting exactly on the
        // source boundary is inside (content) or outside (white), so allow a tiny fraction
        // of pixels to differ arbitrarily.
        let off = out
            .as_raw()
            .as_chunks::<4>()
            .0
            .iter()
            .zip(reference.as_chunks::<4>().0)
            .filter(|(a, b)| !close(&a[..], &b[..], 2))
            .count();
        assert!(off * 200 <= 70 * 50, "{off} boundary pixels differ");
    }

    #[test]
    fn zero_sized_input_does_not_panic() {
        let img = RgbaImage::new(0, 0);
        assert_eq!(img.gpu_blur3x3().dimensions(), (0, 0));
    }
}
