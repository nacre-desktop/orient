//! Pillow-compatible thumbnail arithmetic, independently implemented in Rust.
//! Pillow copyright/permission notice: ../LICENSE-Pillow.
//! Reference: Pillow's documented thumbnail/reduce sequence and 8-bit sampling
//! equations (12.3.0). No Pillow source is compiled or linked into the binary.
use image::RgbaImage;
const UNIT: i64 = 1 << 22;
#[derive(Clone, Copy)]
pub enum Filter {
    Lanczos,
    Bicubic,
}
#[derive(Clone, Copy)]
pub(crate) enum Channels {
    Opaque,
    Alpha,
    Separate,
}

pub(crate) fn dimensions(width: u32, height: u32, maximum: u32) -> (u32, u32) {
    if width <= maximum && height <= maximum {
        return (width, height);
    }
    let aspect = width as f64 / height as f64;
    let closest = |number: f64, horizontal: bool| {
        let a = number.floor();
        let b = number.ceil();
        let error = |n: f64| {
            if horizontal {
                (aspect - n / maximum as f64).abs()
            } else if n == 0.0 {
                0.0
            } else {
                (aspect - maximum as f64 / n).abs()
            }
        };
        if error(a) <= error(b) {
            a.max(1.0) as u32
        } else {
            b.max(1.0) as u32
        }
    };
    if 1.0 >= aspect {
        (closest(maximum as f64 * aspect, true), maximum)
    } else {
        (maximum, closest(maximum as f64 / aspect, false))
    }
}
fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        let angle = x * std::f64::consts::PI;
        angle.sin() / angle
    }
}
fn coefficients(
    input: usize,
    output: usize,
    extent: f32,
    filter: Filter,
) -> Vec<(usize, Vec<i64>)> {
    coefficients_float(input, output, extent, filter)
        .into_iter()
        .map(|(left, weights)| {
            (
                left,
                weights
                    .into_iter()
                    .map(|w| (w * UNIT as f64 + if w < 0.0 { -0.5 } else { 0.5 }) as i64)
                    .collect(),
            )
        })
        .collect()
}

fn coefficients_float(
    input: usize,
    output: usize,
    extent: f32,
    filter: Filter,
) -> Vec<(usize, Vec<f64>)> {
    let scale = extent as f64 / output as f64;
    let stretch = scale.max(1.0);
    let radius = match filter {
        Filter::Lanczos => 3.0,
        Filter::Bicubic => 2.0,
    };
    let support = radius * stretch;
    let inverse = 1.0 / stretch;
    (0..output)
        .map(|i| {
            let center = (i as f64 + 0.5) * scale;
            let left = ((center - support + 0.5) as i64).max(0) as usize;
            let right = (((center + support + 0.5) as i64).max(0) as usize).min(input);
            let mut weights = Vec::new();
            let mut sum = 0.0;
            for p in left..right {
                let x = (p as f64 - center + 0.5) * inverse;
                let weight = match filter {
                    Filter::Lanczos if (-3.0..3.0).contains(&x) => sinc(x) * sinc(x / 3.0),
                    Filter::Bicubic => {
                        let x = x.abs();
                        if x < 1.0 {
                            ((1.5 * x - 2.5) * x) * x + 1.0
                        } else if x < 2.0 {
                            (((-0.5 * x + 2.5) * x - 4.0) * x) + 2.0
                        } else {
                            0.0
                        }
                    }
                    _ => 0.0,
                };
                weights.push(weight);
                sum += weight;
            }
            if sum != 0.0 {
                for w in &mut weights {
                    *w /= sum;
                }
            }
            (left, weights)
        })
        .collect()
}

