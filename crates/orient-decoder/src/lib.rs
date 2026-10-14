//! Bounded native image/video normalization shared by Orient and Rust consumers.
//! Thumbnail arithmetic follows Pillow; see LICENSE-Pillow for MIT-CMU notices.
#![forbid(unsafe_code)]
mod artwork;
mod decoder;
mod jpeg;
mod preview;
pub use preview::rgb_preview_png;
mod resample;
pub use artwork::artwork_png;
pub use decoder::{frames, is_video, still, subprocess, MAX_PIXELS};
pub static CANCELLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn error(s: &str) -> Box<dyn std::error::Error + Send + Sync> {
    orient_core::Error::Value(s.to_owned()).into()
}
fn fail<T>(s: &str) -> Result<T> {
    Err(error(s))
}
