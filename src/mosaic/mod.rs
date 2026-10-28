//! 2D image registration, strip overlap discovery, perspective homography unwarping and
//! multi-frame mosaic stitching.

/// Error type for mosaic operations.
pub mod error;
/// Luminance and per-channel helpers.
pub mod luma;
/// Quadrilateral unwarping and dominant-quad dewarp.
pub mod perspective;
/// Strip overlap detection and drift estimation.
pub mod registration;
/// Pairwise and multi-frame stitching.
pub mod stitch;

pub use error::MosaicError;
pub use luma::{map_rgb_channels, map_rgba_channels, rgb_to_luma_u8, rgba_to_luma_vec};
pub use perspective::{auto_dewarp_dominant_quad, unwarp_quad_rgba};
pub use registration::{OverlapAxis, detect_strip_overlap_zmad, estimate_orthogonal_drift_ecc};
pub use stitch::{stitch_frames_feathered, stitch_pair_horizontal, stitch_pair_vertical};
