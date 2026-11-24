use thiserror::Error;

/// Operational errors for 2D mosaic and registration operations.
#[derive(Debug, Error)]
pub enum MosaicError {
    /// An image or target dimension was zero or otherwise unusable.
    #[error("Invalid image dimensions: {width}x{height} ({reason})")]
    InvalidDimensions {
        /// Offending width.
        width: u32,
        /// Offending height.
        height: u32,
        /// Why the dimensions were rejected.
        reason: String,
    },

    /// Registration or homography estimation failed.
    #[error("Registration or homography error: {0}")]
    RegistrationError(String),

    /// Frames could not be stitched.
    #[error("Stitching failure: {0}")]
    StitchError(String),

    /// A perspective warp failed.
    #[error("Perspective warp error: {0}")]
    PerspectiveError(String),
}
