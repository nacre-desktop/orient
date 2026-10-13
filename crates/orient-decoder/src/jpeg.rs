//! Match Pillow's 512px draft request before EXIF and thumbnail processing.
use crate::Result;
use image::DynamicImage;
use std::path::Path;

pub fn decode(path: &Path) -> Result<Option<DynamicImage>> {
    let bytes = std::fs::read(path)?;
    let mut decoder = turbojpeg::Decompressor::new()?;
    let header = decoder.read_header(&bytes)?;
    if matches!(
        header.colorspace,
        turbojpeg::Colorspace::CMYK | turbojpeg::Colorspace::YCCK
    ) {
        return Ok(None);
    }
    crate::decoder::check(header.width.try_into()?, header.height.try_into()?)?;
    let ratio = (header.width / 512).min(header.height / 512);
    let scale = if ratio >= 8 {
        turbojpeg::ScalingFactor::ONE_EIGHTH
    } else if ratio >= 4 {
        turbojpeg::ScalingFactor::ONE_QUARTER
    } else if ratio >= 2 {
        turbojpeg::ScalingFactor::ONE_HALF
    } else {
        turbojpeg::ScalingFactor::ONE
    };
    decoder.set_scaling_factor(scale)?;
    let header = header.scaled(scale);
    let mut output = turbojpeg::Image {
        pixels: vec![0; header.width * header.height * 3],
        width: header.width,
        height: header.height,
        pitch: header.width * 3,
        format: turbojpeg::PixelFormat::RGB,
    };
    decoder.decompress(&bytes, output.as_deref_mut())?;
    let image = image::RgbImage::from_raw(
        header.width.try_into()?,
        header.height.try_into()?,
        output.pixels,
    )
    .ok_or("Invalid JPEG dimensions")?;
    Ok(Some(DynamicImage::ImageRgb8(image)))
}
