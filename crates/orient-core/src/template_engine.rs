//! Finite role rendering, loops and Matugen expressions; no evaluation or I/O.
use crate::colour;
use crate::{Colors, Error, Result};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::BTreeSet;
pub const MAX_TEMPLATE: usize = 1024 * 1024;
pub const MAX_OUTPUT: usize = 8 * MAX_TEMPLATE;
fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("internal pattern")
}
fn full(pattern: &str, text: &str) -> bool {
    re(&format!("^(?:{pattern})$")).is_match(text)
}
fn valid_color(text: &str) -> bool {
    text.len() == 6 && text.bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn split_quoted(text: &str, separator: char) -> Result<Vec<String>> {
    let (mut start, mut quote, mut escaped) = (0, None, false);
    let mut parts = Vec::new();
    for (index, ch) in text.char_indices() {
        if escaped {
            escaped = false
        } else if ch == '\\' && quote.is_some() {
            escaped = true
        } else if let Some(q) = quote {
            if ch == q {
                quote = None
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch)
        } else if ch == separator {
            parts.push(text[start..index].trim().into());
            start = index + ch.len_utf8();
        }
    }
    if quote.is_some() {
        return Err("Unclosed template string".into());
    }
    parts.push(text[start..].trim().into());
    Ok(parts)
}
fn quoted(text: &str) -> Result<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 2 || !['\'', '"'].contains(&chars[0]) || chars.last() != Some(&chars[0]) {
        return Err("Invalid template string argument".into());
    }
    let quote = chars[0];
    let mut out = String::new();
    let mut i = 1;
    while i < chars.len() - 1 {
        let ch = chars[i];
        if ch == quote || ch == '\n' || ch == '\r' {
            return Err("Invalid template string argument".into());
        }
        if ch != '\\' {
            out.push(ch);
            i += 1;
            continue;
        }
        i += 1;
        if i >= chars.len() - 1 {
            return Err("Invalid template string argument".into());
        }
        let ch = chars[i];
        match ch {
            '\\' | '\'' | '"' => out.push(ch),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'a' => out.push('\u{7}'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'v' => out.push('\u{b}'),
            '\n' => {}
            'x' | 'u' | 'U' => {
                let count = match ch {
                    'x' => 2,
                    'u' => 4,
                    _ => 8,
                };
                if i + count >= chars.len() - 1 {
                    return Err("Invalid template string argument".into());
                }
                let raw: String = chars[i + 1..i + 1 + count].iter().collect();
                let code = u32::from_str_radix(&raw, 16)
                    .map_err(|_| Error::from("Invalid template string argument"))?;
                out.push(char::from_u32(code).ok_or("Invalid template string argument")?);
                i += count;
            }
            '0'..='7' => {
                let mut raw = String::from(ch);
                while raw.len() < 3
                    && i + 1 < chars.len() - 1
                    && ('0'..='7').contains(&chars[i + 1])
                {
                    i += 1;
                    raw.push(chars[i]);
                }
                out.push(char::from_u32(u32::from_str_radix(&raw, 8).unwrap()).unwrap());
            }
            _ => {
                out.push('\\');
                out.push(ch);
            }
        }
        i += 1;
    }
    Ok(out)
}
pub fn literal(text: &str) -> Result<Value> {
    if text.starts_with(['"', '\'']) {
        return quoted(text).map(Value::String);
    }
    if ["hsl", "hsv"].contains(&text) {
        return Ok(json!(text));
    }
    if full(r"[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?", text) {
        if let Ok(v) = text.parse::<f64>() {
            if v.is_finite() {
                return Ok(json!(v));
            }
        }
    }
    Err(format!("Expected a finite number or quoted string argument: {text}").into())
}
pub fn number(value: f64) -> String {
    let result = format!("{value:.3}");
    result.trim_end_matches('0').trim_end_matches('.').into()
}
fn rgb_to_hls([r, g, b]: [f64; 3]) -> [f64; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let sum = max + min;
    let range = max - min;
    let l = sum / 2.0;
    if min == max {
        return [0.0, l, 0.0];
    }
    let s = if l <= 0.5 {
        range / sum
    } else {
        range / (2.0 - max - min)
    };
    let (rc, gc, bc) = ((max - r) / range, (max - g) / range, (max - b) / range);
    let h = if r == max {
        bc - gc
    } else if g == max {
        2.0 + rc - bc
    } else {
        4.0 + gc - rc
    };
    [(h / 6.0).rem_euclid(1.0), l, s]
}
fn hue_channel(m1: f64, m2: f64, h: f64) -> f64 {
    let h = h.rem_euclid(1.0);
    if h < 1.0 / 6.0 {
        m1 + (m2 - m1) * h * 6.0
    } else if h < 0.5 {
        m2
    } else if h < 2.0 / 3.0 {
        m1 + (m2 - m1) * (2.0 / 3.0 - h) * 6.0
    } else {
        m1
    }
}
fn hls_to_rgb([h, l, s]: [f64; 3]) -> [f64; 3] {
    if s == 0.0 {
        return [l, l, l];
    }
    let m2 = if l <= 0.5 {
        l * (1.0 + s)
    } else {
        l + s - (l * s)
    };
    let m1 = 2.0 * l - m2;
    [
        hue_channel(m1, m2, h + 1.0 / 3.0),
        hue_channel(m1, m2, h),
        hue_channel(m1, m2, h - 1.0 / 3.0),
    ]
}
fn rgb_to_hsv([r, g, b]: [f64; 3]) -> [f64; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if min == max {
        return [0.0, 0.0, max];
    }
    let range = max - min;
    let s = range / max;
    let (rc, gc, bc) = ((max - r) / range, (max - g) / range, (max - b) / range);
    let h = if r == max {
        bc - gc
    } else if g == max {
        2.0 + rc - bc
    } else {
        4.0 + gc - rc
    };
    [(h / 6.0).rem_euclid(1.0), s, max]
}
fn hsv_to_rgb([h, s, v]: [f64; 3]) -> [f64; 3] {
    if s == 0.0 {
        return [v, v, v];
    }
    let i = (h * 6.0) as i64;
    let f = h * 6.0 - i as f64;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    match i.rem_euclid(6) {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}
pub fn css_color(text: &str) -> Result<([f64; 4], String)> {
    if full(r"#?[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?", text) {
        let raw = text.strip_prefix('#').unwrap_or(text);
        let mut channels = [0.0, 0.0, 0.0, 1.0];
        for (i, chunk) in raw.as_bytes().chunks_exact(2).enumerate() {
            channels[i] =
                u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap() as f64 / 255.0;
        }
        let fmt = if raw.len() == 8 {
            "hex_alpha"
        } else if text.starts_with('#') {
            "hex"
        } else {
            "hex_stripped"
        };
        return Ok((channels, fmt.into()));
    }
    let pattern = re(r"^(rgba?|hsla?)\(([^()]*)\)$");
    let m = pattern
        .captures(text)
        .ok_or("Color filter requires a CSS color expression")?;
    let fmt = &m[1];
    let parts: Vec<_> = m[2].split(',').map(str::trim).collect();
    if parts.len() != if fmt.ends_with('a') { 4 } else { 3 } {
        return Err("Invalid CSS color channels".into());
    }
    let parse = || -> Result<[f64; 4]> {
        let raw: Vec<f64> = parts
            .iter()
            .map(|v| {
                v.trim_end_matches('%')
                    .parse::<f64>()
                    .map_err(|_| Error::from("Invalid CSS color channel"))
            })
            .collect::<Result<_>>()?;
        if raw.iter().any(|v| !v.is_finite()) {
            return Err("Non-finite CSS color channel".into());
        }
        let rgb = if fmt.starts_with("rgb") {
            [raw[0], raw[1], raw[2]].map(|v| (v / 255.0).clamp(0.0, 1.0))
        } else {
            if !parts[1..3].iter().all(|p| p.ends_with('%')) {
                return Err("HSL channels require percentages".into());
            }
            hls_to_rgb([
                raw[0].rem_euclid(360.0) / 360.0,
                (raw[2] / 100.0).clamp(0.0, 1.0),
                (raw[1] / 100.0).clamp(0.0, 1.0),
            ])
        };
        let alpha = if raw.len() == 4 {
            raw[3].clamp(0.0, 1.0)
        } else {
            1.0
        };
        Ok([rgb[0], rgb[1], rgb[2], alpha])
    };
    Ok((
        parse().map_err(|_| Error::from(format!("Invalid CSS color: {text}")))?,
        fmt.into(),
    ))
}
pub fn serialize([red, green, blue, alpha]: [f64; 4], fmt: &str) -> String {
    let rgb = [red, green, blue].map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5 + 1e-9) as u32);
    if fmt.starts_with("hex") {
        let text = rgb.iter().map(|v| format!("{v:02x}")).collect::<String>();
        if fmt == "hex_alpha" {
            return format!(
                "#{text}{:02x}",
                (alpha.clamp(0.0, 1.0) * 255.0 + 0.5) as u32
            )
            .to_uppercase();
        }
        return if fmt == "hex" {
            format!("#{text}")
        } else {
            text
        };
    }
    let mut body = if fmt.starts_with("rgb") {
        rgb.iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        let [h, l, s] = rgb_to_hls([red, green, blue]);
        format!(
            "{}, {}%, {}%",
            number(h * 360.0),
            number(s * 100.0),
            number(l * 100.0)
        )
    };
    if fmt.ends_with('a') {
        body.push_str(&format!(", {}", number(alpha)));
    }
    format!("{fmt}({body})")
}
pub fn apply_filter(value: &str, expression: &str) -> Result<String> {
    let (name, raw) = expression
        .split_once(':')
        .map(|(n, r)| (n.trim(), Some(r)))
        .unwrap_or((expression.trim(), None));
    let args = if let Some(raw) = raw {
        split_quoted(raw, ',')?
            .iter()
            .map(|v| literal(v))
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    if name == "replace" {
        if args.len() != 2 || !args.iter().all(Value::is_string) {
            return Err("replace requires two quoted strings".into());
        }
        let from = args[0].as_str().unwrap();
        let to = args[1].as_str().unwrap();
        let count = if from.is_empty() {
            value.chars().count() + 1
        } else {
            value.matches(from).count()
        };
        let length = value.chars().count() as i128
            + count as i128 * (to.chars().count() as i128 - from.chars().count() as i128);
        if length > MAX_OUTPUT as i128 {
            return Err("Template replacement exceeds the output limit".into());
        }
        return Ok(value.replace(from, to));
    }
    if ![
        "set_alpha",
        "lighten",
        "auto_lightness",
        "saturate",
        "set_lightness",
    ]
    .contains(&name)
    {
        return Err(format!("Unsupported Matugen filter: {name}").into());
    }
    if !(if name == "saturate" {
        (1..=2).contains(&args.len())
    } else {
        args.len() == 1
    }) || !args[0].is_number()
    {
        return Err(format!("{name} requires a numeric argument").into());
    }
    if name == "saturate" && args.len() == 2 && !matches!(args[1].as_str(), Some("hsl" | "hsv")) {
        return Err("saturate color space must be hsl or hsv".into());
    }
    let ([r, g, b, alpha], fmt) = css_color(value)?;
    let mut amount = args[0].as_f64().unwrap();
    if name == "set_alpha" {
        if !["rgba", "hsla", "hex_alpha"].contains(&fmt.as_str()) {
            return Err("set_alpha requires rgba, hsla or hex_alpha format".into());
        }
        return Ok(serialize([r, g, b, amount.clamp(0.0, 1.0)], &fmt));
    }
    let rgb = if name == "saturate" && args.get(1).is_some_and(|v| v == "hsv") {
        let [h, s, v] = rgb_to_hsv([r, g, b]);
        hsv_to_rgb([h, (s + amount / 100.0).clamp(0.0, 1.0), v])
    } else {
        let [h, mut l, mut s] = rgb_to_hls([r, g, b]);
        if name == "saturate" {
            s = (s + amount / 100.0).clamp(0.0, 1.0)
        } else if name == "set_lightness" {
            l = (amount / 100.0).clamp(0.0, 1.0)
        } else {
            if name == "auto_lightness" && l >= 0.5 {
                amount = -amount
            }
            l = (l + amount / 100.0).clamp(0.0, 1.0)
        }
        hls_to_rgb([h, l, s])
    };
    Ok(serialize([rgb[0], rgb[1], rgb[2], alpha], &fmt))
}
pub fn role_name(name: &str, values: &Colors) -> String {
    if name == "source_color" {
        return "overtone".into();
    }
    if values.contains_key(name) {
        return name.into();
    }
    let mut parts = name.split('_');
    let mut out = parts.next().unwrap_or("").to_owned();
    for part in parts {
        let mut chars = part.chars();
        if let Some(c) = chars.next() {
            out.extend(c.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}
pub fn format_color(value: &Value, fmt: &str) -> Result<String> {
    let value = value
        .as_str()
        .filter(|v| valid_color(v))
        .ok_or("Missing or invalid template color")?
        .to_lowercase();
    let [r, g, b] = colour::rgb(&value)?;
    let [h, l, s] = rgb_to_hls([r, g, b]);
    let [red, green, blue] =
        std::array::from_fn(|i| u32::from_str_radix(&value[i * 2..i * 2 + 2], 16).unwrap());
    Ok(match fmt {
        "hex" => format!("#{value}"),
        "hex_stripped" => value,
        "hex_alpha" => format!("#{}FF", value.to_uppercase()),
        "rgb" => format!("rgb({red}, {green}, {blue})"),
        "rgba" => format!("rgba({red}, {green}, {blue}, 1.0)"),
        "red" => red.to_string(),
        "green" => green.to_string(),
        "blue" => blue.to_string(),
        "alpha" => "255".into(),
        "hsl" => format!(
            "hsl({}, {}%, {}%)",
            number(h * 360.0),
            number(s * 100.0),
            number(l * 100.0)
        ),
        "hsla" => format!(
            "hsla({}, {}%, {}%, 1.0)",
            number(h * 360.0),
            number(s * 100.0),
            number(l * 100.0)
        ),
        "hue" => number(h * 360.0),
        "saturation" => number(s * 100.0),
        "lightness" => number(l * 100.0),
        _ => return Err(format!("Unsupported Matugen color format: {fmt}").into()),
    })
}
fn scalar_string(value: &Value) -> String {
    if let Some(value) = value.as_str() {
        return value.into();
    }
    if value.is_f64() {
        let value = format!("{:?}", value.as_f64().unwrap());
        if let Some((mantissa, exponent)) = value.split_once('e') {
            return format!("{mantissa}e{:+03}", exponent.parse::<i32>().unwrap());
        }
        return value;
    }
    value.to_string()
}
fn snake(role: &str) -> String {
    let chars: Vec<_> = role.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i > 0
            && (chars[i - 1].is_ascii_lowercase() || chars[i - 1].is_ascii_digit())
            && c.is_ascii_uppercase()
        {
            out.push('_');
        }
        out.extend(c.to_lowercase());
    }
    out
}
pub fn public_colors(values: &Colors) -> Vec<(String, String)> {
    let mut result = std::collections::BTreeMap::new();
    for (role, value) in values {
        if !value.as_str().is_some_and(valid_color) {
            continue;
        }
        let name = snake(role);
        if [
            "primary",
            "secondary",
            "tertiary",
            "error",
            "on_",
            "surface",
            "outline",
            "inverse",
            "neutral",
        ]
        .iter()
        .any(|v| name.starts_with(v))
            || ["background", "shadow", "scrim"].contains(&name.as_str())
        {
            result.insert(name, role.clone());
        }
    }
    if values.contains_key("overtone") {
        result.insert("source_color".into(), "overtone".into());
    }
    result.into_iter().collect()
}
fn token_regex() -> Regex {
    re(r"(\\)?\{\{([^{}]*?)\}\}")
}
pub fn requested_modes(template: &str) -> Result<BTreeSet<String>> {
    let mut modes = BTreeSet::new();
    for m in token_regex().captures_iter(template) {
        if m.get(1).is_some() {
            continue;
        }
        let base = split_quoted(m[2].trim(), '|')?.remove(0);
        let base = re(r"\s*\.\s*").replace_all(&base, ".");
        if let Some(request) =
            re(r"^[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)?\.(dark|light)\.[A-Za-z_]\w*$").captures(&base)
        {
            modes.insert(request[1].into());
        }
    }
    Ok(modes)
}
pub fn requested_contexts(template: &str) -> Result<BTreeSet<String>> {
    let mut result = BTreeSet::new();
    for m in token_regex().captures_iter(template) {
        if m.get(1).is_none() {
            let base = split_quoted(m[2].trim(), '|')?.remove(0);
            let name = base.split('.').next().unwrap_or("").trim();
            if ["palettes", "base16"].contains(&name) {
                result.insert(name.into());
            }
        }
    }
    for m in piece_regex().captures_iter(template) {
        if let Some(block) = m.get(3) {
            if let Some(loop_) = re(r"^for\s+\w+\s*,\s*\w+\s+in\s+(palettes|base16)(?:\.\w+)*$")
                .captures(block.as_str().trim())
            {
                result.insert(loop_[1].into());
            }
        }
    }
    Ok(result)
}
fn piece_regex() -> Regex {
    re(r"(?s)(\\)?\{\{([^{}]*?)\}\}|<\*(.*?)\*>")
}
#[derive(Clone, Debug)]
enum Node {
    Text(String),
    Token(String),
    If(String, Vec<Node>, Vec<Node>),
    Loop(String, String, String, Vec<Node>),
}
fn malformed(plain: &str) -> bool {
    ["{{", "<*", "{%"]
        .iter()
        .any(|marker| plain.contains(marker))
}
fn parse_template(template: &str) -> Result<Vec<Node>> {
    if template.chars().count() > MAX_TEMPLATE {
        return Err("Template exceeds the 1 MiB input limit".into());
    }
    let mut pieces: Vec<(u8, String)> = Vec::new();
    let mut position = 0;
    for m in piece_regex().captures_iter(template) {
        let range = m.get(0).unwrap();
        let plain = &template[position..range.start()];
        if malformed(plain) {
            return Err("Malformed or unsupported template expression/block".into());
        }
        pieces.push((0, plain.into()));
        if let Some(block) = m.get(3) {
            pieces.push((1, block.as_str().trim().into()));
        } else if m.get(1).is_some() {
            pieces.push((0, format!("{{{{{}}}}}", &m[2])));
        } else {
            pieces.push((2, m[2].into()));
        }
        position = range.end();
    }
    let tail = &template[position..];
    if malformed(tail) {
        return Err("Malformed or unsupported template expression/block".into());
    }
    pieces.push((0, tail.into()));
    let mut index = 0;
    group(&pieces, &mut index, 0, 0, &[]).map(|v| v.0)
}
fn group(
    pieces: &[(u8, String)],
    index: &mut usize,
    depth: usize,
    loop_depth: usize,
    stops: &[&str],
) -> Result<(Vec<Node>, Option<String>)> {
    if depth > 16 || loop_depth > 4 {
        return Err("Template block/loop nesting exceeds its limit".into());
    }
    let mut nodes = Vec::new();
    while *index < pieces.len() {
        let (block, value) = &pieces[*index];
        *index += 1;
        if *block != 1 {
            nodes.push(if *block == 2 {
                Node::Token(value.clone())
            } else {
                Node::Text(value.clone())
            });
            continue;
        }
        if ["endfor", "else", "endif"].contains(&value.as_str()) {
            if !stops.contains(&value.as_str()) {
                return Err(format!("Unexpected {value} block").into());
            }
            return Ok((nodes, Some(value.clone())));
        }
        if let Some(m) = re(r"^if\s+\{\{\s*([^{}]+?)\s*\}\}$").captures(value) {
            let (yes, ending) = group(pieces, index, depth + 1, loop_depth, &["else", "endif"])?;
            let no = if ending.as_deref() == Some("else") {
                group(pieces, index, depth + 1, loop_depth, &["endif"])?.0
            } else {
                Vec::new()
            };
            nodes.push(Node::If(m[1].trim().into(), yes, no));
            continue;
        }
        let pattern = re(
            r"^for\s+([A-Za-z_]\w*)\s*,\s*([A-Za-z_]\w*)\s+in\s+([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)$",
        );
        let m = pattern
            .captures(value)
            .ok_or_else(|| Error::from(format!("Unsupported Matugen block: {value}")))?;
        let names = [&m[1], &m[2]];
        if names[0] == names[1]
            || names.iter().any(|n| {
                [
                    "colors",
                    "palettes",
                    "base16",
                    "custom",
                    "mode",
                    "image",
                    "is_dark_mode",
                    "loop",
                ]
                .contains(n)
            })
        {
            return Err("Loop bindings must be distinct non-reserved names".into());
        }
        let child = group(pieces, index, depth + 1, loop_depth + 1, &["endfor"])?.0;
        nodes.push(Node::Loop(m[1].into(), m[2].into(), m[3].into(), child));
    }
    if !stops.is_empty() {
        return Err(format!("Missing {} block", stops.join("/")).into());
    }
    Ok((nodes, None))
}
struct Renderer<'a> {
    values: &'a Colors,
    contexts: Colors,
    roots: Colors,
    mode: &'a str,
    image: Option<&'a str>,
    custom: Option<&'a Colors>,
    output: String,
    length: usize,
    operations: usize,
}
impl Renderer<'_> {
    fn lookup(&self, expression: &str, scope: &Colors) -> Result<Value> {
        let parts: Vec<_> = expression.split('.').map(str::trim).collect();
        let mut value = scope
            .get(parts[0])
            .or_else(|| self.roots.get(parts[0]))
            .filter(|v| !v.is_null())
            .ok_or_else(|| Error::from(format!("Missing template map context: {}", parts[0])))?
            .clone();
        for part in &parts[1..] {
            if let Some(map) = value.as_object() {
                value = map.get(*part).cloned().ok_or_else(|| {
                    if ["dark", "light"].contains(part) {
                        Error::from(format!(
                            "Template requires the {part} palette; supply a companion palette"
                        ))
                    } else {
                        Error::from(format!("Missing template map value: {expression}"))
                    }
                })?;
            } else if value.as_str().is_some_and(valid_color) {
                value = json!(format_color(&value, part)?);
            } else {
                return Err(format!("Invalid template map expression: {expression}").into());
            }
        }
        Ok(value)
    }
    fn base(&self, expression: &str, scope: &Colors) -> Result<String> {
        if expression.starts_with(['"', '\'']) {
            return literal(expression)?
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| Error::from("Invalid template string argument"));
        }
        if expression == "mode" && ["dark", "light"].contains(&self.mode) {
            return Ok(self.mode.into());
        }
        if expression == "is_dark_mode" && ["dark", "light"].contains(&self.mode) {
            return Ok(if self.mode == "dark" { "true" } else { "false" }.into());
        }
        if expression == "image" {
            return self
                .image
                .filter(|s| !s.contains('\0'))
                .map(str::to_owned)
                .ok_or_else(|| {
                    Error::from("Template requests image but no source image was supplied")
                });
        }
        if let Some(name) = expression.strip_prefix("custom.") {
            let value = self.custom.and_then(|map| map.get(name));
            if !full(r"[A-Za-z_][A-Za-z0-9_]*", name)
                || !value.is_some_and(|v| v.is_string() || v.is_number() || v.is_boolean())
            {
                return Err(format!("Missing or unsupported custom keyword: {name}").into());
            }
            let value = value.unwrap();
            return Ok(scalar_string(value));
        }
        let mut parts: Vec<_> = expression.split('.').map(str::trim).collect();
        if scope.contains_key(parts[0]) || ["palettes", "base16"].contains(&parts[0]) {
            let value = self.lookup(expression, scope)?;
            return if value.is_string() || value.is_boolean() || value.is_i64() || value.is_u64() {
                Ok(scalar_string(&value))
            } else {
                Err(format!("Template expression must produce a scalar: {expression}").into())
            };
        }
        if parts[0] == "colors" {
            if parts.len() == 3 {
                parts.insert(2, "default")
            }
            if parts.len() != 4 {
                return Err(format!(
                    "Expected colors.ROLE.default|dark|light.FORMAT: {expression}"
                )
                .into());
            }
            let scheme = parts[2];
            if !["default", "dark", "light"].contains(&scheme) {
                return Err("Expected default, dark or light color scheme".into());
            }
            let context = self
                .contexts
                .get(scheme)
                .and_then(Value::as_object)
                .ok_or_else(|| {
                    Error::from(format!(
                        "Template requires the {scheme} palette; supply a companion palette"
                    ))
                })?;
            return format_color(
                context
                    .get(&role_name(parts[1], context))
                    .unwrap_or(&Value::Null),
                parts[3],
            );
        }
        if !full(r"[A-Za-z][A-Za-z0-9_]*", expression) {
            return Err(format!("Unsupported template expression: {expression}").into());
        }
        self.values
            .get(expression)
            .and_then(Value::as_str)
            .filter(|v| valid_color(v))
            .map(str::to_lowercase)
            .ok_or_else(|| {
                Error::from(format!(
                    "Missing or invalid color template token: {expression}"
                ))
            })
    }
    fn evaluate(&self, expression: &str, scope: &Colors) -> Result<String> {
        let pipeline = split_quoted(expression.trim(), '|')?;
        let mut value = self.base(&pipeline[0], scope)?;
        for step in &pipeline[1..] {
            value = apply_filter(&value, step)?;
        }
        Ok(value)
    }
    fn emit(&mut self, text: &str) -> Result<()> {
        self.length += text.chars().count();
        if self.length > MAX_OUTPUT {
            return Err("Template expansion exceeds the 8 MiB output limit".into());
        }
        self.output.push_str(text);
        Ok(())
    }
    fn visit(&mut self, nodes: &[Node], scope: &Colors) -> Result<()> {
        for node in nodes {
            self.operations += 1;
            if self.operations > 100000 {
                return Err("Template exceeds the 100000-operation expansion limit".into());
            }
            match node {
                Node::Text(text) => self.emit(text)?,
                Node::Token(token) => {
                    let value = self.evaluate(token, scope)?;
                    self.emit(&value)?;
                }
                Node::If(expression, yes, no) => {
                    if expression == "is_dark_mode" && !["dark", "light"].contains(&self.mode) {
                        return Err("Template condition requires a light/dark mode".into());
                    }
                    let value = if expression == "is_dark_mode" {
                        json!(self.mode == "dark")
                    } else {
                        self.lookup(expression, scope)?
                    };
                    let condition = value.as_bool().ok_or_else(|| {
                        Error::from(format!(
                            "Template condition must be a boolean: {expression}"
                        ))
                    })?;
                    self.visit(if condition { yes } else { no }, scope)?;
                }
                Node::Loop(a, b, expression, children) => {
                    let mapping = self.lookup(expression, scope)?;
                    let mapping = mapping.as_object().ok_or_else(|| {
                        Error::from(format!("Template loop requires a map: {expression}"))
                    })?;
                    for (index, (name, item)) in mapping.iter().enumerate() {
                        let mut child = scope.clone();
                        child.insert(a.clone(), json!(name));
                        child.insert(b.clone(), item.clone());
                        child.insert(
                            "loop".into(),
                            json!({"last":index==mapping.len()-1,"first":index==0}),
                        );
                        self.visit(children, &child)?;
                    }
                }
            }
        }
        Ok(())
    }
}
pub fn render_values(
    template: &str,
    values: &Colors,
    schemes: Option<&Colors>,
    image: Option<&str>,
    custom: Option<&Colors>,
    data: Option<&Colors>,
) -> Result<String> {
    let mode = values.get("mode").and_then(Value::as_str).unwrap_or("");
    let mut contexts = schemes.cloned().unwrap_or_default();
    if ["dark", "light"].contains(&mode) {
        contexts.insert(mode.into(), json!(values));
    }
    contexts.insert("default".into(), json!(values));
    let mut mapping = Colors::new();
    for (name, role) in public_colors(values) {
        let modes: Colors = contexts
            .iter()
            .filter_map(|(mode, context)| context.get(&role).map(|v| (mode.clone(), v.clone())))
            .collect();
        mapping.insert(name, Value::Object(modes));
    }
    let mut roots = Colors::new();
    roots.insert("colors".into(), Value::Object(mapping));
    if let Some(data) = data {
        for (namespace, mapping) in data {
            if !["palettes", "base16"].contains(&namespace.as_str()) || !mapping.is_object() {
                return Err(format!("Unsupported template map context: {namespace}").into());
            }
            roots.insert(namespace.clone(), mapping.clone());
        }
    }
    let nodes = parse_template(template)?;
    let mut renderer = Renderer {
        values,
        contexts,
        roots,
        mode,
        image,
        custom,
        output: String::new(),
        length: 0,
        operations: 0,
    };
    renderer.visit(&nodes, &Colors::new())?;
    Ok(renderer.output)
}
pub fn render(
    template: &str,
    palette: &Value,
    schemes: Option<&Colors>,
    custom: Option<&Colors>,
) -> Result<String> {
    let values = palette["colours"].as_object().ok_or("Invalid palette")?;
    crate::palette::validate(values)?;
    let mode = palette["mode"].as_str().ok_or("Invalid palette")?;
    let contexts = schemes.cloned().unwrap_or_default();
    for values in contexts.values() {
        crate::palette::validate(values.as_object().ok_or("Invalid palette")?)?;
    }
    let mut data = Colors::new();
    let requested = requested_contexts(template)?;
    if requested.contains("palettes") {
        data.insert(
            "palettes".into(),
            crate::template_data::tonal_contexts(values)?,
        );
    }
    if requested.contains("base16") {
        let mut all = contexts.clone();
        all.insert("default".into(), json!(values));
        all.insert(mode.into(), json!(values));
        data.insert(
            "base16".into(),
            crate::template_data::base16_contexts(&all, mode)?,
        );
    }
    let mut values = values.clone();
    values.insert("mode".into(), json!(mode));
    render_values(
        template,
        &values,
        Some(&contexts),
        palette["source"]["path"].as_str(),
        custom,
        Some(&data),
    )
}
