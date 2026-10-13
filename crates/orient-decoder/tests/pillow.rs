//! Frozen CURRENT Python decoder outputs; never regenerate to bless a mismatch.
use std::path::Path;

#[test]
fn frozen_pillow_pixels() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/parity/decoder");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
    let mut failures = Vec::new();
    for case in manifest["cases"].as_array().unwrap() {
        let input = case["input"].as_str().unwrap();
        let frames = orient_decoder::frames(&root.join("input").join(input)).unwrap();
        let expected = case["frames"].as_array().unwrap();
        assert_eq!(frames.len(), expected.len(), "{input}: frame count");
        for (actual, expected) in frames.iter().zip(expected) {
            let expected = image::open(
                root.join("expected")
                    .join(expected["file"].as_str().unwrap()),
            )
            .unwrap()
            .to_rgba8();
            if (actual.width, actual.height)
                != (expected.width() as usize, expected.height() as usize)
            {
                failures.push(format!(
                    "{input}: dimensions {}x{} vs {}x{}",
                    actual.width,
                    actual.height,
                    expected.width(),
                    expected.height()
                ));
            } else {
                let count = actual
                    .rgba
                    .iter()
                    .zip(expected.as_raw())
                    .filter(|(a, b)| a != b)
                    .count();
                if count != 0 {
                    failures.push(format!("{input}: {count} differing bytes"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
