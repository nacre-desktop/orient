//! Pillow-compatible raw 512px RGB artwork staging; no EXIF/ICC transform.
//! Sampling/filter arithmetic derives from Pillow; see LICENSE-Pillow.
use crate::{
    resample::{self, Channels, Filter},
    Result,
};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use std::{fs, path::Path};

pub(crate) fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], payload: &[u8]) {
    output.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(payload);
    let mut crc = crc32fast::Hasher::new();
    crc.update(kind);
    crc.update(payload);
    output.extend_from_slice(&crc.finalize().to_be_bytes());
}
pub(crate) fn compress(input: &[u8], row_bytes: Option<usize>) -> Result<Vec<u8>> {
    compress_level(input, row_bytes, 6)
}
pub(crate) fn compress_level(
    input: &[u8],
    row_bytes: Option<usize>,
    level: i32,
) -> Result<Vec<u8>> {
    let config = zlib_rs::DeflateConfig {
        level,
        mem_level: if row_bytes.is_some() { 9 } else { 8 },
        strategy: if row_bytes.is_some() {
            zlib_rs::Strategy::Filtered
        } else {
            zlib_rs::Strategy::Default
        },
        ..Default::default()
    };
    if let Some(row_bytes) = row_bytes {
        // Pillow submits each filtered scanline with Z_NO_FLUSH before the
        // final Z_FINISH. A single whole-image call changes deflate matches.
        let mut compressor = zlib_rs::Deflate::new_with_config(config);
        let mut result = Vec::new();
        let mut buffer = [0; 65536];
        for row in input.chunks(row_bytes) {
            let mut remaining = row;
            while !remaining.is_empty() {
                let start_in = compressor.total_in();
                let start_out = compressor.total_out();
                let available = buffer.len() - result.len() % buffer.len();
                compressor
                    .compress(
                        remaining,
                        &mut buffer[..available],
                        zlib_rs::DeflateFlush::NoFlush,
                    )
                    .map_err(|error| format!("PNG compression failed: {error:?}"))?;
                let consumed = (compressor.total_in() - start_in) as usize;
                let written = (compressor.total_out() - start_out) as usize;
                result.extend_from_slice(&buffer[..written]);
                if consumed == 0 && written == 0 {
                    return Err("PNG compressor made no progress".into());
                }
                remaining = &remaining[consumed..];
            }
        }
        loop {
            let start_out = compressor.total_out();
            let available = buffer.len() - result.len() % buffer.len();
            let status = compressor
                .compress(&[], &mut buffer[..available], zlib_rs::DeflateFlush::Finish)
                .map_err(|error| format!("PNG compression failed: {error:?}"))?;
            let written = (compressor.total_out() - start_out) as usize;
            result.extend_from_slice(&buffer[..written]);
            if status == zlib_rs::Status::StreamEnd {
                return Ok(result);
            }
            if written == 0 {
                return Err("PNG compressor did not finish".into());
            }
        }
    }
    let mut output = vec![0; zlib_rs::compress_bound(input.len())];
    let (bytes, result) = zlib_rs::compress_slice(&mut output, input, config);
    if result != zlib_rs::ReturnCode::Ok {
        return Err(format!("PNG compression failed: {result:?}").into());
    }
    Ok(bytes.to_vec())
}
fn score(row: &[u8]) -> u64 {
    row.iter()
        .map(|&value| u64::from(value.min(value.wrapping_neg())))
        .sum()
}
pub(crate) fn filtered(rgb: &[u8], width: usize) -> Vec<u8> {
    filtered_channels(rgb, width, 3)
}
pub(crate) fn filtered_channels(rgb: &[u8], width: usize, channels: usize) -> Vec<u8> {
    let stride = width * channels;
    let mut previous = vec![0; stride];
    let mut output = Vec::with_capacity(rgb.len() + rgb.len() / stride);
    for row in rgb.chunks_exact(stride) {
        let mut best = row.to_vec();
        let mut selector = 0;
        let mut minimum = score(&best);
        for filter in [2, 1, 4] {
            if minimum == 0 {
                break;
            }
            let candidate: Vec<_> = row
                .iter()
                .enumerate()
                .map(|(i, &value)| {
                    let left = if i >= channels { row[i - channels] } else { 0 };
                    let up = previous[i];
                    let corner = if i >= channels {
                        previous[i - channels]
                    } else {
                        0
                    };
                    let predictor = match filter {
                        2 => up,
                        1 => left,
                        _ => {
                            let a = i32::from(left);
                            let b = i32::from(up);
                            let c = i32::from(corner);
                            let pa = (b - c).abs();
                            let pb = (a - c).abs();
                            let pc = (a + b - 2 * c).abs();
                            if pa <= pb && pa <= pc {
                                left
                            } else if pb <= pc {
                                up
                            } else {
                                corner
                            }
                        }
                    };
                    value.wrapping_sub(predictor)
                })
                .collect();
            let sum = score(&candidate);
            if sum < minimum {
                best = candidate;
                selector = filter;
                minimum = sum;
            }
        }
        output.push(selector);
        output.extend_from_slice(&best);
        previous.copy_from_slice(row);
    }
    output
}
pub(crate) fn png_transparency(bytes: &[u8]) -> Option<Vec<u8>> {
    let color = *bytes.get(25)?;
    let depth = *bytes.get(24)?;
    let mut palette = None;
    let mut transparency = None;
    let mut offset = 8;
    while offset + 12 <= bytes.len() {
        let size = u32::from_be_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
        let kind = &bytes[offset + 4..offset + 8];
        let data = bytes.get(offset + 8..offset + 8 + size)?;
        if kind == b"PLTE" {
            palette = Some(data);
        }
        if kind == b"tRNS" {
            transparency = Some(data);
        }
        offset += 12 + size;
    }
    let transparency = transparency?;
    match color {
        2 => Some(transparency.to_vec()),
        0 => {
            let value = u16::from_be_bytes(transparency.try_into().ok()?);
            let value = if depth == 1 {
                if value == 0 {
                    0
                } else {
                    255
                }
            } else {
                value
            };
            Some([value.to_be_bytes(); 3].concat())
        }
        3 => {
            let index = transparency.iter().position(|&value| value == 0)?;
            if transparency
                .iter()
                .enumerate()
                .any(|(i, &value)| i != index && value != 255)
            {
                return None;
            }
            Some(
                palette?
                    .get(index * 3..index * 3 + 3)?
                    .iter()
                    .flat_map(|&value| [0, value])
                    .collect(),
            )
        }
        _ => None,
    }
}
/// Reproduce Nacre's Pillow artwork staging: raw orientation and color samples,
/// a bounded 512px bicubic thumbnail, then RGB PNG bytes with retained ICC/tRNS.
/// This does not apply EXIF orientation or an ICC color-space transformation.
pub fn artwork_png(path: &Path) -> Result<Vec<u8>> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    let format = reader.format();
    let nearest = crate::decoder::nearest_mode(path, format)?;
    let mut decoder = reader.into_decoder()?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > 16_000_000 {
        return Err("Artwork dimensions too large".into());
    }
    let profile = decoder.icc_profile()?;
    let original = fs::read(path)?;
    let mut transparency = if format == Some(ImageFormat::Png) {
        png_transparency(&original)
    } else {
        None
    };
    let mut cmyk = false;
    let mut draft_scale = 1;
    let image = if format == Some(ImageFormat::Jpeg) {
        let packed =
            crate::jpeg::decode_packed(path, 1024, true)?.ok_or("Unsupported JPEG colorspace")?;
        cmyk = packed.cmyk;
        draft_scale = packed.scale;
        if cmyk {
            DynamicImage::ImageRgba8(
                image::RgbaImage::from_raw(packed.width, packed.height, packed.pixels)
                    .ok_or("Invalid JPEG dimensions")?,
            )
        } else {
            DynamicImage::ImageRgb8(
                image::RgbImage::from_raw(packed.width, packed.height, packed.pixels)
                    .ok_or("Invalid JPEG dimensions")?,
            )
        }
    } else {
        DynamicImage::from_decoder(decoder)?
    };
    // Pillow retains grayscale 16-bit samples through thumbnailing, but reads
    // multichannel 16-bit PNG/TIFF samples as their high byte.
    let mut rgba = match &image {
        DynamicImage::ImageRgb16(image) => image::RgbaImage::from_fn(width, height, |x, y| {
            let p = image.get_pixel(x, y);
            image::Rgba([(p[0] >> 8) as u8, (p[1] >> 8) as u8, (p[2] >> 8) as u8, 255])
        }),
        DynamicImage::ImageRgba16(image) => image::RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba(image.get_pixel(x, y).0.map(|value| (value >> 8) as u8))
        }),
        DynamicImage::ImageLumaA16(image) => image::RgbaImage::from_fn(width, height, |x, y| {
            let p = image.get_pixel(x, y);
            let value = (p[0] >> 8) as u8;
            image::Rgba([value, value, value, (p[1] >> 8) as u8])
        }),
        _ => image.to_rgba8(),
    };
    let alpha = if format == Some(ImageFormat::Png) {
        matches!(original.get(25), Some(4 | 6))
    } else {
        !cmyk && image.color().has_alpha()
    };
    if format == Some(ImageFormat::Gif) {
        let mut metadata = gif::DecodeOptions::new().read_info(std::io::Cursor::new(&original))?;
        let first = metadata.next_frame_info()?.cloned();
        if let Some(frame) = first {
            if let Some(index) = frame.transparent {
                if let Some(colors) = frame
                    .palette
                    .as_deref()
                    .or_else(|| metadata.global_palette())
                {
                    if let Some(color) =
                        colors.get(usize::from(index) * 3..usize::from(index) * 3 + 3)
                    {
                        transparency = Some(color.iter().flat_map(|&value| [0, value]).collect());
                        for pixel in rgba.pixels_mut().filter(|pixel| pixel[3] == 0) {
                            pixel.0[0..3].copy_from_slice(color);
                        }
                    }
                }
            }
        }
    }
    let rgba = if let DynamicImage::ImageLuma16(image) = &image {
        resample::thumbnail_gray16(image.clone())?
    } else if format == Some(ImageFormat::Jpeg) {
        // Pillow chooses the output aspect before JPEG draft, then samples the
        // fractional original/scale box rather than the rounded decoder size.
        resample::thumbnail_region(
            rgba,
            if cmyk {
                Channels::Separate
            } else {
                Channels::Opaque
            },
            nearest,
            resample::dimensions(width, height, 512),
            (
                f64::from(width) / f64::from(draft_scale),
                f64::from(height) / f64::from(draft_scale),
            ),
            Filter::Bicubic,
        )
    } else {
        resample::thumbnail_sized(rgba, alpha, nearest, 512, Filter::Bicubic)
    };
    let rgb: Vec<_> = rgba
        .pixels()
        .flat_map(|pixel| {
            if cmyk {
                let black = 255 - u32::from(pixel[3]);
                [pixel[0], pixel[1], pixel[2]].map(|value| {
                    let product = u32::from(value) * black + 128;
                    (black - (((product >> 8) + product) >> 8)) as u8
                })
            } else {
                [pixel[0], pixel[1], pixel[2]]
            }
        })
        .collect();
    let mut output = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend_from_slice(&rgba.width().to_be_bytes());
    header.extend_from_slice(&rgba.height().to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut output, b"IHDR", &header);
    if let Some(profile) = profile {
        let mut data = b"ICC Profile\0\0".to_vec();
        data.extend_from_slice(&compress(&profile, None)?);
        chunk(&mut output, b"iCCP", &data);
    }
    if let Some(transparency) = transparency {
        chunk(&mut output, b"tRNS", &transparency);
    }
    let data = compress(
        &filtered(&rgb, rgba.width() as usize),
        Some(rgba.width() as usize * 3 + 1),
    )?;
    for data in data.chunks(65536) {
        chunk(&mut output, b"IDAT", data);
    }
    chunk(&mut output, b"IEND", &[]);
    Ok(output)
}
