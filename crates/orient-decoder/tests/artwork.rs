use std::path::PathBuf;

#[test]
fn pillow_artwork_png_bytes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/parity/media-stage");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
    let mut failures = Vec::new();
    for case in manifest["cases"].as_array().unwrap() {
        let name = case["input"].as_str().unwrap();
        let actual = orient_decoder::artwork_png(&root.join(case["source"].as_str().unwrap()));
        if let Some(expected) = case["error"].as_str() {
            match actual {
                Err(error) if error.to_string() == expected => {}
                Err(error) => failures.push(format!("{name}: expected {expected:?}, got {error}")),
                Ok(_) => failures.push(format!("{name}: expected error {expected:?}")),
            }
            continue;
        }
        let expected =
            std::fs::read(root.join("expected").join(case["png"].as_str().unwrap())).unwrap();
        match actual {
            Ok(actual) if actual == expected => {}
            Ok(actual) => {
                let first = actual.iter().zip(&expected).position(|(a, b)| a != b);
                failures.push(format!(
                    "{name}: PNG bytes differ at {first:?}, lengths {} / {}",
                    actual.len(),
                    expected.len()
                ));
            }
            Err(error) => failures.push(format!("{name}: {error}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
