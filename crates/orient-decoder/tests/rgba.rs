use orient_decoder::{Filter, Result};
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
fn rgba_operations_match_frozen_pillow_pixels_and_png_bytes() -> Result<()> {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/rgba-operations-reference.json"))?;
    for case in fixture["cases"].as_array().ok_or("Missing cases")? {
        let input = &fixture["inputs"][case["input"].as_str().ok_or("Missing input")?];
        let image = image::RgbaImage::from_raw(
            input["size"][0].as_u64().ok_or("Missing width")? as u32,
            input["size"][1].as_u64().ok_or("Missing height")? as u32,
            decode(input["rgba"].as_str().ok_or("Missing input pixels")?),
        )
        .ok_or("Invalid input pixels")?;
        let actual = match case["operation"].as_str().ok_or("Missing operation")? {
            "encode" => orient_decoder::encode_rgba_png(
                &image,
                case["level"].as_u64().ok_or("Missing level")? as u8,
            )?,
            "resize" => orient_decoder::resize_rgba(
                &image,
                (
                    case["size"][0].as_u64().ok_or("Missing width")? as u32,
                    case["size"][1].as_u64().ok_or("Missing height")? as u32,
                ),
                if case["filter"] == "LANCZOS" {
                    Filter::Lanczos
                } else {
                    Filter::Bicubic
                },
            )?
            .into_raw(),
            "blur" => orient_decoder::blur_rgba(
                &image,
                case["radius"].as_f64().ok_or("Missing radius")? as f32,
            )?
            .into_raw(),
            _ => return Err("Unknown fixture operation".into()),
        };
        let expected = decode(
            case[if case["operation"] == "encode" {
                "png"
            } else {
                "rgba"
            }]
            .as_str()
            .ok_or("Missing expected bytes")?,
        );
        assert!(
            actual == expected,
            "RGBA mismatch: {}; bytes {} vs {}, differing {}",
            case["name"],
            actual.len(),
            expected.len(),
            actual
                .iter()
                .zip(expected.iter())
                .filter(|(a, b)| a != b)
                .count()
        );
    }
    Ok(())
}
#[test]
fn invalid_rgba_bounds_radius_and_compression_fail_without_output() {
    let empty = image::RgbaImage::new(0, 0);
    let image = image::RgbaImage::new(1, 1);
    assert!(orient_decoder::encode_rgba_png(&empty, 6).is_err());
    assert!(orient_decoder::encode_rgba_png(&image, 10).is_err());
    assert!(orient_decoder::resize_rgba(&image, (0, 1), Filter::Lanczos).is_err());
    assert!(orient_decoder::resize_rgba(&image, (50001, 1000), Filter::Lanczos).is_err());
    for radius in [f32::NAN, f32::INFINITY, -1.0, 1001.0] {
        assert!(orient_decoder::blur_rgba(&image, radius).is_err());
    }
}
