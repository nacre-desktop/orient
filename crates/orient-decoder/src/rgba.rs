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
    encode_pixels(
        image.as_raw(),
        image.width(),
        image.height(),
        4,
        compression,
    )
}
/// Serialize generated RGB artwork with the same bounded Pillow PNG path.
pub fn encode_rgb_png(image: &image::RgbImage, compression: u8) -> Result<Vec<u8>> {
    encode_pixels(
        image.as_raw(),
        image.width(),
        image.height(),
        3,
        compression,
    )
}
fn encode_pixels(
    pixels: &[u8],
    width: u32,
    height: u32,
    channels: usize,
    compression: u8,
) -> Result<Vec<u8>> {
    bounds(width, height)?;
    if compression > 9 {
        return Err("Invalid PNG compression level".into());
    }
    let mut output = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = width.to_be_bytes().to_vec();
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, if channels == 4 { 6 } else { 2 }, 0, 0, 0]);
    artwork::chunk(&mut output, b"IHDR", &header);
    let filtered = artwork::filtered_channels(pixels, width as usize, channels);
    let data = artwork::compress_level(
        &filtered,
        Some(width as usize * channels + 1),
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

/// Resize a fractional source rectangle without rounding it to whole pixels.
/// This is the bounded crop/resample operation used by Pillow's ImageOps.fit.
pub fn resize_rgba_region(
    image: &RgbaImage,
    size: (u32, u32),
    region: (f64, f64, f64, f64),
    filter: Filter,
) -> Result<RgbaImage> {
    bounds(image.width(), image.height())?;
    bounds(size.0, size.1)?;
    let (left, top, right, bottom) = region;
    if ![left, top, right, bottom].iter().all(|v| v.is_finite())
        || left < 0.0
        || top < 0.0
        || right > f64::from(image.width())
        || bottom > f64::from(image.height())
        || left >= right
        || top >= bottom
    {
        return Err("Invalid RGBA source rectangle".into());
    }
    Ok(resample::alpha_region(
        image,
        size,
        (left as f32, top as f32, right as f32, bottom as f32),
        filter,
    ))
}

/// Decode the first image at full resolution into Pillow-compatible RGBA pixels.
/// Keeps raw EXIF orientation and performs no color-profile transform, matching
/// Image.open(path).convert("RGBA"). The shared 50-megapixel bound applies.
pub fn load_rgba(path: &std::path::Path) -> Result<RgbaImage> {
    load_pixels(path, false)
}
/// Decode RGBA and apply EXIF orientation before callers convert/thumbnail it.
pub fn load_rgba_oriented(path: &std::path::Path) -> Result<RgbaImage> {
    load_pixels(path, true)
}
fn load_pixels(path: &std::path::Path, oriented: bool) -> Result<RgbaImage> {
    use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Rgba};
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    let format = reader.format();
    let mut decoder = reader.into_decoder()?;
    let orientation = if oriented {
        Some(decoder.orientation()?)
    } else {
        None
    };
    let (width, height) = decoder.dimensions();
    bounds(width, height)?;
    let (mut image, cmyk) = preview::decode_source(path, format, decoder)?;
    if format == Some(ImageFormat::Gif) {
        preview::restore_gif(&mut image, &std::fs::read(path)?)?;
    }
    if let Some(orientation) = orientation {
        image.apply_orientation(orientation);
    }
    let (width, height) = (image.width(), image.height());
    if cmyk {
        return Ok(preview::opaque(image, true));
    }
    Ok(match image {
        DynamicImage::ImageLuma16(image) => RgbaImage::from_fn(width, height, |x, y| {
            let value = image[(x, y)][0].min(255) as u8;
            Rgba([value, value, value, 255])
        }),
        DynamicImage::ImageLumaA16(image) => RgbaImage::from_fn(width, height, |x, y| {
            let p = image[(x, y)];
            let value = (p[0] >> 8) as u8;
            Rgba([value, value, value, (p[1] >> 8) as u8])
        }),
        DynamicImage::ImageRgb16(image) => RgbaImage::from_fn(width, height, |x, y| {
            let p = image[(x, y)];
            Rgba([(p[0] >> 8) as u8, (p[1] >> 8) as u8, (p[2] >> 8) as u8, 255])
        }),
        DynamicImage::ImageRgba16(image) => RgbaImage::from_fn(width, height, |x, y| {
            Rgba(image[(x, y)].0.map(|value| (value >> 8) as u8))
        }),
        image => image.to_rgba8(),
    })
}

/// Non-upscaling RGBA thumbnail with Pillow's aspect-ratio rounding.
pub fn thumbnail_rgba(image: &RgbaImage, maximum: (u32, u32), filter: Filter) -> Result<RgbaImage> {
    bounds(image.width(), image.height())?;
    bounds(maximum.0, maximum.1)?;
    let size = preview::dimensions(image.width(), image.height(), maximum);
    resize_rgba(image, size, filter)
}

/// RGB thumbnailing retains Pillow's integer pre-reduce before fixed-point resampling.
pub fn thumbnail_rgb(
    image: &image::RgbImage,
    maximum: (u32, u32),
    filter: Filter,
) -> Result<image::RgbImage> {
    bounds(image.width(), image.height())?;
    bounds(maximum.0, maximum.1)?;
    let size = preview::dimensions(image.width(), image.height(), maximum);
    let rgba = image::RgbaImage::from_fn(image.width(), image.height(), |x, y| {
        let p = image[(x, y)];
        image::Rgba([p[0], p[1], p[2], 255])
    });
    let result = resample::thumbnail_region(
        rgba,
        resample::Channels::Opaque,
        false,
        size,
        (f64::from(image.width()), f64::from(image.height())),
        filter,
    );
    Ok(image::DynamicImage::ImageRgba8(result).to_rgb8())
}
