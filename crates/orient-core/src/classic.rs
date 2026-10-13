//! Original Natural policy: hue families with coverage and spatial spread.
use crate::colour::*;
use crate::extract::Frame;
use crate::{num, rounded, str_at, Result};
use indexmap::IndexMap;
use serde_json::{json, Value};
#[derive(Default)]
struct Bin {
    weight: f64,
    rgb: [f64; 3],
    tiles: IndexMap<(usize, usize), f64>,
}
#[derive(Default)]
struct Group {
    weight: f64,
    pigment: f64,
    sum: [f64; 3],
    tiles: IndexMap<(usize, usize), f64>,
    lch: [f64; 3],
}
pub fn analyze(images: &[Frame]) -> Result<Value> {
    let image = images.first().ok_or("Invalid normalized RGBA frame")?;
    image.validate()?;
    let mut bins: IndexMap<[u8; 3], Bin> = IndexMap::new();
    let mut total = 0.0;
    for (i, p) in image.pixels().enumerate() {
        if p[3] == 0 {
            continue;
        }
        let weight = p[3] as f64 / 255.0;
        let entry = bins.entry([p[0] >> 3, p[1] >> 3, p[2] >> 3]).or_default();
        entry.weight += weight;
        for (k, v) in p[..3].iter().enumerate() {
            entry.rgb[k] += *v as f64 * weight;
        }
        let (x, y) = (i % image.width, i / image.width);
        let tile = ((x * 4 / image.width).min(3), (y * 4 / image.height).min(3));
        *entry.tiles.entry(tile).or_insert(0.0) += weight;
        total += weight;
    }
    if total == 0.0 {
        return Err("Wallpaper has no visible pixels".into());
    }
    let mut ordered: Vec<_> = bins.values().collect();
    ordered.sort_by(|a, b| {
        b.weight
            .total_cmp(&a.weight)
            .then_with(|| a.rgb.partial_cmp(&b.rgb).unwrap())
    });
    let mut groups: Vec<Group> = Vec::new();
    let (mut mean_light, mut observed, mut mean_a, mut mean_b) = (0.0, 0.0, 0.0, 0.0);
    for bin in ordered {
        let weight = bin.weight;
        let [light, a, b] = lab_from_rgb(bin.rgb.map(|v| v / weight / 255.0));
        let chroma = a.hypot(b);
        let hue = b.atan2(a).to_degrees().rem_euclid(360.0);
        mean_light += light * weight;
        observed += weight;
        mean_a += a * weight;
        mean_b += b * weight;
        let index = groups
            .iter()
            .position(|g| {
                let [gl, gc, gh] = g.lch;
                let neutral = chroma < 0.012 && gc < 0.012;
                let related = chroma >= 0.012
                    && gc >= 0.012
                    && hue_distance(hue, gh) < 16.0
                    && (chroma - gc).abs() < 0.12;
                (neutral || related) && (light - gl).abs() < 0.36
            })
            .unwrap_or_else(|| {
                groups.push(Group {
                    lch: [light, chroma, hue],
                    ..Default::default()
                });
                groups.len() - 1
            });
        let group = &mut groups[index];
        group.weight += weight;
        let pigment = weight * (0.25 + 0.75 * (light / 0.65).min(1.0));
        group.pigment += pigment;
        for (k, v) in [light, a, b].into_iter().enumerate() {
            group.sum[k] += v * pigment;
        }
        for (tile, w) in &bin.tiles {
            *group.tiles.entry(*tile).or_insert(0.0) += w;
        }
        let [gl, ga, gb] = group.sum.map(|v| v / group.pigment);
        group.lch = [
            gl,
            ga.hypot(gb),
            gb.atan2(ga).to_degrees().rem_euclid(360.0),
        ];
    }
    let mut candidates = Vec::new();
    for group in groups {
        let [light, chroma, hue] = group.lch;
        let coverage = group.weight / total;
        if coverage < 0.004 || chroma < 0.012 || !(0.12..=0.96).contains(&light) {
            continue;
        }
        let spread = group
            .tiles
            .values()
            .filter(|w| **w >= total * 0.001)
            .count() as f64
            / 16.0;
        let mut score = coverage.powf(0.60) * (chroma / 0.12).min(1.5).powf(0.8);
        score *= (0.50 + 0.50 * (light / 0.55).min(1.0)) * (0.80 + 0.20 * spread);
        candidates.push(json!({"hex":color(light,chroma,hue),"coverage":rounded(coverage,5),"chroma":rounded(chroma,5),"score":rounded(score,6),"spread":rounded(spread,4),"salience":rounded(0.80+0.20*spread,4),"lightness":rounded(light,5),"regions":{"columns":4,"rows":4,"cells":group.tiles.iter().map(|((x,y),w)|json!({"index":y*4+x,"coverage":rounded(w/total,6)})).collect::<Vec<_>>()}}));
    }
    candidates.sort_by(|a, b| {
        num(b, "score")
            .total_cmp(&num(a, "score"))
            .then(str_at(a, "hex").cmp(str_at(b, "hex")))
    });
    let mut distinct = Vec::new();
    for separation in [48.0, 24.0] {
        for c in &candidates {
            let hue = lch(str_at(c, "hex"))?[2];
            if distinct
                .iter()
                .all(|v| hue_distance(hue, lch(str_at(v, "hex")).unwrap()[2]) >= separation)
            {
                distinct.push(c.clone());
            }
            if distinct.len() == 5 {
                break;
            }
        }
        if distinct.len() == 5 {
            break;
        }
    }
    if distinct.is_empty() {
        let (a, b) = (mean_a / observed, mean_b / observed);
        distinct.push(json!({"hex":color(0.55,a.hypot(b).min(0.025),b.atan2(a).to_degrees()),"coverage":1.0,"chroma":0.0,"score":0.0,"spread":1.0}));
    }
    let neutral = num(&distinct[0], "score") == 0.0;
    Ok(
        json!({"seed":distinct[0]["hex"],"candidates":distinct,"meanLightness":rounded(mean_light/observed,5),"neutral":neutral,"reason":if neutral{"neutral image fallback"}else{"coverage, chroma, usable lightness and spatial spread"}}),
    )
}
