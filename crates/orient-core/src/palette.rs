//! Faithful readable role policies.
use crate::accessibility::{difference, text_pairs, title, VISIONS};
use crate::colour::*;
use crate::{text, Colors, Result};
use serde_json::{json, Value};
pub const DEFAULT_SEED: &str = "47a99a";
pub const VARIANTS: [&str; 9] = [
    "tonalspot",
    "vibrant",
    "expressive",
    "fidelity",
    "fruitsalad",
    "monochrome",
    "neutral",
    "rainbow",
    "content",
];
pub const SEMANTIC_SEEDS: [(&str, &str); 14] = [
    ("rosewater", "d8ada7"),
    ("flamingo", "dd8d81"),
    ("pink", "d965aa"),
    ("mauve", "a86fd0"),
    ("red", "df303e"),
    ("maroon", "b64b60"),
    ("peach", "e68d50"),
    ("yellow", "d4b14b"),
    ("green", "289563"),
    ("teal", "309f95"),
    ("sky", "55accc"),
    ("sapphire", "398ab7"),
    ("blue", "477fe0"),
    ("lavender", "8f84d5"),
];
pub const PERSONALITIES: [&str; 8] = [
    "natural", "pigment", "harmony", "pop", "mist", "vivid", "pearl", "tide",
];
pub const REQUIRED_ROLES: [&str; 120] = [
    "background",
    "base",
    "blue",
    "crust",
    "error",
    "errorContainer",
    "errorDim",
    "errorPaletteKeyColor",
    "flamingo",
    "green",
    "inverseOnSurface",
    "inversePrimary",
    "inverseSurface",
    "klink",
    "klinkSelection",
    "knegative",
    "knegativeSelection",
    "kneutral",
    "kneutralSelection",
    "kpositive",
    "kpositiveSelection",
    "kvisited",
    "kvisitedSelection",
    "lavender",
    "mantle",
    "maroon",
    "mauve",
    "neutralPaletteKeyColor",
    "neutralVariantPaletteKeyColor",
    "neutral_paletteKeyColor",
    "neutral_variant_paletteKeyColor",
    "onBackground",
    "onError",
    "onErrorContainer",
    "onPrimary",
    "onPrimaryContainer",
    "onPrimaryFixed",
    "onPrimaryFixedVariant",
    "onSecondary",
    "onSecondaryContainer",
    "onSecondaryFixed",
    "onSecondaryFixedVariant",
    "onSuccess",
    "onSuccessContainer",
    "onSurface",
    "onSurfaceVariant",
    "onTertiary",
    "onTertiaryContainer",
    "onTertiaryFixed",
    "onTertiaryFixedVariant",
    "outline",
    "outlineVariant",
    "overlay0",
    "overlay1",
    "overlay2",
    "peach",
    "pink",
    "primary",
    "primaryContainer",
    "primaryDim",
    "primaryFixed",
    "primaryFixedDim",
    "primaryPaletteKeyColor",
    "primary_paletteKeyColor",
    "red",
    "rosewater",
    "sapphire",
    "scrim",
    "secondary",
    "secondaryContainer",
    "secondaryDim",
    "secondaryFixed",
    "secondaryFixedDim",
    "secondaryPaletteKeyColor",
    "secondary_paletteKeyColor",
    "shadow",
    "sky",
    "subtext0",
    "subtext1",
    "success",
    "successContainer",
    "surface",
    "surface0",
    "surface1",
    "surface2",
    "surfaceBright",
    "surfaceContainer",
    "surfaceContainerHigh",
    "surfaceContainerHighest",
    "surfaceContainerLow",
    "surfaceContainerLowest",
    "surfaceDim",
    "surfaceTint",
    "surfaceVariant",
    "teal",
    "term0",
    "term1",
    "term10",
    "term11",
    "term12",
    "term13",
    "term14",
    "term15",
    "term2",
    "term3",
    "term4",
    "term5",
    "term6",
    "term7",
    "term8",
    "term9",
    "tertiary",
    "tertiaryContainer",
    "tertiaryDim",
    "tertiaryFixed",
    "tertiaryFixedDim",
    "tertiaryPaletteKeyColor",
    "tertiary_paletteKeyColor",
    "text",
    "yellow",
];
fn put(colors: &mut Colors, key: impl Into<String>, value: impl Into<String>) {
    colors.insert(key.into(), Value::String(value.into()));
}
fn alias(colors: &mut Colors, key: &str, source: &str) -> Result<()> {
    let value = text(colors, source)?.to_owned();
    put(colors, key, value);
    Ok(())
}
fn backgrounds(colors: &Colors, keys: &[&str]) -> Result<Vec<String>> {
    keys.iter()
        .map(|k| text(colors, k).map(str::to_owned))
        .collect()
}
pub fn background_policy(personality: &str, background: &Value) -> Result<(String, bool)> {
    let p = if personality == "source" {
        "natural"
    } else {
        personality
    };
    if !PERSONALITIES.contains(&p) {
        return Err("Unknown palette personality".into());
    }
    let background = if background.is_null() {
        if personality == "source" {
            true
        } else {
            !["natural", "pigment"].contains(&p)
        }
    } else {
        background
            .as_bool()
            .ok_or("Background from wallpaper must be a boolean")?
    };
    Ok((p.into(), background))
}
pub fn supporting_sources(seed: &str, candidates: &[Value], harmony: bool) -> Result<Vec<String>> {
    let seed = clean(seed)?;
    let records: Vec<Value> = candidates
        .iter()
        .map(|c| {
            if c.is_object() {
                Ok(c.clone())
            } else {
                Ok(json!({"hex":clean(c.as_str().ok_or("Expected a six-digit RGB color")?)?}))
            }
        })
        .collect::<Result<_>>()?;
    let records: Vec<_> = records
        .into_iter()
        .map(|c| {
            let value = clean(crate::str_at(&c, "hex"))?;
            Ok((value != seed).then_some(c))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    let mut chosen: Vec<String> = Vec::new();
    if !harmony {
        for c in records {
            let value = clean(crate::str_at(&c, "hex"))?;
            let hue = lch(&value)?[2];
            if std::iter::once(&seed)
                .chain(chosen.iter())
                .all(|v| hue_distance(hue, lch(v).unwrap()[2]) >= 18.0)
            {
                chosen.push(value);
            }
            if chosen.len() == 2 {
                break;
            }
        }
        return Ok(chosen);
    }
    let main = lch(&seed)?;
    let dominant = candidates
        .iter()
        .filter(|v| v.is_object())
        .map(|v| crate::num(v, "coverage"))
        .fold(0.0, f64::max);
    let mut ranked = Vec::new();
    for c in records {
        let coverage = c.get("coverage").and_then(Value::as_f64);
        if coverage.is_some_and(|v| v < 0.04_f64.max(dominant * 0.05)) {
            continue;
        }
        let value = clean(crate::str_at(&c, "hex"))?;
        let [light, chroma, hue] = lch(&value)?;
        let distance = hue_distance(main[2], hue);
        if distance > 100.0 && coverage.is_some_and(|v| v < 0.12) {
            continue;
        }
        let mut related: f64 = if distance <= 80.0 { 1.0 } else { 0.55 };
        if coverage.is_some_and(|v| v >= 0.15) {
            related = related.max(0.90)
        }
        let olive =
            (85.0..=145.0).contains(&hue) && light < 0.62 && (0.012..0.12).contains(&chroma);
        let weight = c
            .get("score")
            .and_then(Value::as_f64)
            .unwrap_or(coverage.filter(|v| *v != 0.0).unwrap_or(1.0))
            * related
            * if olive { 0.28 } else { 1.0 };
        ranked.push((weight, value));
    }
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, value) in ranked {
        let hue = lch(&value)?[2];
        if chosen
            .iter()
            .all(|v| hue_distance(hue, lch(v).unwrap()[2]) >= 24.0)
        {
            chosen.push(value)
        }
        if chosen.len() == 2 {
            break;
        }
    }
    Ok(chosen)
}
struct Accent<'a> {
    brightness: f64,
    saturation: f64,
    personality: &'a str,
    variant: &'a str,
    harmony: bool,
    dark: bool,
    soft: bool,
    chroma: f64,
}
impl Accent<'_> {
    fn family(
        &self,
        colors: &mut Colors,
        backgrounds: &[String],
        role: &str,
        source: &str,
        strength: f64,
    ) -> Result<()> {
        let [light, mut c, h] = lch(source)?;
        let mut light = (light + self.brightness).clamp(0.0, 1.0);
        c = c
            * strength
            * self.saturation
            * if self.personality == "vivid" {
                1.35
            } else {
                1.0
            };
        let accent_role = ["primary", "secondary", "tertiary"].contains(&role);
        if self.variant == "monochrome" && accent_role {
            c = 0.0
        }
        if self.harmony
            && accent_role
            && (85.0..=145.0).contains(&h)
            && light < 0.62
            && (0.012..0.12).contains(&c)
        {
            light = 0.70
        }
        if role == "primary" {
            c = self.chroma
        }
        let mut lifted = if self.dark && !self.soft && c >= 0.012 {
            light.max(0.60)
        } else {
            light
        };
        let mut lifted_chroma = c * (lifted / light.max(0.12)).min(1.8);
        if self.dark && !self.soft && self.variant != "monochrome" && c >= 0.012 {
            lifted_chroma = lifted_chroma.max(0.065_f64.min(c * 3.0));
        }
        if !self.soft && self.variant != "vibrant" {
            lifted_chroma = lifted_chroma.min(if role == "primary" { 0.19 } else { 0.16 });
        }
        if !self.dark && accent_role {
            lifted = light.clamp(0.52, 0.58);
            lifted_chroma = c;
            if light >= 0.65 && (45.0..=110.0).contains(&h) && (0.012..=0.12).contains(&c) {
                lifted = 0.62;
                lifted_chroma = 0.14_f64.min(c * 1.28 + 0.01);
            }
            if c < 0.012 || self.variant == "monochrome" {
                lifted = match role {
                    "primary" => 0.50,
                    "secondary" => 0.41,
                    _ => 0.35,
                }
            }
        }
        let accent = readable(&color(lifted, lifted_chroma, h), backgrounds, 4.5)?;
        let container_chroma = if self.dark && !self.soft {
            (c * 0.65).max(0.045_f64.min(c * 1.7)).min(0.095)
        } else {
            (c * 0.42).min(0.085)
        };
        let container = color(if self.dark { 0.35 } else { 0.88 }, container_chroma, h);
        put(colors, role, &accent);
        put(colors, format!("on{}", title(role)), foreground(&accent)?);
        put(
            colors,
            format!("{role}Dim"),
            readable(
                &color(
                    lch(&accent)?[0] + if self.dark { -0.025 } else { 0.025 },
                    c,
                    h,
                ),
                backgrounds,
                4.5,
            )?,
        );
        put(colors, format!("{role}Container"), &container);
        put(
            colors,
            format!("on{}Container", title(role)),
            readable(
                &color(if self.dark { 0.93 } else { 0.22 }, (c * 0.3).min(0.05), h),
                &[container],
                4.5,
            )?,
        );
        if accent_role {
            let fixed = color(0.87, c.min(0.14), h);
            let fixed_dim = color(0.78, c.min(0.14), h);
            put(colors, format!("{role}Fixed"), &fixed);
            put(colors, format!("{role}FixedDim"), &fixed_dim);
            let bgs = [fixed, fixed_dim];
            put(
                colors,
                format!("on{}Fixed", title(role)),
                readable(&color(0.20, c.min(0.07), h), &bgs, 4.5)?,
            );
            put(
                colors,
                format!("on{}FixedVariant", title(role)),
                readable(&color(0.35, c.min(0.07), h), &bgs, 4.5)?,
            );
            let key = color(light, c, h);
            put(colors, format!("{role}PaletteKeyColor"), &key);
            put(colors, format!("{role}_paletteKeyColor"), &key);
        }
        Ok(())
    }
}
pub fn validate(colors: &Colors) -> Result<()> {
    for (fg, bg) in text_pairs() {
        if contrast(text(colors, &fg)?, text(colors, &bg)?)? < 4.5 {
            return Err(format!("Unreadable generated roles: {fg}/{bg}").into());
        }
    }
    if !REQUIRED_ROLES.iter().all(|k| colors.contains_key(*k)) {
        return Err("Incomplete palette roles".into());
    }
    for value in colors.values() {
        clean(value.as_str().ok_or("Expected a six-digit RGB color")?)?;
    }
    Ok(())
}
fn semantic_roles(colors: &mut Colors, hue: f64, bgs: &[String]) -> Result<()> {
    for (role, value) in SEMANTIC_SEEDS {
        let [light, c, h] = lch(value)?;
        put(
            colors,
            role,
            readable(
                &color(light, c, h + 4.0 * (hue - h).to_radians().sin()),
                bgs,
                4.5,
            )?,
        )
    }
    for (role, value) in [
        ("text", "onSurface"),
        ("subtext1", "onSurfaceVariant"),
        ("subtext0", "onSurfaceVariant"),
        ("overlay2", "outline"),
        ("overlay1", "outline"),
        ("overlay0", "outlineVariant"),
        ("surface2", "surfaceContainerHighest"),
        ("surface1", "surfaceContainerHigh"),
        ("surface0", "surfaceContainer"),
        ("base", "surface"),
        ("mantle", "surfaceContainerLowest"),
        ("crust", "surfaceContainerLowest"),
        ("klink", "blue"),
        ("kvisited", "mauve"),
        ("knegative", "error"),
        ("kneutral", "yellow"),
        ("kpositive", "success"),
    ] {
        alias(colors, role, value)?;
    }
    for role in ["klink", "kvisited", "knegative", "kneutral", "kpositive"] {
        let value = readable(text(colors, role)?, &[text(colors, "primary")?.into()], 4.5)?;
        put(colors, format!("{role}Selection"), value);
    }
    for (i, role) in [
        "surface",
        "red",
        "green",
        "yellow",
        "blue",
        "mauve",
        "teal",
        "onSurface",
    ]
    .iter()
    .enumerate()
    {
        alias(colors, &format!("term{i}"), role)?;
        alias(
            colors,
            &format!("term{}", i + 8),
            if i == 0 { "onSurfaceVariant" } else { role },
        )?;
    }
    Ok(())
}
pub fn generate(seed: &str, options: &Value) -> Result<Colors> {
    let harmony = options["harmony"].as_bool().unwrap_or(false);
    let p = options["personality"]
        .as_str()
        .filter(|v| !v.is_empty())
        .unwrap_or(if harmony { "harmony" } else { "natural" });
    let (personality, wallpaper) = background_policy(p, &options["background_from_wallpaper"])?;
    let p = personality.as_str();
    let brightness = options["brightness"].as_f64().unwrap_or(0.0);
    let saturation = options["saturation"].as_f64().unwrap_or(1.0);
    let hour = options["hour"].as_f64().unwrap_or(12.0);
    if !(-0.15..=0.15).contains(&brightness)
        || !(0.0..=1.6).contains(&saturation)
        || !(0.0..=23.0).contains(&hour)
    {
        return Err("Color adjustment is outside its safe bounds".into());
    }
    let empty = Colors::new();
    let overrides = options["overrides"].as_object().unwrap_or(&empty);
    if overrides
        .keys()
        .any(|k| !["primary", "secondary", "tertiary"].contains(&k.as_str()))
    {
        return Err("Unknown accent role".into());
    }
    let seed = clean(
        overrides
            .get("primary")
            .and_then(Value::as_str)
            .unwrap_or(seed),
    )?;
    let seed = if p == "pearl" {
        clean(
            overrides
                .get("primary")
                .and_then(Value::as_str)
                .unwrap_or("d99fb7"),
        )?
    } else {
        seed
    };
    let variant = if p == "mist" {
        "neutral"
    } else if p == "vivid" {
        "vibrant"
    } else {
        options["variant"].as_str().unwrap_or("tonalspot")
    };
    let mode = options["mode"].as_str().unwrap_or("dark");
    let flavour = options["flavour"].as_str().unwrap_or("default");
    if !["light", "dark"].contains(&mode)
        || !VARIANTS.contains(&variant)
        || !["default", "hard"].contains(&flavour)
    {
        return Err("Unsupported palette mode, variant or flavour".into());
    }
    let body = if wallpaper && options["body"].is_object() {
        Some(&options["body"])
    } else {
        None
    };
    let dark = mode == "dark";
    let [_, mut chroma, hue] = lch(&seed)?;
    chroma *= saturation * if p == "vivid" { 1.35 } else { 1.0 };
    if p == "mist" {
        chroma = chroma.min(0.035)
    }
    let soft = ["neutral", "content"].contains(&variant);
    if variant == "monochrome" {
        chroma = 0.0
    } else if soft {
        chroma = (chroma * 0.55).min(0.10)
    } else if variant != "vibrant" {
        chroma = chroma.min(0.19)
    }
    let mut tint = (chroma * 0.18).min(0.028);
    if flavour == "hard" {
        tint *= 0.5
    }
    let levels = [
        ("surface", if dark { 0.18 } else { 0.975 }),
        ("surfaceDim", if dark { 0.18 } else { 0.87 }),
        ("surfaceBright", if dark { 0.27 } else { 0.985 }),
        ("surfaceContainerLowest", if dark { 0.15 } else { 0.995 }),
        ("surfaceContainerLow", if dark { 0.19 } else { 0.955 }),
        ("surfaceContainer", if dark { 0.215 } else { 0.935 }),
        ("surfaceContainerHigh", if dark { 0.24 } else { 0.91 }),
        ("surfaceContainerHighest", if dark { 0.255 } else { 0.885 }),
    ];
    let (mut body_hue, mut body_chroma) = (hue, tint);
    if let Some(body) = body.filter(|_| p != "pearl") {
        let [_, c, h] = lch(crate::str_at(
            &body[if dark { "dark" } else { "light" }],
            "hex",
        ))?;
        body_hue = h;
        body_chroma = c.min(if dark { 0.045 } else { 0.035 });
    }
    if ["mist", "pearl"].contains(&p) {
        body_chroma *= if p == "mist" { 0.18 } else { 0.0 }
    }
    if p == "tide" || (options["time_tint"].as_bool().unwrap_or(false) && p != "pearl") {
        body_hue = if !(5.0..17.0).contains(&hour) {
            55.0
        } else {
            250.0
        };
        body_chroma = body_chroma.clamp(0.012, 0.022);
    }
    let mut colors = Colors::new();
    for (role, light) in levels {
        put(&mut colors, role, color(light, body_chroma, body_hue));
    }
    let frame_chroma = 0.055_f64.min((chroma * 0.5).max(0.028_f64.min(chroma * 1.4)));
    put(
        &mut colors,
        "frame",
        color(
            if dark { 0.225 } else { 0.97 },
            if body.is_some() {
                body_chroma
            } else {
                frame_chroma
            },
            body_hue,
        ),
    );
    let bgs = backgrounds(
        &colors,
        &[
            "frame",
            "surface",
            "surfaceContainerHighest",
            if dark { "surfaceBright" } else { "surfaceDim" },
        ],
    )?;
    alias(&mut colors, "background", "surface")?;
    alias(&mut colors, "surfaceVariant", "surfaceContainerHighest")?;
    put(
        &mut colors,
        "onSurface",
        readable(&color(if dark { 0.94 } else { 0.20 }, tint, hue), &bgs, 4.5)?,
    );
    put(
        &mut colors,
        "onSurfaceVariant",
        readable(&color(if dark { 0.76 } else { 0.40 }, tint, hue), &bgs, 4.5)?,
    );
    put(&mut colors, "shadow", "000000");
    put(&mut colors, "scrim", "000000");
    alias(&mut colors, "onBackground", "onSurface")?;
    put(
        &mut colors,
        "outline",
        readable(&color(if dark { 0.58 } else { 0.54 }, tint, hue), &bgs, 3.0)?,
    );
    put(
        &mut colors,
        "outlineVariant",
        color(if dark { 0.37 } else { 0.79 }, tint, hue),
    );
    let (mut inverse_chroma, mut inverse_hue) = (tint, hue);
    if let Some(body) = body.filter(|_| p != "pearl") {
        let [_, c, h] = lch(crate::str_at(
            &body[if dark { "light" } else { "dark" }],
            "hex",
        ))?;
        inverse_chroma = c.min(0.035);
        inverse_hue = h;
    }
    put(
        &mut colors,
        "inverseSurface",
        color(if dark { 0.96 } else { 0.15 }, inverse_chroma, inverse_hue),
    );
    let inverse = readable(
        &color(if dark { 0.25 } else { 0.92 }, tint, hue),
        &[text(&colors, "inverseSurface")?.into()],
        4.5,
    )?;
    put(&mut colors, "inverseOnSurface", inverse);
    let companions = options["companions"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut sources = supporting_sources(&seed, companions, harmony)?;
    if p == "harmony" {
        let [l, c, h] = lch(&seed)?;
        sources = vec![
            color(l + 0.04, c * 0.8, h - 20.0),
            color(l + 0.08, c * 0.7, h + 24.0),
        ];
    } else if p == "pearl" {
        sources = vec!["aaa6d5".into(), "a5c5b4".into()];
    }
    let secondary = sources
        .first()
        .cloned()
        .unwrap_or_else(|| color(lch(&seed).unwrap()[0], chroma * 0.70, hue));
    let tertiary = sources
        .get(1)
        .cloned()
        .unwrap_or_else(|| color(lch(&seed).unwrap()[0], chroma * 0.45, hue));
    let policy = Accent {
        brightness,
        saturation,
        personality: p,
        variant,
        harmony,
        dark,
        soft,
        chroma,
    };
    policy.family(&mut colors, &bgs, "primary", &seed, 1.0)?;
    let secondary = clean(
        overrides
            .get("secondary")
            .and_then(Value::as_str)
            .unwrap_or(&secondary),
    )?;
    let tertiary = clean(
        overrides
            .get("tertiary")
            .and_then(Value::as_str)
            .unwrap_or(&tertiary),
    )?;
    policy.family(
        &mut colors,
        &bgs,
        "secondary",
        &secondary,
        if soft { 0.5 } else { 0.85 },
    )?;
    policy.family(
        &mut colors,
        &bgs,
        "tertiary",
        &tertiary,
        if soft { 0.5 } else { 0.85 },
    )?;
    policy.family(&mut colors, &bgs, "error", "df303e", 1.0)?;
    policy.family(&mut colors, &bgs, "success", "289563", 1.0)?;
    for (role, offset) in [("secondary", 0.09), ("tertiary", 0.16)] {
        let previous = if role == "tertiary" {
            vec![text(&colors, "primary")?, text(&colors, "secondary")?]
        } else {
            vec![text(&colors, "primary")?]
        };
        let mut minimum = f64::INFINITY;
        for v in previous {
            for vision in VISIONS {
                minimum = minimum.min(difference(text(&colors, role)?, v, vision)?);
            }
        }
        if dark && minimum < 0.045 && !overrides.contains_key(role) {
            let [_, c, h] = lch(text(&colors, role)?)?;
            let target = 0.94_f64.min(lch(text(&colors, "primary")?)?[0] + offset);
            let value = readable(&color(target, c, h), &bgs, 4.5)?;
            put(&mut colors, role, &value);
            put(
                &mut colors,
                format!("on{}", title(role)),
                foreground(&value)?,
            );
        }
    }
    alias(&mut colors, "surfaceTint", "primary")?;
    put(
        &mut colors,
        "overtone",
        if variant != "monochrome" {
            seed.clone()
        } else {
            color(lch(&seed)?[0], 0.0, hue)
        },
    );
    alias(&mut colors, "orient1", "overtone")?;
    put(
        &mut colors,
        "orient2",
        if variant == "monochrome" {
            color(lch(&secondary)?[0], 0.0, hue)
        } else {
            secondary
        },
    );
    put(
        &mut colors,
        "orient3",
        if variant == "monochrome" {
            color(lch(&tertiary)?[0], 0.0, hue)
        } else {
            tertiary
        },
    );
    let inverse = readable(
        text(&colors, "overtone")?,
        &[text(&colors, "inverseSurface")?.into()],
        4.5,
    )?;
    put(&mut colors, "inversePrimary", inverse);
    put(
        &mut colors,
        "neutralPaletteKeyColor",
        color(0.55, body_chroma, body_hue),
    );
    put(
        &mut colors,
        "neutralVariantPaletteKeyColor",
        color(0.55, (body_chroma * 2.0).min(0.03), body_hue),
    );
    alias(
        &mut colors,
        "neutral_paletteKeyColor",
        "neutralPaletteKeyColor",
    )?;
    alias(
        &mut colors,
        "neutral_variant_paletteKeyColor",
        "neutralVariantPaletteKeyColor",
    )?;
    put(&mut colors, "errorPaletteKeyColor", "df303e");
    semantic_roles(&mut colors, hue, &bgs)?;
    let mut all_backgrounds = backgrounds(&colors, &levels.map(|(role, _)| role))?;
    all_backgrounds.push(text(&colors, "frame")?.into());
    for role in [
        "onSurface",
        "onSurfaceVariant",
        "primary",
        "secondary",
        "tertiary",
        "error",
        "success",
        "primaryDim",
        "secondaryDim",
        "tertiaryDim",
        "errorDim",
        "successDim",
    ] {
        let value = readable(text(&colors, role)?, &all_backgrounds, 4.5)?;
        put(&mut colors, role, value);
    }
    for role in ["primary", "secondary", "tertiary", "error", "success"] {
        let value = foreground(text(&colors, role)?)?;
        put(&mut colors, format!("on{}", title(role)), value);
    }
    alias(&mut colors, "onBackground", "onSurface")?;
    alias(&mut colors, "text", "onSurface")?;
    alias(&mut colors, "subtext0", "onSurfaceVariant")?;
    alias(&mut colors, "subtext1", "onSurfaceVariant")?;
    alias(&mut colors, "surfaceTint", "primary")?;
    validate(&colors)?;
    Ok(colors)
}
