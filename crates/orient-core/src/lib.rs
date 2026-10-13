#![forbid(unsafe_code)]
//! Deterministic Orient calculations. Decoding, storage and publication are callers' responsibilities.

pub mod accessibility;
pub mod api;
pub mod classic;
pub mod colour;
pub mod engine;
pub mod extract;
pub mod palette;
pub mod template_data;
pub mod template_engine;

use serde_json::{Map, Value};
pub type Colors = Map<String, Value>;
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Value(String),
    Key(String),
}
impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Self::Value(s.into())
    }
}
impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::Value(s)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(s) | Self::Key(s) => f.write_str(s),
        }
    }
}
impl std::error::Error for Error {}
pub fn text<'a>(map: &'a Colors, key: &str) -> Result<&'a str> {
    map.get(key)
        .ok_or_else(|| Error::Key(key.into()))?
        .as_str()
        .ok_or_else(|| Error::Value("Expected a six-digit RGB color".into()))
}
pub fn str_at<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}
pub fn num(value: &Value, key: &str) -> f64 {
    value[key].as_f64().unwrap_or(0.0)
}
/// Python rounds decimal places correctly, rather than multiplying before rounding.
pub fn rounded(value: f64, places: usize) -> f64 {
    format!("{value:.places$}")
        .parse()
        .expect("finite computation")
}
/// Compensated sum, matching Python 3.12+'s floating point summation.
pub fn sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let (mut high, mut low) = (0.0_f64, 0.0_f64);
    for x in values {
        let t = high + x;
        low += if high.abs() >= x.abs() {
            (high - t) + x
        } else {
            (x - t) + high
        };
        high = t;
    }
    high + low
}
pub const FORMAT_VERSION: u32 = 1;
pub const ENGINE_ID: &str = env!("ORIENT_ENGINE_ID");

/// Python booleans are numeric, but remain booleans in returned input settings.
pub fn numeric(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_bool().map(|v| if v { 1.0 } else { 0.0 }))
}
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(v) => *v,
        Value::Number(v) => v.as_f64().is_some_and(|v| v != 0.0),
        Value::String(v) => !v.is_empty(),
        Value::Array(v) => !v.is_empty(),
        Value::Object(v) => !v.is_empty(),
    }
}
pub fn personality(options: &Value, harmony: bool) -> Result<&str> {
    if truthy(&options["personality"]) {
        options["personality"]
            .as_str()
            .ok_or_else(|| "Unknown palette personality".into())
    } else {
        Ok(if harmony { "harmony" } else { "natural" })
    }
}
