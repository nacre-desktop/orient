//! Pillow-compatible thumbnail arithmetic, independently implemented in Rust.
//! Pillow copyright/permission notice: ../LICENSE-Pillow.
//! Reference: Pillow's documented thumbnail/reduce sequence and 8-bit sampling
//! equations (12.3.0). No Pillow source is compiled or linked into the binary.
use image::RgbaImage;
const UNIT: i64 = 1 << 22;
fn dimensions(width: u32, height: u32) -> (u32, u32) {
    if width <= 128 && height <= 128 {
        return (width, height);
    }
    let aspect = width as f64 / height as f64;
    let closest = |number: f64, horizontal: bool| {
        let a = number.floor();
        let b = number.ceil();
        let error = |n: f64| {
            if horizontal {
                (aspect - n / 128.0).abs()
            } else if n == 0.0 {
                0.0
            } else {
                (aspect - 128.0 / n).abs()
            }
        };
        if error(a) <= error(b) {
            a.max(1.0) as u32
        } else {
            b.max(1.0) as u32
        }
    };
    if 1.0 >= aspect {
        (closest(128.0 * aspect, true), 128)
    } else {
        (128, closest(128.0 / aspect, false))
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
fn coefficients(input: usize, output: usize, extent: f32) -> Vec<(usize, Vec<i64>)> {
    let scale = extent as f64 / output as f64;
    let stretch = scale.max(1.0);
    let support = 3.0 * stretch;
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
                let weight = if (-3.0..3.0).contains(&x) {
                    sinc(x) * sinc(x / 3.0)
                } else {
                    0.0
                };
                weights.push(weight);
                sum += weight;
            }
            if sum != 0.0 {
                for w in &mut weights {
                    *w /= sum;
                }
            }
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
fn pass(image: &RgbaImage, output: u32, extent: f32, horizontal: bool) -> RgbaImage {
    let (input, other) = if horizontal {
        (image.width(), image.height())
    } else {
        (image.height(), image.width())
    };
    let kernels = coefficients(input as usize, output as usize, extent);
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
pub fn thumbnail(mut image: RgbaImage, alpha: bool, nearest: bool) -> RgbaImage {
    let (oldw, oldh) = (image.width(), image.height());
    let (w, h) = dimensions(oldw, oldh);
    if (w, h) == (oldw, oldh) {
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
            ((oldw as f64 / w as f64 / 2.0) as u32).max(1),
            ((oldh as f64 / h as f64 / 2.0) as u32).max(1),
        )
    };
    let extent_x = (oldw as f64 / fx as f64) as f32;
    let extent_y = (oldh as f64 / fy as f64) as f32;
    if fx > 1 || fy > 1 {
        image = reduce(&image, fx, fy);
    }
    if image.height() > image.width().saturating_mul(100) && h < image.height() {
        image = pass(&image, h, extent_y, false);
        if image.width() != w || extent_x != image.width() as f32 {
            image = pass(&image, w, extent_x, true);
        }
    } else {
        if image.width() != w || extent_x != image.width() as f32 {
            image = pass(&image, w, extent_x, true);
        }
        if image.height() != h || extent_y != image.height() as f32 {
            image = pass(&image, h, extent_y, false);
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
    if !alpha {
        for p in image.pixels_mut() {
            p[3] = 255;
        }
    }
    image
}
