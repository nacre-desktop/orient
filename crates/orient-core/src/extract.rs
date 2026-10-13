//! Bounded clustering of already normalized RGBA pixels; no decoding or I/O.
use crate::colour::*;
use crate::{num, rounded, str_at, sum, Result};
use indexmap::IndexMap;
use serde_json::{json, Value};
#[derive(Clone, Debug)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}
impl Frame {
    pub fn validate(&self) -> Result<()> {
        if self.width == 0
            || self.height == 0
            || self.width > 128
            || self.height > 128
            || self
                .width
                .checked_mul(self.height)
                .and_then(|v| v.checked_mul(4))
                != Some(self.rgba.len())
        {
            Err("Invalid normalized RGBA frame".into())
        } else {
            Ok(())
        }
    }
    pub fn pixels(&self) -> impl Iterator<Item = &[u8]> {
        self.rgba.chunks_exact(4)
    }
}
pub fn sample_color(frame: &Frame, x: f64, y: f64) -> Result<String> {
    if !x.is_finite() || !y.is_finite() || !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
        return Err("Pick coordinates between zero and one".into());
    }
    frame.validate()?;
    let i = ((y * frame.height as f64) as usize).min(frame.height - 1) * frame.width
        + ((x * frame.width as f64) as usize).min(frame.width - 1);
    let p = &frame.rgba[i * 4..i * 4 + 4];
    if p[3] == 0 {
        return Err("The selected point is transparent".into());
    }
    Ok(format!("{:02x}{:02x}{:02x}", p[0], p[1], p[2]))
}
pub fn distance(a: [f64; 3], b: [f64; 3], light_weight: f64) -> f64 {
    ((a[0] - b[0]).powf(2.0) * light_weight + (a[1] - b[1]).powf(2.0) + (a[2] - b[2]).powf(2.0))
        .sqrt()
}
#[derive(Default)]
struct Bin {
    weight: f64,
    rgb: [f64; 3],
    salience: f64,
    cells: IndexMap<usize, f64>,
    xy: [f64; 2],
}
#[derive(Default)]
struct Group {
    weight: f64,
    sum: [f64; 3],
    salience: f64,
    cells: IndexMap<usize, f64>,
    xy: [f64; 2],
    lab: [f64; 3],
}
fn direction(lab: [f64; 3]) -> [f64; 3] {
    let strength = lab[1].hypot(lab[2]);
    [0.0, lab[1] * 0.1 / strength, lab[2] * 0.1 / strength]
}
fn compatible(a: [f64; 3], b: [f64; 3]) -> bool {
    let chromatic = a[1].hypot(a[2]) >= 0.012;
    let other = b[1].hypot(b[2]) >= 0.012;
    chromatic == other && (!chromatic || distance(direction(a), direction(b), 1.0) < 0.028)
}
fn cells(cells: &IndexMap<usize, f64>, divisor: f64) -> Vec<Value> {
    let mut ordered: Vec<_> = cells.iter().collect();
    ordered.sort_by(|a, b| b.1.total_cmp(a.1));
    ordered
        .into_iter()
        .map(|(i, w)| json!({"index":i,"coverage":rounded(w/divisor,6)}))
        .collect()
}
fn rank(records: &mut [Value], key: &str) {
    records.sort_by(|a, b| {
        num(b, key)
            .total_cmp(&num(a, key))
            .then(str_at(a, "hex").cmp(str_at(b, "hex")))
    })
}
pub fn pigment_families(records: &[Value]) -> Result<Vec<Value>> {
    let mut ordered = records.to_vec();
    rank(&mut ordered, "coverage");
    let mut groups: Vec<(Vec<Value>, Vec<[f64; 3]>)> = Vec::new();
    for record in ordered {
        if num(&record, "chroma") < 0.012
            || !(0.12 < num(&record, "lightness") && num(&record, "lightness") < 0.96)
        {
            continue;
        }
        let [_, _, h] = lch(str_at(&record, "hex"))?;
        let angle = h.to_radians();
        let vector = [0.0, 0.1 * angle.cos(), 0.1 * angle.sin()];
        let index = groups
            .iter()
            .position(|(_, vectors)| vectors.iter().all(|v| distance(vector, *v, 1.0) < 0.028))
            .unwrap_or_else(|| {
                groups.push((Vec::new(), Vec::new()));
                groups.len() - 1
            });
        groups[index].0.push(record);
        groups[index].1.push(vector);
    }
    let mut families = Vec::new();
    for (members, _) in groups {
        let coverage = sum(members.iter().map(|r| num(r, "coverage")));
        if coverage < 0.0025 {
            continue;
        }
        let pigment: Vec<_> = members
            .iter()
            .map(|r| num(r, "coverage") * (0.25 + 0.75 * (num(r, "lightness") / 0.65).min(1.0)))
            .collect();
        let mass = sum(pigment.iter().copied());
        let labs: Vec<_> = members
            .iter()
            .map(|r| rgb(str_at(r, "hex")).map(lab_from_rgb))
            .collect::<Result<_>>()?;
        let [light, a, b] = std::array::from_fn(|k| {
            sum(labs.iter().zip(&pigment).map(|(lab, w)| lab[k] * w)) / mass
        });
        let chroma = a.hypot(b);
        let salience = sum(members
            .iter()
            .map(|r| num(r, "salience") * num(r, "coverage")))
            / coverage;
        let mut bins = IndexMap::new();
        for r in &members {
            for cell in r["regions"]["cells"].as_array().unwrap() {
                *bins
                    .entry(cell["index"].as_u64().unwrap() as usize)
                    .or_insert(0.0) += num(cell, "coverage");
            }
        }
        let score = coverage.powf(0.7)
            * (chroma / 0.12).min(1.5).powf(0.8)
            * (0.5 + 0.5 * (light / 0.55).min(1.0))
            * salience;
        families.push(json!({"hex":color(light,chroma,b.atan2(a).to_degrees()),"coverage":rounded(coverage,6),"chroma":rounded(chroma,6),"lightness":rounded(light,6),"salience":rounded(salience,4),"score":rounded(score,7),"spread":rounded(bins.len() as f64/96.0,4),"location":{"x":rounded(sum(members.iter().map(|r|num(&r["location"],"x")*num(r,"coverage")))/coverage,4),"y":rounded(sum(members.iter().map(|r|num(&r["location"],"y")*num(r,"coverage")))/coverage,4)},"regions":{"columns":12,"rows":8,"cells":cells(&bins,1.0)},"members":members.iter().map(|r|r["hex"].clone()).collect::<Vec<_>>()}));
    }
    rank(&mut families, "score");
    Ok(families)
}
pub fn analyze(images: &[Frame]) -> Result<Value> {
    if images.is_empty() {
        return Err("Invalid normalized RGBA frame".into());
    }
    let mut bins: IndexMap<[u8; 3], Bin> = IndexMap::new();
    let mut total = 0.0;
    for image in images {
        image.validate()?;
        let pixels: Vec<_> = image.pixels().collect();
        for (i, p) in pixels.iter().enumerate() {
            let (r, g, b, alpha) = (p[0], p[1], p[2], p[3]);
            if alpha == 0 {
                continue;
            }
            let weight = alpha as f64 / 255.0 / images.len() as f64;
            let (x, y) = (i % image.width, i / image.width);
            let (nx, ny) = (
                (x as f64 + 0.5) / image.width as f64,
                (y as f64 + 0.5) / image.height as f64,
            );
            let center = (-(((nx - 0.5) / 0.42).powf(2.0) + ((ny - 0.5) / 0.42).powf(2.0))).exp();
            let mut detail = 0.0;
            for j in [
                if x + 1 < image.width { i + 1 } else { i },
                if y + 1 < image.height {
                    i + image.width
                } else {
                    i
                },
            ] {
                detail += sum([r, g, b]
                    .into_iter()
                    .enumerate()
                    .map(|(k, v)| (pixels[j][k] as f64 - v as f64).abs()))
                    / 1530.0;
            }
            let salience = 1.0 + 0.28 * center + 0.45_f64.min(detail * 2.5);
            let record = bins.entry([r >> 3, g >> 3, b >> 3]).or_default();
            record.weight += weight;
            record.salience += salience * weight;
            for (k, v) in [r, g, b].into_iter().enumerate() {
                record.rgb[k] += v as f64 * weight;
            }
            record.xy[0] += nx * weight;
            record.xy[1] += ny * weight;
            let cell = ((nx * 12.0) as usize).min(11) + ((ny * 8.0) as usize).min(7) * 12;
            *record.cells.entry(cell).or_insert(0.0) += weight;
            total += weight;
        }
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
    for record in ordered {
        let weight = record.weight;
        let lab = lab_from_rgb(record.rgb.map(|v| v / weight / 255.0));
        let best = groups
            .iter()
            .enumerate()
            .filter(|(_, g)| compatible(lab, g.lab))
            .map(|(i, g)| (distance(lab, g.lab, 0.45), i))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let limit = (lab[1].hypot(lab[2]) * 0.4).clamp(0.012, 0.045);
        let index = if let Some((d, i)) = best.filter(|(d, _)| *d < limit) {
            let _ = d;
            i
        } else {
            groups.push(Group {
                lab,
                ..Default::default()
            });
            groups.len() - 1
        };
        let group = &mut groups[index];
        group.weight += weight;
        group.salience += record.salience;
        for (i, w) in &record.cells {
            *group.cells.entry(*i).or_insert(0.0) += w;
        }
        for (k, v) in lab.into_iter().enumerate() {
            group.sum[k] += v * weight;
        }
        for k in 0..2 {
            group.xy[k] += record.xy[k];
        }
        group.lab = group.sum.map(|v| v / group.weight);
    }
    let mut records = Vec::new();
    for group in groups {
        let [light, a, b] = group.lab;
        let chroma = a.hypot(b);
        let coverage = group.weight / total;
        let salience = group.salience / group.weight;
        let score = coverage.powf(0.7)
            * (chroma / 0.12).min(1.5).powf(0.8)
            * (0.5 + 0.5 * (light / 0.55).min(1.0))
            * salience;
        records.push(json!({"hex":color(light,chroma,b.atan2(a).to_degrees().rem_euclid(360.0)),"coverage":rounded(coverage,6),"chroma":rounded(chroma,6),"score":rounded(score,7),"salience":rounded(salience,4),"lightness":rounded(light,6),"spread":rounded(group.cells.len() as f64/96.0,4),"location":{"x":rounded(group.xy[0]/group.weight,4),"y":rounded(group.xy[1]/group.weight,4)},"regions":{"columns":12,"rows":8,"cells":cells(&group.cells,total)}}));
    }
    rank(&mut records, "score");
    let families = pigment_families(&records)?;
    let mut candidates = Vec::new();
    for separation in [48.0, 18.0] {
        for record in &families {
            let hue = lch(str_at(record, "hex"))?[2];
            if candidates
                .iter()
                .all(|c| hue_distance(hue, lch(str_at(c, "hex")).unwrap()[2]) >= separation)
            {
                candidates.push(record.clone());
            }
            if candidates.len() == 5 {
                break;
            }
        }
        if candidates.len() == 5 {
            break;
        }
    }
    let neutral = candidates.is_empty();
    if neutral {
        let mut dominant = records
            .iter()
            .enumerate()
            .max_by(|a, b| {
                num(a.1, "coverage")
                    .total_cmp(&num(b.1, "coverage"))
                    .then(b.0.cmp(&a.0))
            })
            .unwrap()
            .1
            .clone();
        dominant["chroma"] = json!(0.0);
        dominant["score"] = json!(0.0);
        candidates.push(dominant);
    }
    let mut ordered = records.clone();
    ordered.sort_by(|a, b| num(a, "lightness").total_cmp(&num(b, "lightness")));
    let tone = |low: bool| {
        let walk: Vec<_> = if low {
            ordered.iter().collect()
        } else {
            ordered.iter().rev().collect()
        };
        let mut mass = 0.0;
        let mut selected = Vec::new();
        for item in walk {
            selected.push(item);
            mass += num(item, "coverage");
            if mass >= 0.25 {
                break;
            }
        }
        selected
            .into_iter()
            .enumerate()
            .max_by(|a, b| {
                num(a.1, "coverage")
                    .total_cmp(&num(b.1, "coverage"))
                    .then(b.0.cmp(&a.0))
            })
            .unwrap()
            .1
            .clone()
    };
    let body = json!({"dark":tone(true),"light":tone(false)});
    let mean = rounded(
        sum(records
            .iter()
            .map(|r| num(r, "lightness") * num(r, "coverage"))),
        6,
    );
    records.sort_by(|a, b| num(b, "coverage").total_cmp(&num(a, "coverage")));
    records.truncate(64);
    Ok(
        json!({"seed":candidates[0]["hex"],"candidates":candidates,"families":families,"clusters":records,"body":body,"meanLightness":mean,"neutral":neutral,"sample":{"width":images[0].width,"height":images[0].height,"frames":images.len()},"reason":"perceptual shade clusters; aggregated pigment-family coverage with bounded salience"}),
    )
}
