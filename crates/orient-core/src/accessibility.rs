//! Machado et al. (2009) full-severity matrices; approximate simulations.
use crate::colour::*;
use crate::{rounded, sum, text, Colors, Result};
use serde_json::{json, Value};
pub const VISIONS: [&str; 4] = ["normal", "protanopia", "deuteranopia", "tritanopia"];
pub fn simulate(value: &str, vision: &str) -> Result<String> {
    if vision == "normal" {
        return Ok(value.into());
    }
    let matrix = match vision {
        "protanopia" => [
            [0.152286, 1.052583, -0.204868],
            [0.114503, 0.786281, 0.099216],
            [-0.003882, -0.048116, 1.051998],
        ],
        "deuteranopia" => [
            [0.367322, 0.860646, -0.227968],
            [0.280085, 0.672501, 0.047413],
            [-0.011820, 0.042940, 0.968881],
        ],
        "tritanopia" => [
            [1.255528, -0.076749, -0.178779],
            [-0.078411, 0.930809, 0.147602],
            [0.004733, 0.691367, 0.303900],
        ],
        _ => return Err("Unknown vision simulation".into()),
    };
    let channels = rgb(value)?.map(linear);
    Ok(matrix
        .iter()
        .map(|row| {
            format!(
                "{:02x}",
                (encoded(sum(channels.into_iter().zip(row).map(|(v, w)| v * w)).clamp(0.0, 1.0))
                    * 255.0)
                    .round_ties_even() as u8
            )
        })
        .collect())
}
pub fn difference(a: &str, b: &str, vision: &str) -> Result<f64> {
    let x = lab_from_rgb(rgb(&simulate(a, vision)?)?);
    let y = lab_from_rgb(rgb(&simulate(b, vision)?)?);
    Ok(sum(x.into_iter().zip(y).map(|(u, v)| (u - v).powf(2.0))).powf(0.5))
}
pub const BACKGROUNDS: [&str; 9] = [
    "frame",
    "surface",
    "surfaceDim",
    "surfaceBright",
    "surfaceContainerLowest",
    "surfaceContainerLow",
    "surfaceContainer",
    "surfaceContainerHigh",
    "surfaceContainerHighest",
];
pub const TEXT_ROLES: [&str; 26] = [
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
    "red",
    "green",
    "blue",
    "yellow",
    "teal",
    "mauve",
    "pink",
    "peach",
    "lavender",
    "sky",
    "sapphire",
    "flamingo",
    "maroon",
    "rosewater",
];
pub fn title(value: &str) -> String {
    let mut c = value.chars();
    c.next()
        .map(|x| x.to_uppercase().to_string() + c.as_str())
        .unwrap_or_default()
}
pub fn text_pairs() -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for fg in TEXT_ROLES {
        for bg in BACKGROUNDS {
            pairs.push((fg.into(), bg.into()))
        }
    }
    for role in ["primary", "secondary", "tertiary", "error", "success"] {
        pairs.push((format!("on{}", title(role)), role.into()));
        pairs.push((
            format!("on{}Container", title(role)),
            format!("{role}Container"),
        ));
    }
    for role in ["primary", "secondary", "tertiary"] {
        for suffix in ["Fixed", "FixedVariant"] {
            for bg in ["Fixed", "FixedDim"] {
                pairs.push((format!("on{}{suffix}", title(role)), format!("{role}{bg}")))
            }
        }
    }
    pairs.extend([
        ("inverseOnSurface".into(), "inverseSurface".into()),
        ("inversePrimary".into(), "inverseSurface".into()),
    ]);
    pairs
}
pub fn audit(colors: &Colors) -> Result<Value> {
    let mut pairs = Vec::new();
    for (fg, bg) in text_pairs() {
        let ratio = contrast(text(colors, &fg)?, text(colors, &bg)?)?;
        pairs.push(
            json!({"foreground":fg,"background":bg,"ratio":rounded(ratio,3),"passes":ratio>=4.5}),
        );
    }
    let mut distinctions = Vec::new();
    let mut warnings = Vec::new();
    for vision in VISIONS {
        for (a, b) in [
            ("primary", "secondary"),
            ("primary", "tertiary"),
            ("secondary", "tertiary"),
        ] {
            let d = difference(text(colors, a)?, text(colors, b)?, vision)?;
            distinctions.push(
                json!({"vision":vision,"roles":[a,b],"distance":rounded(d,4),"distinct":d>=0.045}),
            );
            if d < 0.045 {
                warnings.push(format!(
                    "{a} / {b} are close under {vision}; keep labels or shapes"
                ));
            }
        }
    }
    let mut simulated = serde_json::Map::new();
    for vision in &VISIONS[1..] {
        let mut roles = serde_json::Map::new();
        for (role, value) in colors {
            let value = value.as_str().ok_or("Expected a six-digit RGB color")?;
            roles.insert(role.clone(), json!(simulate(value, vision)?));
        }
        simulated.insert((*vision).into(), Value::Object(roles));
    }
    Ok(
        json!({"textPasses":pairs.iter().all(|p|p["passes"]==true),"minimumTextContrast":pairs.iter().map(|p|p["ratio"].as_f64().unwrap()).fold(f64::INFINITY,f64::min),"text":pairs,"distinctions":distinctions,"warnings":warnings,"simulation":"Machado 2009, severity 1; approximate, not a guarantee","simulated":simulated}),
    )
}
