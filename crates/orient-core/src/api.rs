//! Shared pure JSON-facing API for Python bindings and the native client bridge.
use crate as core;
use crate::{Colors, Error, Result};
use serde_json::{json, Value};
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| Error::Value("Expected a six-digit RGB color".into()))
}
fn number(value: &Value, key: &str) -> Result<f64> {
    core::numeric(&value[key]).ok_or_else(|| Error::Value(format!("Expected numeric {key}")))
}
fn map<'a>(value: &'a Value, key: &str) -> Result<&'a Colors> {
    value[key]
        .as_object()
        .ok_or_else(|| Error::Value(format!("Expected {key} mapping")))
}
pub fn invoke(op: &str, v: &Value) -> Result<Value> {
    use core::{
        accessibility as a, colour as c, engine as e, palette as p, template_data as d,
        template_engine as t,
    };
    Ok(match op {
        "clean" => json!(c::clean(string(v, "value")?)?),
        "rgb" => json!(c::rgb(string(v, "value")?)?),
        "lch" => json!(c::lch(string(v, "value")?)?),
        "linear" => json!(c::linear(number(v, "channel")?)),
        "encoded" => json!(c::encoded(number(v, "channel")?)),
        "luminance" => json!(c::luminance(string(v, "value")?)?),
        "contrast" => json!(c::contrast(string(v, "a")?, string(v, "b")?)?),
        "lab_from_rgb" => {
            let channels = v["channels"]
                .as_array()
                .ok_or("Expected three RGB channels")?;
            if channels.len() != 3 {
                return Err("Expected three RGB channels".into());
            }
            json!(c::lab_from_rgb([
                channels[0].as_f64().ok_or("Expected numeric channel")?,
                channels[1].as_f64().ok_or("Expected numeric channel")?,
                channels[2].as_f64().ok_or("Expected numeric channel")?
            ]))
        }
        "lab_to_linear" => json!(c::lab_to_linear(
            number(v, "light")?,
            number(v, "a")?,
            number(v, "b")?
        )),
        "color" => json!(c::color(
            number(v, "light")?,
            number(v, "chroma")?,
            number(v, "hue")?
        )),
        "foreground" => json!(c::foreground(string(v, "background")?)?),
        "readable" => {
            let backgrounds = v["backgrounds"]
                .as_array()
                .ok_or("Expected backgrounds")?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .ok_or("Expected a six-digit RGB color")
                })
                .collect::<std::result::Result<Vec<_>, _>>()?;
            json!(c::readable(
                string(v, "value")?,
                &backgrounds,
                number(v, "minimum")?
            )?)
        }
        "hue_distance" => json!(c::hue_distance(number(v, "a")?, number(v, "b")?)),
        "simulate" => json!(a::simulate(string(v, "value")?, string(v, "vision")?)?),
        "difference" => json!(a::difference(
            string(v, "a")?,
            string(v, "b")?,
            string(v, "vision")?
        )?),
        "text_pairs" => json!(a::text_pairs()),
        "audit" => a::audit(map(v, "colors")?)?,
        "generate" => Value::Object(p::generate(string(v, "seed")?, &v["options"])?),
        "validate" => {
            p::validate(map(v, "colors")?)?;
            Value::Null
        }
        "background_policy" => json!(p::background_policy(
            string(v, "personality")?,
            &v["background_from_wallpaper"]
        )?),
        "supporting_sources" => json!(p::supporting_sources(
            string(v, "seed")?,
            v["candidates"].as_array().ok_or("Expected candidates")?,
            v["harmony"].as_bool().unwrap_or(false)
        )?),
        "constants" => {
            json!({"DEFAULT_SEED":p::DEFAULT_SEED,"SEMANTIC_SEEDS":p::SEMANTIC_SEEDS.into_iter().collect::<std::collections::BTreeMap<_,_>>(),"VARIANTS":p::VARIANTS,"PERSONALITIES":p::PERSONALITIES,"REQUIRED_ROLES": &p::REQUIRED_ROLES[..]})
        }
        "settings" => e::settings(&v["options"])?,
        "from_analysis" => e::from_analysis(
            v["analysis"].clone(),
            v.get("classic").filter(|v| !v.is_null()),
            &v["settings"],
            v.get("hour"),
        )?,
        "default_palette" => e::default_palette(&v["options"])?,
        "palette_options" => e::palette_options(&v["analysis"], &v["options"])?,
        "render_values" => json!(t::render_values(
            string(v, "template")?,
            map(v, "values")?,
            v["schemes"].as_object(),
            v["image"].as_str(),
            v["custom"].as_object(),
            v["data"].as_object()
        )?),
        "render" => json!(t::render(
            string(v, "template")?,
            &v["palette"],
            v["schemes"].as_object(),
            v["custom"].as_object()
        )?),
        "requested_modes" => json!(t::requested_modes(string(v, "template")?)?),
        "requested_contexts" => json!(t::requested_contexts(string(v, "template")?)?),
        "tonal_contexts" => d::tonal_contexts(map(v, "values")?)?,
        "base16_contexts" => d::base16_contexts(map(v, "contexts")?, string(v, "active_mode")?)?,
        "BASE16_ROLES" => json!(d::BASE16_ROLES),
        _ => return Err("Unknown core operation".into()),
    })
}
