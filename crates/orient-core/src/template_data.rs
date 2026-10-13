//! Export-only namespaces: Orient shades rather than Matugen HCT generation.
use crate::colour::{color, lch};
use crate::{text, Colors, Result};
use serde_json::{json, Value};
pub const BASE16_ROLES: [&str; 16] = [
    "surface",
    "surfaceContainerLow",
    "surfaceContainerHighest",
    "onSurfaceVariant",
    "onSurfaceVariant",
    "onSurface",
    "onSurface",
    "onSurface",
    "red",
    "peach",
    "yellow",
    "green",
    "teal",
    "blue",
    "mauve",
    "maroon",
];
pub fn base16_contexts(contexts: &Colors, active_mode: &str) -> Result<Value> {
    let mut result: Colors = (0..16)
        .map(|i| (format!("base{i:02x}"), json!({})))
        .collect();
    for (mode, values) in contexts {
        let values = values.as_object().ok_or("Invalid template context")?;
        for (index, role) in BASE16_ROLES.iter().enumerate() {
            result[&format!("base{index:02x}")][mode] = json!(text(values, role)?);
        }
        let [dim, chroma, hue] = lch(text(values, "onSurfaceVariant")?)?;
        let foreground = lch(text(values, "onSurface")?)?[0];
        result["base04"][mode] = json!(color((dim + foreground) / 2.0, chroma, hue));
        let edge = if (if mode == "default" { active_mode } else { mode }) == "light" {
            0.0
        } else {
            1.0
        };
        result["base06"][mode] = json!(color(foreground + (edge - foreground) / 3.0, chroma, hue));
        result["base07"][mode] = json!(color(
            foreground + (edge - foreground) * 2.0 / 3.0,
            chroma,
            hue
        ));
    }
    Ok(Value::Object(result))
}
pub fn tonal_contexts(values: &Colors) -> Result<Value> {
    let mut result = Colors::new();
    for family in [
        "primary",
        "secondary",
        "tertiary",
        "neutral",
        "neutral_variant",
        "error",
    ] {
        let role = if family == "neutral_variant" {
            "neutralVariant"
        } else {
            family
        };
        let [_, chroma, hue] = lch(text(values, &format!("{role}PaletteKeyColor"))?)?;
        let mapping: Colors = (0..101)
            .map(|tone| {
                (
                    format!("_{tone}"),
                    json!(color(tone as f64 / 100.0, chroma, hue)),
                )
            })
            .collect();
        result.insert(family.into(), Value::Object(mapping));
    }
    Ok(Value::Object(result))
}
