//! Pure palette assembly; callers supply analyzed pixels and source identity.
use crate::colour::{clean, hue_distance, lch};
use crate::{accessibility, palette};
use crate::{num, str_at, text, Colors, Error, Result};
use serde_json::{json, Value};
pub fn settings(options: &Value) -> Result<Value> {
    let p = crate::personality(options, options["harmony"] == true)?;
    let (p, background) = palette::background_policy(p, &options["background_from_wallpaper"])?;
    let mut overrides = Colors::new();
    if let Some(map) = options["overrides"].as_object() {
        for (role, value) in map {
            overrides.insert(
                role.clone(),
                json!(clean(
                    value.as_str().ok_or("Expected a six-digit RGB color")?
                )?),
            );
        }
    }
    let accent = if crate::truthy(&options["accent"]) {
        json!(clean(
            options["accent"]
                .as_str()
                .ok_or("Expected a six-digit RGB color")?
        )?)
    } else {
        Value::Null
    };
    Ok(
        json!({"mode":options.get("mode").cloned().unwrap_or(json!("dark")),"variant":options.get("variant").cloned().unwrap_or(json!("tonalspot")),"flavour":options.get("flavour").cloned().unwrap_or(json!("default")),"accent":accent,"smart":options["smart"].as_bool().unwrap_or(false),"harmony":p=="harmony","personality":p,"background_from_wallpaper":background,"overrides":overrides,"brightness":options.get("brightness").cloned().unwrap_or(json!(0.0)),"saturation":options.get("saturation").cloned().unwrap_or(json!(1.0)),"hour":if p=="tide"||options["time_tint"]==true{options.get("hour").cloned().unwrap_or(json!(12))}else{json!(12)},"time_tint":options["time_tint"].as_bool().unwrap_or(false)}),
    )
}
pub fn pop_seed(
    analysis: &Value,
    seed: &str,
    accent: bool,
    overrides: &Colors,
    personality: &str,
) -> Result<String> {
    if personality != "pop" || accent || overrides.contains_key("primary") {
        return Ok(seed.into());
    }
    let main_hue = lch(str_at(analysis, "seed"))?[2];
    let families = analysis["families"].as_array().ok_or("Invalid analysis")?;
    let main_coverage = families
        .iter()
        .find(|c| c["hex"] == analysis["seed"])
        .map(|c| num(c, "coverage"))
        .unwrap_or(1.0);
    let mut minorities = Vec::new();
    for c in families {
        let coverage = num(c, "coverage");
        let hue = lch(str_at(c, "hex"))?[2];
        if coverage >= 0.004
            && coverage <= 0.15_f64.min(main_coverage * 0.65)
            && num(c, "chroma") >= 0.025
            && (0.2..=0.9).contains(&num(c, "lightness"))
            && hue_distance(main_hue, hue) >= 48.0
        {
            let score = num(c, "salience")
                * (num(c, "chroma") / 0.12).min(1.5).powf(0.8)
                * coverage.powf(0.25)
                * (0.5 + 0.5 * (hue_distance(main_hue, hue) / 120.0).min(1.0));
            minorities.push((score, str_at(c, "hex")));
        }
    }
    Ok(minorities
        .into_iter()
        .max_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(b.1)))
        .map(|(_, v)| v)
        .unwrap_or(seed)
        .into())
}
pub fn palette_options(analysis: &Value, options: &Value) -> Result<Value> {
    let candidates = analysis["candidates"]
        .as_array()
        .ok_or("Invalid analysis")?;
    let mut result: Vec<Value> = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let seed = str_at(candidate, "hex");
        let mut opts = options.clone();
        opts["companions"] = analysis["candidates"].clone();
        opts["body"] = analysis["body"].clone();
        let colors = palette::generate(seed, &opts)?;
        let [_, chroma, hue] = lch(seed)?;
        let names = [
            (25.0, "Rose"),
            (55.0, "Copper"),
            (95.0, "Gold"),
            (130.0, "Olive"),
            (170.0, "Green"),
            (205.0, "Teal"),
            (245.0, "Blue"),
            (285.0, "Indigo"),
            (325.0, "Lilac"),
            (360.0, "Rose"),
        ];
        let mut name = if chroma < 0.012 {
            "Neutral"
        } else {
            names.iter().find(|(end, _)| hue < *end).unwrap().1
        }
        .to_string();
        if result.iter().any(|r| r["name"] == name) {
            name.push_str(" tone");
        }
        result.push(json!({"accent":seed,"name":name,"swatches":[colors["primary"],colors["secondary"],colors["tertiary"]],"surface":colors["frame"],"recommended":index==0,"coverage":candidate["coverage"],"location":candidate.get("location"),"regions":candidate.get("regions")}));
    }
    Ok(json!(result))
}
fn role_metadata(
    analysis: &Value,
    colors: &Colors,
    seed: &str,
    options: &Value,
) -> Result<(Value, Value)> {
    let mode = str_at(options, "mode");
    let personality = str_at(options, "personality");
    let overrides = options["overrides"].as_object().ok_or("Invalid settings")?;
    let accent = options["accent"].is_string();
    let background = options["background_from_wallpaper"] == true;
    let mut sources = Colors::new();
    for role in ["primary", "secondary", "tertiary"] {
        let actual = overrides
            .get(role)
            .and_then(Value::as_str)
            .unwrap_or(if role == "primary" {
                seed
            } else {
                text(
                    colors,
                    if role == "secondary" {
                        "orient2"
                    } else {
                        "orient3"
                    },
                )?
            });
        let actual = if personality == "pearl" && role == "primary" {
            text(colors, "overtone")?
        } else {
            actual
        };
        let found = ["candidates", "families", "clusters"]
            .iter()
            .flat_map(|k| analysis[*k].as_array().into_iter().flatten())
            .find(|c| c["hex"] == actual);
        sources.insert(role.into(),json!({"sourceColor":actual,"type":if personality=="pearl"{"signature"}else if overrides.contains_key(role)||(role=="primary"&&accent){"picked"}else if found.is_some(){"observed"}else{"derived"},"coverage":found.map(|c|num(c,"coverage")).unwrap_or(0.0),"location":found.and_then(|c|c.get("location")),"regions":found.and_then(|c|c.get("regions"))}));
    }
    let body_source = if background {
        str_at(
            &analysis["body"][if mode == "dark" { "dark" } else { "light" }],
            "hex",
        )
    } else {
        seed
    };
    let mut semantic: std::collections::BTreeMap<&str, &str> =
        palette::SEMANTIC_SEEDS.into_iter().collect();
    semantic.insert("error", "df303e");
    semantic.insert("success", "289563");
    let ansi = [
        "surface",
        "red",
        "green",
        "yellow",
        "blue",
        "mauve",
        "teal",
        "onSurface",
    ];
    let mut provenance = Colors::new();
    for (role, value) in colors {
        let mut parent = ["primary", "secondary", "tertiary"]
            .into_iter()
            .find(|name| role.to_lowercase().contains(name))
            .unwrap_or("body");
        let mut original = if sources.contains_key(parent) {
            Some(str_at(&sources[parent], "sourceColor"))
        } else {
            Some(body_source)
        };
        let mut kind = "derived";
        if let Some(value) = semantic.get(role.as_str()) {
            original = Some(value);
            kind = "semantic";
        } else if let Some(term) = role.strip_prefix("term") {
            let index = term
                .parse::<usize>()
                .map_err(|_| Error::Value("Invalid terminal role".into()))?
                % 8;
            parent = ansi[index];
            original = semantic.get(parent).copied();
            kind = if (1..=6).contains(&index) {
                "semantic"
            } else {
                "derived"
            };
        } else if role.starts_with("on")
            || [
                "text",
                "subtext0",
                "subtext1",
                "outline",
                "klinkSelection",
                "kvisitedSelection",
                "knegativeSelection",
                "kneutralSelection",
                "kpositiveSelection",
            ]
            .contains(&role.as_str())
        {
            original = None;
            kind = "contrast";
        }
        provenance.insert(role.clone(),json!({"parent":parent,"type":kind,"original":original,"adjusted":original.is_none()||value.as_str()!=original}));
    }
    for (role, source) in &sources {
        provenance.insert(role.clone(),json!({"parent":role,"type":source["type"],"original":source["sourceColor"],"adjusted":colors[role]!=source["sourceColor"]}));
    }
    Ok((Value::Object(sources), Value::Object(provenance)))
}
pub fn from_analysis(
    mut analysis: Value,
    classic: Option<&Value>,
    settings: &Value,
    hour: Option<&Value>,
) -> Result<Value> {
    let personality = str_at(settings, "personality");
    let background = settings["background_from_wallpaper"]
        .as_bool()
        .ok_or("Invalid settings")?;
    let overrides = settings["overrides"]
        .as_object()
        .ok_or("Invalid settings")?;
    let accent = settings["accent"].as_str();
    let mut mode = str_at(settings, "mode");
    if personality == "natural" {
        let classic = classic.ok_or("Natural requires classic analysis")?;
        for k in ["seed", "candidates", "neutral", "reason"] {
            analysis[k] = classic[k].clone();
        }
        analysis["selectionPolicy"] = json!("classic-natural");
    }
    if settings["smart"] == true {
        mode = if num(&analysis, "meanLightness") >= 0.68 {
            "light"
        } else {
            "dark"
        };
    }
    let seed = overrides
        .get("primary")
        .and_then(Value::as_str)
        .or(accent)
        .unwrap_or(str_at(&analysis, "seed"));
    let seed = pop_seed(&analysis, seed, accent.is_some(), overrides, personality)?;
    let variant = str_at(settings, "variant");
    let effective = if analysis["neutral"] == true
        && accent.is_none()
        && !["pearl", "vivid"].contains(&personality)
        && !["neutral", "monochrome"].contains(&variant)
    {
        "neutral"
    } else {
        variant
    };
    let mut opts = settings.clone();
    opts["mode"] = json!(mode);
    opts["variant"] = json!(effective);
    opts["companions"] = analysis["candidates"].clone();
    opts["body"] = analysis["body"].clone();
    if let Some(hour) = hour {
        opts["hour"] = hour.clone();
    }
    let colors = palette::generate(&seed, &opts)?;
    let (sources, provenance) = role_metadata(&analysis, &colors, &seed, &opts)?;
    // Picker options deliberately use only the policy, not individual adjustments.
    let picker = json!({"mode":mode,"variant":effective,"flavour":settings["flavour"],"personality":personality,"background_from_wallpaper":background});
    analysis["selected"] = json!(seed);
    analysis["options"] = palette_options(&analysis, &picker)?;
    Ok(
        json!({"formatVersion":crate::FORMAT_VERSION,"name":"dynamic","mode":mode,"variant":variant,"flavour":settings["flavour"],"colours":colors,"engine":crate::ENGINE_ID,"input":settings,"source":analysis,"roleSources":sources,"provenance":provenance,"accessibility":accessibility::audit(&colors)?}),
    )
}
pub fn default_palette(options: &Value) -> Result<Value> {
    let colors = palette::generate(palette::DEFAULT_SEED, options)?;
    Ok(
        json!({"formatVersion":crate::FORMAT_VERSION,"name":"default","mode":options.get("mode").cloned().unwrap_or(json!("dark")),"variant":options.get("variant").cloned().unwrap_or(json!("tonalspot")),"flavour":options.get("flavour").cloned().unwrap_or(json!("default")),"colours":colors,"engine":crate::ENGINE_ID,"input":{"personality":"natural"},"accessibility":accessibility::audit(&colors)?}),
    )
}
