use orient_decoder::Result;
use serde_json::Value;
fn decode(text: &str) -> Vec<u8> {
    let mut bits = 0u32;
    let mut count = 0;
    let mut output = Vec::new();
    for byte in text.bytes().take_while(|byte| *byte != b'=') {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        };
        bits = (bits << 6) | u32::from(value);
        count += 6;
        if count >= 8 {
            count -= 8;
            output.push((bits >> count) as u8);
        }
    }
    output
}
#[test]
fn rgb_previews_match_frozen_pillow_png_bytes() -> Result<()> {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/rgb-preview-reference.json"))?;
    let root = tempfile::tempdir()?;
    for case in fixture["cases"].as_array().ok_or("Missing cases")? {
        let name = case["name"].as_str().ok_or("Missing name")?;
        let path = root.path().join(name);
        std::fs::write(
            &path,
            decode(fixture["inputs"][name].as_str().ok_or("Missing input")?),
        )?;
        let output = orient_decoder::rgb_preview_png(&path, (2560, 1600), 10.0)?;
        let expected = decode(case["png"].as_str().ok_or("Missing expected PNG")?);
        if output != expected {
            let actual = image::load_from_memory(&output)?.to_rgb8();
            let reference = image::load_from_memory(&expected)?.to_rgb8();
            assert_eq!(actual.dimensions(), reference.dimensions(), "{name}");
            let differences = actual
                .as_raw()
                .iter()
                .zip(reference.as_raw())
                .filter(|(a, b)| a != b)
                .count();
            panic!(
                "{name}: PNG mismatch, RGB differences={differences}, sizes {}/{}",
                output.len(),
                expected.len()
            );
        }
    }
    Ok(())
}

#[test]
fn gif_canvas_and_transparent_palette_colors_match_pillow() -> Result<()> {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/rgb-preview-gif-edges.json"))?;
    let root = tempfile::tempdir()?;
    for case in fixture["cases"].as_array().ok_or("Missing GIF cases")? {
        let name = case["name"].as_str().ok_or("Missing name")?;
        let path = root.path().join(name);
        std::fs::write(
            &path,
            decode(case["input"].as_str().ok_or("Missing input")?),
        )?;
        let output = orient_decoder::rgb_preview_png(&path, (2560, 1600), 10.0)?;
        let expected = decode(case["png"].as_str().ok_or("Missing PNG")?);
        assert_eq!(output, expected, "{name}");
    }
    Ok(())
}
