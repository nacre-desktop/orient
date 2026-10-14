//! Bounded RGBA image operations shared by artwork consumers.
//! Pillow-compatible arithmetic/PNG filtering: see ../LICENSE-Pillow (MIT-CMU).
use crate::{artwork, preview, resample, Filter, Result, MAX_PIXELS};
use image::RgbaImage;
fn bounds(width: u32, height: u32) -> Result<()> {
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err("Invalid RGBA image bounds".into());
    }
    Ok(())
}
/// Serialize RGBA pixels with Pillow's scanline filter choices and deflate rounding.
/// Metadata is intentionally absent: the caller owns the generated pixel buffer.
pub fn encode_rgba_png(image: &RgbaImage, compression: u8) -> Result<Vec<u8>> {
    bounds(image.width(), image.height())?;
    if compression > 9 {
        return Err("Invalid PNG compression level".into());
    }
    let mut output = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = image.width().to_be_bytes().to_vec();
    header.extend_from_slice(&image.height().to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    artwork::chunk(&mut output, b"IHDR", &header);
    let filtered = artwork::filtered_channels(image.as_raw(), image.width() as usize, 4);
    let data = artwork::compress_level(
        &filtered,
        Some(image.width() as usize * 4 + 1),
        i32::from(compression),
    )?;
    for data in data.chunks(65536) {
        artwork::chunk(&mut output, b"IDAT", data);
    }
    artwork::chunk(&mut output, b"IEND", &[]);
    Ok(output)
}
/// Resize RGBA using Pillow's premultiplied-alpha fixed-point resampling.
/// Unlike thumbnailing, this operation does not run an integer pre-reduce.
pub fn resize_rgba(image: &RgbaImage, size: (u32, u32), filter: Filter) -> Result<RgbaImage> {
    bounds(image.width(), image.height())?;
    bounds(size.0, size.1)?;
    Ok(resample::thumbnail_region(
        image.clone(),
        resample::Channels::Alpha,
        false,
        size,
        (f64::from(image.width()), f64::from(image.height())),
        filter,
    ))
}
/// Apply Pillow's three-pass Gaussian approximation to all four channels.
pub fn blur_rgba(image: &RgbaImage, radius: f32) -> Result<RgbaImage> {
    bounds(image.width(), image.height())?;
    if !radius.is_finite() || !(0.0..=1000.0).contains(&radius) {
        return Err("Invalid RGBA blur radius".into());
    }
    Ok(preview::blur(image.clone(), radius))
}
