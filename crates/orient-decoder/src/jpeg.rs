//! Match Pillow's 512px draft request before EXIF and thumbnail processing.
use crate::Result;
use image::DynamicImage;
use std::path::Path;

pub fn decode(path: &Path) -> Result<Option<DynamicImage>> {
    decode_draft(path, 512)
}

pub(crate) fn decode_draft(path: &Path, maximum: usize) -> Result<Option<DynamicImage>> {
    decode_packed(path, maximum, false)?
        .map(|decoded| {
            let image = image::RgbImage::from_raw(decoded.width, decoded.height, decoded.pixels)
                .ok_or("Invalid JPEG dimensions")?;
            Ok(DynamicImage::ImageRgb8(image))
        })
        .transpose()
}

pub(crate) struct Packed {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub cmyk: bool,
    pub scale: u32,
}

pub(crate) fn decode_packed(
    path: &Path,
    maximum: usize,
    allow_cmyk: bool,
) -> Result<Option<Packed>> {
    let bytes = std::fs::read(path)?;
    let mut decoder = turbojpeg::Decompressor::new()?;
    let header = decoder.read_header(&bytes)?;
    let cmyk = matches!(
        header.colorspace,
        turbojpeg::Colorspace::CMYK | turbojpeg::Colorspace::YCCK
    );
    if cmyk && !allow_cmyk {
        return Ok(None);
    }
    crate::decoder::check(header.width.try_into()?, header.height.try_into()?)?;
    let ratio = (header.width / maximum).min(header.height / maximum);
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
    let divisor = if ratio >= 8 {
        8
    } else if ratio >= 4 {
        4
    } else if ratio >= 2 {
        2
    } else {
        1
    };
    let header = header.scaled(scale);
    let channels = if cmyk { 4 } else { 3 };
    let mut output = turbojpeg::Image {
        pixels: vec![0; header.width * header.height * channels],
        width: header.width,
        height: header.height,
        pitch: header.width * channels,
        format: if cmyk {
            turbojpeg::PixelFormat::CMYK
        } else {
            turbojpeg::PixelFormat::RGB
        },
    };
    decoder.decompress(&bytes, output.as_deref_mut())?;
    if cmyk {
        // Pillow's JPEG raw mode is CMYK;I, assuming Adobe polarity.
        for value in &mut output.pixels {
            *value = !*value;
        }
    }
    Ok(Some(Packed {
        width: header.width.try_into()?,
        height: header.height.try_into()?,
        pixels: output.pixels,
        cmyk,
        scale: divisor,
    }))
}