pub(crate) fn thumbnail_gray16(
    mut image: image::ImageBuffer<image::Luma<u16>, Vec<u16>>,
) -> crate::Result<RgbaImage> {
    let (oldw, oldh) = image.dimensions();
    let (w, h) = dimensions(oldw, oldh, 512);
    let fx = ((oldw as f64 / w as f64 / 2.0) as u32).max(1);
    let fy = ((oldh as f64 / h as f64 / 2.0) as u32).max(1);
    if fx > 1 || fy > 1 {
        // Pillow 12.3 refuses integer pre-reduction for mode I;16.
        return Err("image has wrong mode".into());
    }
    for (horizontal, output, extent) in [
        (true, w, oldw as f64 / fx as f64),
        (false, h, oldh as f64 / fy as f64),
    ] {
        let input = if horizontal {
            image.width()
        } else {
            image.height()
        };
        if input == output && extent == f64::from(input) {
            continue;
        }
        let kernels = coefficients_float(
            input as usize,
            output as usize,
            extent as f32,
            Filter::Bicubic,
        );
        let mut resized = image::ImageBuffer::new(
            if horizontal { output } else { image.width() },
            if horizontal { image.height() } else { output },
        );
        for (x, y, pixel) in resized.enumerate_pixels_mut() {
            let (start, weights) = &kernels[if horizontal { x } else { y } as usize];
            let mut value = 0.0;
            for (offset, weight) in weights.iter().enumerate() {
                let index = (start + offset) as u32;
                value += f64::from(
                    image.get_pixel(
                        if horizontal { index } else { x },
                        if horizontal { y } else { index },
                    )[0],
                ) * weight;
            }
            *pixel = image::Luma([value.round().clamp(0.0, 65535.0) as u16]);
        }
        image = resized;
    }
    Ok(RgbaImage::from_fn(w, h, |x, y| {
        let value = image.get_pixel(x, y)[0].min(255) as u8;
        image::Rgba([value, value, value, 255])
    }))
}
fn pass(
    image: &RgbaImage,
    output: u32,
    extent: f32,
    horizontal: bool,
    filter: Filter,
) -> RgbaImage {
    let (input, other) = if horizontal {
        (image.width(), image.height())
    } else {
        (image.height(), image.width())
    };
    let kernels = coefficients(input as usize, output as usize, extent, filter);
    let mut result = RgbaImage::new(
        if horizontal { output } else { other },
        if horizontal { other } else { output },
    );
    for j in 0..other {
        for (i, (start, kernel)) in kernels.iter().enumerate() {
            let mut pixel = [0; 4];
            for (channel, out) in pixel.iter_mut().enumerate() {
                let mut total = UNIT / 2;
                for (k, w) in kernel.iter().enumerate() {
                    let (x, y) = if horizontal {
                        ((*start + k) as u32, j)
                    } else {
                        (j, (*start + k) as u32)
                    };
                    total += image.get_pixel(x, y)[channel] as i64 * w;
                }
                *out = (total >> 22).clamp(0, 255) as u8;
            }
            let (x, y) = if horizontal {
                (i as u32, j)
            } else {
                (j, i as u32)
            };
            result.put_pixel(x, y, image::Rgba(pixel));
        }
    }
    result
}
fn reduce(image: &RgbaImage, fx: u32, fy: u32) -> RgbaImage {
    let mut result = RgbaImage::new(image.width().div_ceil(fx), image.height().div_ceil(fy));
    for y in 0..result.height() {
        for x in 0..result.width() {
            let width = fx.min(image.width() - x * fx);
            let height = fy.min(image.height() - y * fy);
            let count = u64::from(width) * u64::from(height);
            let mut sum = [count / 2; 4];
            for j in y * fy..y * fy + height {
                for i in x * fx..x * fx + width {
                    for (k, v) in image.get_pixel(i, j).0.iter().enumerate() {
                        sum[k] += u64::from(*v);
                    }
                }
            }
            let multiplier = (16777216.0_f32 / count as f32) as u64;
            result.put_pixel(
                x,
                y,
                image::Rgba(sum.map(|s| ((s * multiplier) >> 24).min(255) as u8)),
            );
        }
    }
    result
}
pub fn thumbnail(image: RgbaImage, alpha: bool, nearest: bool) -> RgbaImage {
    thumbnail_sized(image, alpha, nearest, 128, Filter::Lanczos)
}

pub fn thumbnail_sized(
    image: RgbaImage,
    alpha: bool,
    nearest: bool,
    maximum: u32,
    filter: Filter,
) -> RgbaImage {
    let (oldw, oldh) = (image.width(), image.height());
    let (w, h) = dimensions(oldw, oldh, maximum);
    thumbnail_region(
        image,
        if alpha {
            Channels::Alpha
        } else {
            Channels::Opaque
        },
        nearest,
        (w, h),
        (oldw as f64, oldh as f64),
        filter,
    )
}

pub(crate) fn thumbnail_region(
    mut image: RgbaImage,
    channels: Channels,
    nearest: bool,
    (w, h): (u32, u32),
    (extent_x, extent_y): (f64, f64),
    filter: Filter,
) -> RgbaImage {
    let alpha = matches!(channels, Channels::Alpha);
    let (oldw, oldh) = (image.width(), image.height());
    if (w, h) == (oldw, oldh) && (extent_x, extent_y) == (oldw as f64, oldh as f64) {
        return image;
    }
    if nearest {
        let mut result = RgbaImage::new(w, h);
        // Pillow's affine scaler advances coordinates by addition. Multiplying
        // the index instead can select the next pixel at an integer boundary.
        let dx = oldw as f64 / w as f64;
        let dy = oldh as f64 / h as f64;
        let mut xo = dx * 0.5;
        let columns: Vec<u32> = (0..w)
            .map(|_| {
                let column = (xo as u32).min(oldw - 1);
                xo += dx;
                column
            })
            .collect();
        let mut yo = dy * 0.5;
        for y in 0..h {
            let j = (yo as u32).min(oldh - 1);
            for (x, &i) in columns.iter().enumerate() {
                result.put_pixel(x as u32, y, *image.get_pixel(i, j));
            }
            yo += dy;
        }
        return result;
    }
    if alpha {
        for p in image.pixels_mut() {
            for i in 0..3 {
                let value = u32::from(p[i]) * u32::from(p[3]) + 128;
                p[i] = (((value >> 8) + value) >> 8) as u8;
            }
        }
    }
    let (fx, fy) = if alpha {
        (1, 1)
    } else {
        (
            ((extent_x / w as f64 / 2.0) as u32).max(1),
            ((extent_y / h as f64 / 2.0) as u32).max(1),
        )
    };
    let extent_x = (extent_x / fx as f64) as f32;
    let extent_y = (extent_y / fy as f64) as f32;
    if fx > 1 || fy > 1 {
        image = reduce(&image, fx, fy);
    }
    if image.height() > image.width().saturating_mul(100) && h < image.height() {
        image = pass(&image, h, extent_y, false, filter);
        if image.width() != w || extent_x != image.width() as f32 {
            image = pass(&image, w, extent_x, true, filter);
        }
    } else {
        if image.width() != w || extent_x != image.width() as f32 {
            image = pass(&image, w, extent_x, true, filter);
        }
        if image.height() != h || extent_y != image.height() as f32 {
            image = pass(&image, h, extent_y, false, filter);
        }
    }
    if alpha {
        for p in image.pixels_mut() {
            if p[3] != 0 && p[3] != 255 {
                for i in 0..3 {
                    p[i] = (u32::from(p[i]) * 255 / u32::from(p[3])).min(255) as u8;
                }
            }
        }
    }
    if matches!(channels, Channels::Opaque) {
        for p in image.pixels_mut() {
            p[3] = 255;
        }
    }
    image
}
