//! Local JSON interface, with an optional persistent JSON-lines transport.
use crate::{build_info, decoder, engine_id, export, from_image, resolve, Result};
use serde_json::{json, Map, Value};
use std::{
    io::{BufRead, Read, Write},
    path::Path,
};
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .ok_or_else(|| crate::error(&format!("Expected {key} string")))
}
fn execute(operation: &str, payload: &Value) -> Result<Value> {
    Ok(match operation {
        "info" => build_info(),
        "from_image" => from_image(Path::new(string(payload, "path")?), &payload["options"])?,
        "analyze" => orient_core::extract::analyze(&decoder::frames(&resolve(Path::new(
            string(payload, "path")?,
        ))?)?)?,
        "sample_color" => {
            let x = orient_core::numeric(&payload["x"])
                .ok_or("Pick coordinates between zero and one")?;
            let y = orient_core::numeric(&payload["y"])
                .ok_or("Pick coordinates between zero and one")?;
            let path = resolve(Path::new(string(payload, "path")?))?;
            let frame = if decoder::is_video(&path) {
                decoder::frames(&path)?.remove(0)
            } else {
                decoder::still(&path)?
            };
            json!(orient_core::extract::sample_color(&frame, x, y)?)
        }
        "render" => {
            let empty = Map::new();
            json!(export::render(
                string(payload, "template")?,
                &payload["palette"],
                payload["schemes"].as_object().unwrap_or(&empty),
                payload["custom"].as_object()
            )?)
        }
        "export" => {
            let empty = Map::new();
            json!(export::export(
                &payload["palette"],
                Path::new(string(payload, "directory")?),
                payload["templates"].as_str().map(Path::new),
                payload["schemes"].as_object().unwrap_or(&empty),
                payload["custom"].as_object()
            )?)
        }
        "constants" => {
            let mut data = orient_core::api::invoke(operation, payload)?;
            data["ENGINE_ID"] = json!(engine_id());
            data["FORMAT_VERSION"] = json!(orient_core::FORMAT_VERSION);
            data["VERSION"] = build_info()["version"].clone();
            data
        }
        "default_palette" => {
            let mut data = orient_core::api::invoke(operation, payload)?;
            data["engine"] = json!(engine_id());
            data
        }
        _ => orient_core::api::invoke(operation, payload)?,
    })
}
pub fn serve() {
    let request = || -> Result<Value> {
        let mut input = Vec::new();
        std::io::stdin()
            .take(16 * 1024 * 1024 + 1)
            .read_to_end(&mut input)?;
        if input.len() > 16 * 1024 * 1024 {
            return Err(crate::error("Native request exceeds its 16 MiB limit"));
        }
        let data: Value = serde_json::from_slice(&input)?;
        execute(string(&data, "operation")?, &data["payload"])
    };
    let result = request();
    let failed = result.is_err();
    let response = response(result);
    let output = serde_json::to_vec(&response).expect("finite engine reply");
    if std::io::stdout().write_all(&output).is_err() {
        std::process::exit(1)
    }
    if failed {
        std::process::exit(1)
    }
}

fn response(result: Result<Value>) -> Value {
    match result {
        Ok(value) => json!({"interfaceVersion":1,"engine":engine_id(),"result":value}),
        Err(error) => {
            let kind = match error.downcast_ref::<orient_core::Error>() {
                Some(orient_core::Error::Key(_)) => "key",
                Some(orient_core::Error::Value(_)) => "value",
                None => {
                    if error.is::<std::io::Error>() {
                        "io"
                    } else {
                        "value"
                    }
                }
            };
            json!({"interfaceVersion":1,"engine":engine_id(),"error":{"kind":kind,"message":error.to_string()}})
        }
    }
}

/// Process bounded JSON lines serially. EOF shuts down without polling or threads.
pub fn serve_lines() -> Result<()> {
    const LIMIT: u64 = 16 * 1024 * 1024;
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    loop {
        let mut line = Vec::new();
        if input
            .by_ref()
            .take(LIMIT + 1)
            .read_until(b'\n', &mut line)?
            == 0
        {
            return Ok(());
        }
        if line.len() as u64 > LIMIT {
            let mut reply = response(Err(crate::error("Native request exceeds its 16 MiB limit")));
            reply["id"] = Value::Null;
            serde_json::to_writer(&mut output, &reply)?;
            output.write_all(b"\n")?;
            output.flush()?;
            return Err(crate::error("Native request exceeds its 16 MiB limit"));
        }
        let request = serde_json::from_slice::<Value>(&line);
        let id = request
            .as_ref()
            .ok()
            .and_then(|v| v.get("id"))
            .cloned()
            .unwrap_or(Value::Null);
        let result = request.map_err(Into::into).and_then(|data| {
            if !(id.is_string() || id.is_u64() || id.is_i64()) {
                return Err(crate::error("Expected request id string or integer"));
            }
            execute(string(&data, "operation")?, &data["payload"])
        });
        let mut reply = response(result);
        reply["id"] = id;
        serde_json::to_writer(&mut output, &reply)?;
        output.write_all(b"\n")?;
        output.flush()?;
        if crate::CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(crate::error("Native request cancelled"));
        }
    }
}
