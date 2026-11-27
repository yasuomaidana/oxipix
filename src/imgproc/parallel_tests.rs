//! The parallel and the sequential path of every banded kernel must give identical results.

use super::rows::with_min_elements;
use super::{bilateral, blur, filter2d, median, pyramid, warp};
use crate::imgproc::testing::noise;
use image::{GrayImage, Rgb, RgbImage};
use rstest::rstest;

type Kernel = fn(&RgbImage) -> Vec<u8>;

/// A colour picture with different content in every channel.
fn picture() -> RgbImage {
    let grey = noise(97, 61, 11);
    RgbImage::from_fn(97, 61, |x, y| {
        let v = grey.get_pixel(x, y).0[0];
        Rgb([v, v.wrapping_mul(3), 255 - v])
    })
}

fn gaussian(image: &RgbImage) -> Vec<u8> {
    blur::gaussian_blur(image, 1.4).into_raw()
}

fn boxed(image: &RgbImage) -> Vec<u8> {
    blur::box_blur(image, 3).into_raw()
}

fn pyr(image: &RgbImage) -> Vec<u8> {
    pyramid::pyr_down(image).into_raw()
}

fn median_small(image: &RgbImage) -> Vec<u8> {
    median::median_blur(image, 1).into_raw()
}

fn median_large(image: &RgbImage) -> Vec<u8> {
    median::median_blur(image, 2).into_raw()
}

fn bilateral_rgb(image: &RgbImage) -> Vec<u8> {
    bilateral::bilateral_filter(image, 2, 30.0, 2.0).into_raw()
}

fn affine(image: &RgbImage) -> Vec<u8> {
    warp::warp_affine(image, &[0.9, -0.2, 7.0, 0.2, 0.9, -3.0], (120, 70)).into_raw()
}

fn perspective(image: &RgbImage) -> Vec<u8> {
    warp::warp_perspective(
        image,
        &[1.0, 0.05, 2.0, 0.0, 1.0, 1.0, 0.0005, 0.0, 1.0],
        (90, 60),
    )
    .into_raw()
}

fn convolution(image: &RgbImage) -> Vec<u8> {
    filter2d::filter2d(image, &[1.0 / 9.0; 9], 3).into_raw()
}

#[rstest]
#[case(gaussian)]
#[case(boxed)]
#[case(pyr)]
#[case(median_small)]
#[case(median_large)]
#[case(bilateral_rgb)]
#[case(affine)]
#[case(perspective)]
#[case(convolution)]
fn forced_parallel_equals_forced_sequential(#[case] kernel: Kernel) {
    let image = picture();
    let sequential = with_min_elements(usize::MAX, || kernel(&image));
    let parallel = with_min_elements(0, || kernel(&image));
    assert_eq!(sequential, parallel);
    assert!(!sequential.is_empty());
}

#[test]
fn grey_pictures_agree_too() {
    let grey: GrayImage = noise(70, 50, 5);
    let run = |min| with_min_elements(min, || blur::gaussian_blur(&grey, 1.1).into_raw());
    assert_eq!(run(usize::MAX), run(0));
}
