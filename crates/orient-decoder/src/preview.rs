//! Pillow-compatible oriented RGB previews, without desktop/session assumptions.
//! Box blur equations/rounding follow Pillow 12.3.0 BoxBlur.c (MIT-CMU).
//! See ../LICENSE-Pillow and https://github.com/python-pillow/Pillow/blob/12.3.0/src/libImaging/BoxBlur.c.
use crate::{
    artwork,
    resample::{self, Channels, Filter},
    Result,
};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Rgba, RgbaImage};
use std::{fs, path::Path};
fn dimensions(width: u32, height: u32, bounds: (u32, u32)) -> (u32, u32) {
    if width <= bounds.0 && height <= bounds.1 {
        return (width, height);
    }
    let aspect = f64::from(width) / f64::from(height);
    let closest = |number: f64, horizontal: bool| {
        let error = |n: f64| {
            if horizontal {
                (aspect - n / f64::from(bounds.1)).abs()
            } else if n == 0.0 {
                0.0
            } else {
                (aspect - f64::from(bounds.0) / n).abs()
            }
        };
        let a = number.floor();
        let b = number.ceil();
        if error(a) <= error(b) {
            a.max(1.0) as u32
        } else {
            b.max(1.0) as u32
        }
    };
    if f64::from(bounds.0) / f64::from(bounds.1) >= aspect {
        (closest(f64::from(bounds.1) * aspect, true), bounds.1)
    } else {
        (bounds.0, closest(f64::from(bounds.0) / aspect, false))
    }
}
fn opaque(image: DynamicImage, cmyk: bool) -> RgbaImage {
    match image {
        DynamicImage::ImageLuma16(image) => {
            RgbaImage::from_fn(image.width(), image.height(), |x, y| {
                let value = image[(x, y)].0[0].min(255) as u8;
                Rgba([value, value, value, 255])
            })
        }
        DynamicImage::ImageLumaA16(image) => {
            RgbaImage::from_fn(image.width(), image.height(), |x, y| {
                let value = (image[(x, y)].0[0] >> 8) as u8;
                Rgba([value, value, value, 255])
            })
        }
        DynamicImage::ImageRgb16(image) => {
            RgbaImage::from_fn(image.width(), image.height(), |x, y| {
                let p = image[(x, y)].0;
                Rgba([(p[0] >> 8) as u8, (p[1] >> 8) as u8, (p[2] >> 8) as u8, 255])
            })
        }
        DynamicImage::ImageRgba16(image) => {
            RgbaImage::from_fn(image.width(), image.height(), |x, y| {
                let p = image[(x, y)].0;
                Rgba([(p[0] >> 8) as u8, (p[1] >> 8) as u8, (p[2] >> 8) as u8, 255])
            })
        }
        image => {
            let mut rgba = image.to_rgba8();
            for pixel in rgba.pixels_mut() {
                if cmyk {
                    let black = 255 - u32::from(pixel[3]);
                    for channel in &mut pixel.0[..3] {
                        let product = u32::from(*channel) * black + 128;
                        *channel = (black - (((product >> 8) + product) >> 8)) as u8;
                    }
                }
                pixel[3] = 255;
            }
            rgba
        }
    }
}
fn box_pass(image: &RgbaImage, float_radius: f32) -> RgbaImage {
    let radius = float_radius as usize;
    let weight = (16777216.0f32 / (float_radius * 2.0 + 1.0)) as u32;
    let far = (16777216 - ((radius * 2 + 1) as u32) * weight) / 2;
    let mut output = RgbaImage::new(image.width(), image.height());
    let width = image.width() as usize;
    for y in 0..image.height() {
        let sample = |x: isize| image[(x.clamp(0, width as isize - 1) as u32, y)].0;
        let mut sums = [0u32; 4];
        for x in -(radius as isize)..=radius as isize {
            let pixel = sample(x);
            for c in 0..4 {
                sums[c] += u32::from(pixel[c]);
            }
        }
        for x in 0..width {
            let left = sample(x as isize - radius as isize - 1);
            let right = sample(x as isize + radius as isize + 1);
            for c in 0..4 {
                output[(x as u32, y)].0[c] = ((sums[c] * weight
                    + (u32::from(left[c]) + u32::from(right[c])) * far
                    + (1 << 23))
                    >> 24) as u8;
            }
            let removed = sample(x as isize - radius as isize);
            let added = sample(x as isize + radius as isize + 1);
            for c in 0..4 {
                sums[c] = sums[c] - u32::from(removed[c]) + u32::from(added[c]);
            }
        }
    }
    output
}
fn blur(mut image: RgbaImage, radius: f32) -> RgbaImage {
    if radius == 0.0 {
        return image;
    }
    let sigma2 = radius * radius / 3.0;
    let length = (12.0 * f64::from(sigma2) + 1.0).sqrt() as f32;
    let integer = ((f64::from(length) - 1.0) / 2.0).floor() as f32;
    let mut fraction = (2.0 * integer + 1.0) * (integer * (integer + 1.0) - 3.0 * sigma2);
    fraction /= 6.0 * (sigma2 - (integer + 1.0) * (integer + 1.0));
    let box_radius = integer + fraction;
    for _ in 0..3 {
        image = box_pass(&image, box_radius);
    }
    image = RgbaImage::from_fn(image.height(), image.width(), |x, y| image[(y, x)]);
    for _ in 0..3 {
        image = box_pass(&image, box_radius);
    }
    RgbaImage::from_fn(image.height(), image.width(), |x, y| image[(y, x)])
}
/// Read a bounded image, apply EXIF orientation, convert RGB before bicubic
/// thumbnailing, and optionally blur with Pillow's three-pass box approximation.
/// Returns PNG bytes with ICC/transparency preserved; no files are written.
pub fn rgb_preview_png(path: &Path, bounds: (u32, u32), radius: f32) -> Result<Vec<u8>> {
    if bounds.0 == 0 || bounds.1 == 0 || !radius.is_finite() || !(0.0..=1000.0).contains(&radius) {
        return Err("Invalid RGB preview bounds or blur radius".into());
    }
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    let format = reader.format();
    let mut decoder = reader.into_decoder()?;
    let (width, height) = decoder.dimensions();
    crate::decoder::check(width, height)?;
    let profile = decoder.icc_profile()?;
    let orientation = decoder.orientation()?;
    let original = fs::read(path)?;
    let mut transparency = if format == Some(ImageFormat::Png) {
        artwork::png_transparency(&original)
    } else {
        None
    };
    let (mut image, cmyk) = if format == Some(ImageFormat::Jpeg) {
        let packed = crate::jpeg::decode_packed(path, usize::MAX, true)?
            .ok_or("Unsupported JPEG colorspace")?;
        let image = if packed.cmyk {
            DynamicImage::ImageRgba8(
                RgbaImage::from_raw(packed.width, packed.height, packed.pixels)
                    .ok_or("Invalid JPEG dimensions")?,
            )
        } else {
            DynamicImage::ImageRgb8(
                image::RgbImage::from_raw(packed.width, packed.height, packed.pixels)
                    .ok_or("Invalid JPEG dimensions")?,
            )
        };
        (image, packed.cmyk)
    } else {
        (DynamicImage::from_decoder(decoder)?, false)
    };
    image.apply_orientation(orientation);
    if format == Some(ImageFormat::Gif) {
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::Indexed);
        let mut gif = options.read_info(std::io::Cursor::new(&original))?;
        let global = gif.global_palette().map(<[u8]>::to_vec);
        if let Some(frame) = gif.read_next_frame()? {
            if let Some(index) = frame.transparent {
                if let Some(palette) = frame.palette.as_ref().or(global.as_ref()) {
                    if let Some(color) =
                        palette.get(usize::from(index) * 3..usize::from(index) * 3 + 3)
                    {
                        transparency = Some(color.iter().flat_map(|c| [0, *c]).collect());
                        let mut rgba = image.to_rgba8();
                        // GIF decoder canvases may clear transparent RGB. Pillow
                        // retains the palette color, including outside an offset frame.
                        for pixel in rgba.pixels_mut().filter(|pixel| pixel[3] == 0) {
                            pixel.0[..3].copy_from_slice(color);
                        }
                        image = DynamicImage::ImageRgba8(rgba);
                    }
                }
            }
        }
    }
    let mut rgba = opaque(image, cmyk);
    let extent = (f64::from(rgba.width()), f64::from(rgba.height()));
    let size = dimensions(rgba.width(), rgba.height(), bounds);
    rgba = resample::thumbnail_region(rgba, Channels::Opaque, false, size, extent, Filter::Bicubic);
    rgba = blur(rgba, radius);
    let rgb: Vec<_> = rgba
        .pixels()
        .flat_map(|pixel| pixel.0[..3].to_vec())
        .collect();
    let mut output = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend_from_slice(&rgba.width().to_be_bytes());
    header.extend_from_slice(&rgba.height().to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    artwork::chunk(&mut output, b"IHDR", &header);
    if let Some(profile) = profile {
        let mut data = b"ICC Profile\0\0".to_vec();
        data.extend_from_slice(&artwork::compress(&profile, None)?);
        artwork::chunk(&mut output, b"iCCP", &data);
    }
    if let Some(transparency) = transparency {
        artwork::chunk(&mut output, b"tRNS", &transparency);
    }
    let data = artwork::compress(
        &artwork::filtered(&rgb, rgba.width() as usize),
        Some(rgba.width() as usize * 3 + 1),
    )?;
    for data in data.chunks(65536) {
        artwork::chunk(&mut output, b"IDAT", data);
    }
    artwork::chunk(&mut output, b"IEND", &[]);
    Ok(output)
}
