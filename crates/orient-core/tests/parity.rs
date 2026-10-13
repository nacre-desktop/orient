use orient_core::{
    classic,
    extract::{self, Frame},
    Error,
};
use serde_json::{json, Value};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/parity/golden")
}
fn reference() -> Value {
    let mut buf = Vec::new();
    flate2::read::GzDecoder::new(File::open(root().join("outputs.json.gz")).unwrap())
        .read_to_end(&mut buf)
        .unwrap();
    serde_json::from_slice(&buf).unwrap()
}
fn expand(value: &Value, pool: &Value) -> Value {
    if let Some(i) = value.get("$ref") {
        return expand(&pool[i.as_u64().unwrap() as usize], pool);
    }
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), expand(v, pool)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(|v| expand(v, pool)).collect()),
        _ => value.clone(),
    }
}
fn resolve<'a>(mut value: &'a Value, pool: &'a Value) -> &'a Value {
    while let Some(i) = value.get("$ref") {
        value = &pool[i.as_u64().unwrap() as usize];
    }
    value
}
fn frames(case: &Value) -> Vec<Frame> {
    case["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| {
            let mut decoder = png::Decoder::new(BufReader::new(
                File::open(root().join(name.as_str().unwrap())).unwrap(),
            ));
            decoder.set_transformations(png::Transformations::EXPAND);
            let mut reader = decoder.read_info().unwrap();
            let mut data = vec![0; reader.output_buffer_size().unwrap()];
            let info = reader.next_frame(&mut data).unwrap();
            assert_eq!(info.color_type, png::ColorType::Rgba);
            data.truncate(info.buffer_size());
            Frame {
                width: info.width as usize,
                height: info.height as usize,
                rgba: data,
            }
        })
        .collect()
}
fn outcome(result: orient_core::Result<Value>) -> Value {
    match result {
        Ok(v) => json!({"value":v}),
        Err(Error::Value(message)) => json!({"error":"ValueError","message":message}),
        Err(Error::Key(message)) => json!({"error":"KeyError","message":message}),
    }
}
fn difference(a: &Value, b: &Value, path: &str) -> Option<String> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            if x.len() != y.len() {
                return Some(format!("{path}: key counts {} != {}", x.len(), y.len()));
            }
            for (k, v) in x {
                if !y.contains_key(k) {
                    return Some(format!("{path}: missing {k}"));
                }
                if let Some(d) = difference(v, &y[k], &format!("{path}.{k}")) {
                    return Some(d);
                }
            }
            None
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Some(format!("{path}: lengths {} != {}", x.len(), y.len()));
            }
            for (i, (v, w)) in x.iter().zip(y).enumerate() {
                if let Some(d) = difference(v, w, &format!("{path}[{i}]")) {
                    return Some(d);
                }
            }
            None
        }
        _ => {
            if a == b {
                None
            } else {
                Some(format!("{path}: {a} != {b}"))
            }
        }
    }
}
fn same(a: Value, b: &Value, label: &str) {
    if let Some(d) = difference(&a, b, label) {
        panic!("{d}")
    }
}
#[test]
fn normalized_pixel_parity() {
    let data = reference();
    let pool = &data["pool"];
    let cases = resolve(&resolve(&data["root"], pool)["images"], pool)
        .as_array()
        .unwrap();
    for item in cases {
        let case = expand(item, pool);
        let images = frames(&case);
        let label = case["name"].as_str().unwrap();
        same(
            outcome(extract::analyze(&images)),
            &case["analysis"],
            &format!("{label}.analysis"),
        );
        same(
            outcome(classic::analyze(&images)),
            &case["classic"],
            &format!("{label}.classic"),
        );
        for sample in case["samples"].as_array().unwrap() {
            same(
                outcome(
                    extract::sample_color(
                        &images[0],
                        sample["point"][0].as_f64().unwrap(),
                        sample["point"][1].as_f64().unwrap(),
                    )
                    .map(Value::String),
                ),
                &sample["expected"],
                &format!("{label}.sample"),
            );
        }
    }
}
#[test]
fn generated_role_parity() {
    let data = reference();
    let pool = &data["pool"];
    let root = resolve(&data["root"], pool);
    for field in ["generated", "defaults"] {
        for item in resolve(&root[field], pool).as_array().unwrap() {
            let case = expand(item, pool);
            let (seed, options) = if field == "defaults" {
                ("47a99a", case["options"].clone())
            } else {
                let mut opts = case["kwargs"].clone();
                opts["mode"] = case["args"][1].clone();
                (case["args"][0].as_str().unwrap(), opts)
            };
            same(
                Value::Object(orient_core::palette::generate(seed, &options).unwrap()),
                if field == "defaults" {
                    &case["expected"]["value"]["colours"]
                } else {
                    &case["expected"]["value"]
                },
                "placeholder",
            );
        }
    }
}
fn scrub(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.remove("engine");
            m.remove("path");
            for v in m.values_mut() {
                scrub(v)
            }
        }
        Value::Array(a) => {
            for v in a {
                scrub(v)
            }
        }
        _ => {}
    }
}
#[test]
fn image_palette_parity() {
    let data = reference();
    let pool = &data["pool"];
    for item in resolve(&resolve(&data["root"], pool)["images"], pool)
        .as_array()
        .unwrap()
    {
        let case = expand(item, pool);
        let images = frames(&case);
        let label = case["name"].as_str().unwrap();
        let analysis = extract::analyze(&images);
        let classic = classic::analyze(&images);
        for (index, record) in case["palettes"].as_array().unwrap().iter().enumerate() {
            let result = analysis.clone().and_then(|mut a| {
                let opts = orient_core::engine::settings(&record["options"])?;
                a["digest"] = record["expected"]["value"]["source"]["digest"].clone();
                orient_core::engine::from_analysis(
                    a,
                    classic.as_ref().ok(),
                    &opts,
                    record["options"].get("hour"),
                )
            });
            let mut actual = outcome(result);
            scrub(&mut actual);
            same(
                actual,
                &record["expected"],
                &format!("{label}.palette[{index}]"),
            );
        }
    }
}
#[test]
fn template_parity() {
    let data = reference();
    let pool = &data["pool"];
    let root = resolve(&data["root"], pool);
    for item in resolve(&root["expressions"], pool).as_array().unwrap() {
        let c = expand(item, pool);
        let args = c["args"].as_array().unwrap();
        let kwargs = &c["kwargs"];
        let get = |index: usize, name: &str| args.get(index).or_else(|| kwargs.get(name));
        let result = orient_core::template_engine::render_values(
            args[0].as_str().unwrap(),
            args[1].as_object().unwrap(),
            get(2, "schemes").and_then(Value::as_object),
            get(3, "image").and_then(Value::as_str),
            get(4, "custom").and_then(Value::as_object),
            get(5, "data").and_then(Value::as_object),
        );
        same(
            outcome(result.map(Value::String)),
            &c["expected"],
            &format!(
                "expression: {}",
                args[0]
                    .as_str()
                    .unwrap()
                    .chars()
                    .take(100)
                    .collect::<String>()
            ),
        );
    }
    for item in resolve(&root["templates"], pool).as_array().unwrap() {
        let c = expand(item, pool);
        let palette = orient_core::engine::default_palette(&json!({"mode":c["mode"]})).unwrap();
        same(
            outcome(
                orient_core::template_engine::render(
                    c["text"].as_str().unwrap(),
                    &palette,
                    None,
                    None,
                )
                .map(Value::String),
            ),
            &c["expected"],
            c["name"].as_str().unwrap(),
        );
    }
}
