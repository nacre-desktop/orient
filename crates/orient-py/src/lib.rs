use orient_core::{self as core, Error};
use pyo3::{
    exceptions::{PyKeyError, PyValueError},
    prelude::*,
};
use serde_json::Value;
fn error(error: Error) -> PyErr {
    match error {
        Error::Value(s) => PyValueError::new_err(s),
        Error::Key(s) => PyKeyError::new_err(s),
    }
}
#[pyfunction]
fn invoke(py: Python<'_>, operation: &str, payload: &str) -> PyResult<String> {
    let value: Value =
        serde_json::from_str(payload).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let result = py
        .detach(|| core::api::invoke(operation, &value))
        .map_err(error)?;
    serde_json::to_string(&result).map_err(|e| PyValueError::new_err(e.to_string()))
}
#[pyfunction]
fn analyze_frames(
    py: Python<'_>,
    frames: Vec<(usize, usize, Vec<u8>)>,
    classic: bool,
) -> PyResult<String> {
    let frames: Vec<_> = frames
        .into_iter()
        .map(|(width, height, rgba)| core::extract::Frame {
            width,
            height,
            rgba,
        })
        .collect();
    let result = py
        .detach(|| {
            if classic {
                core::classic::analyze(&frames)
            } else {
                core::extract::analyze(&frames)
            }
        })
        .map_err(error)?;
    serde_json::to_string(&result).map_err(|e| PyValueError::new_err(e.to_string()))
}
#[pyfunction]
fn sample_pixel(width: usize, height: usize, rgba: Vec<u8>, x: f64, y: f64) -> PyResult<String> {
    core::extract::sample_color(
        &core::extract::Frame {
            width,
            height,
            rgba,
        },
        x,
        y,
    )
    .map_err(error)
}
#[pymodule]
fn _core(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(invoke, module)?)?;
    module.add_function(wrap_pyfunction!(sample_pixel, module)?)?;
    module.add_function(wrap_pyfunction!(analyze_frames, module)?)?;
    module.add("ENGINE_ID", core::ENGINE_ID)?;
    module.add("FORMAT_VERSION", core::FORMAT_VERSION)?;
    Ok(())
}
