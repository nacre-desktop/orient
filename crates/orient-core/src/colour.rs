//! sRGB/OKLab equations from Björn Ottosson's public-domain definition.
use crate::{sum, Result};
pub fn clean(value: &str) -> Result<String> {
    let v = value.strip_prefix('#').unwrap_or(value);
    if v.len() != 6 || !v.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Expected a six-digit RGB color".into());
    }
    Ok(v.to_ascii_lowercase())
}
pub fn rgb(value: &str) -> Result<[f64; 3]> {
    let v = clean(value)?;
    Ok(std::array::from_fn(|i| {
        u8::from_str_radix(&v[i * 2..i * 2 + 2], 16).unwrap() as f64 / 255.0
    }))
}
pub fn linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
pub fn encoded(c: f64) -> f64 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}
pub fn luminance(value: &str) -> Result<f64> {
    let v = rgb(value)?;
    Ok(sum(v
        .into_iter()
        .zip([0.2126, 0.7152, 0.0722])
        .map(|(v, w)| linear(v) * w)))
}
pub fn contrast(a: &str, b: &str) -> Result<f64> {
    let x = luminance(a)?;
    let y = luminance(b)?;
    Ok((x.max(y) + 0.05) / (x.min(y) + 0.05))
}
pub fn lab_from_rgb([r, g, b]: [f64; 3]) -> [f64; 3] {
    let (r, g, b) = (linear(r), linear(g), linear(b));
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).powf(1.0 / 3.0);
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).powf(1.0 / 3.0);
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).powf(1.0 / 3.0);
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}
pub fn lch(value: &str) -> Result<[f64; 3]> {
    let [l, a, b] = lab_from_rgb(rgb(value)?);
    Ok([l, a.hypot(b), b.atan2(a).to_degrees().rem_euclid(360.0)])
}
pub fn lab_to_linear(l: f64, a: f64, b: f64) -> [f64; 3] {
    let long = (l + 0.3963377774 * a + 0.2158037573 * b).powf(3.0);
    let m = (l - 0.1055613458 * a - 0.0638541728 * b).powf(3.0);
    let s = (l - 0.0894841775 * a - 1.2914855480 * b).powf(3.0);
    [
        4.0767416621 * long - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * long + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * long - 0.7034186147 * m + 1.7076147010 * s,
    ]
}
pub fn color(light: f64, chroma: f64, hue: f64) -> String {
    let light = light.clamp(0.0, 1.0);
    let chroma = chroma.max(0.0);
    let angle = hue.to_radians();
    let channels = |c: f64| lab_to_linear(light, c * angle.cos(), c * angle.sin());
    let inside = |v: [f64; 3]| v.iter().all(|v| (-1e-7..=1.0000001).contains(v));
    let mut values = channels(chroma);
    if !inside(values) {
        let (mut low, mut high) = (0.0, chroma);
        for _ in 0..20 {
            let mid = (low + high) / 2.0;
            if inside(channels(mid)) {
                low = mid
            } else {
                high = mid
            }
        }
        values = channels(low);
    }
    values
        .iter()
        .map(|v| {
            format!(
                "{:02x}",
                (encoded(v.clamp(0.0, 1.0)) * 255.0).round_ties_even() as u8
            )
        })
        .collect()
}
pub fn foreground(bg: &str) -> Result<String> {
    Ok(if contrast("101014", bg)? >= contrast("ffffff", bg)? {
        "101014"
    } else {
        "ffffff"
    }
    .into())
}
pub fn readable(value: &str, backgrounds: &[String], minimum: f64) -> Result<String> {
    let value = clean(value)?;
    let meets = |v: &str, min: f64| -> Result<bool> {
        for bg in backgrounds {
            if contrast(v, bg)? < min {
                return Ok(false);
            }
        }
        Ok(true)
    };
    if meets(&value, minimum)? {
        return Ok(value);
    }
    let [light, c, h] = lch(&value)?;
    let mut targets = [0.0_f64, 1.0];
    targets.sort_by(|a, b| (a - light).abs().total_cmp(&(b - light).abs()));
    let mut choices = Vec::new();
    for target in targets {
        if !meets(&color(target, c, h), minimum)? {
            continue;
        }
        let (mut low, mut high) = (0.0, 1.0);
        for _ in 0..18 {
            let mid = (low + high) / 2.0;
            if meets(
                &color(light + (target - light) * mid, c, h),
                minimum + 0.015,
            )? {
                high = mid
            } else {
                low = mid
            }
        }
        let result = color(light + (target - light) * high, c, h);
        choices.push(((lch(&result)?[0] - light).abs(), result));
    }
    choices.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    choices
        .into_iter()
        .next()
        .map(|v| v.1)
        .ok_or_else(|| "No foreground can meet contrast on these backgrounds".into())
}
pub fn hue_distance(a: f64, b: f64) -> f64 {
    ((a - b + 180.0).rem_euclid(360.0) - 180.0).abs()
}
