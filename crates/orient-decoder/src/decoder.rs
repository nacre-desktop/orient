use crate::{fail, Result};
use image::{AnimationDecoder, DynamicImage, ImageDecoder, ImageReader};
use nix::{
    sys::signal::{killpg, Signal},
    unistd::Pid,
};
use orient_core::extract::Frame;
use serde_json::Value;
use std::os::unix::process::CommandExt;
use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
pub const MAX_PIXELS: u64 = 50_000_000;
pub(crate) fn check(width: u32, height: u32) -> Result<()> {
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        fail("Image exceeds the 50 megapixel safety limit")
    } else {
        Ok(())
    }
}
fn normalize(image: DynamicImage, profile: Option<&[u8]>, nearest: bool) -> Result<Frame> {
    check(image.width(), image.height())?;
    let alpha = image.color().has_alpha();
    let mut rgba = crate::resample::thumbnail(image.to_rgba8(), alpha, nearest);
    if let Some(profile) = profile {
        let convert = || -> std::result::Result<Vec<[u8; 3]>, lcms2::Error> {
            let input = lcms2::Profile::new_icc(profile)?;
            let output = lcms2::Profile::new_srgb();
            let transform = lcms2::Transform::<[u8; 3], [u8; 3]>::new(
                &input,
                lcms2::PixelFormat::RGB_8,
                &output,
                lcms2::PixelFormat::RGB_8,
                lcms2::Intent::Perceptual,
            )?;
            let pixels: Vec<[u8; 3]> = rgba.pixels().map(|p| [p[0], p[1], p[2]]).collect();
            let mut colors = vec![[0; 3]; pixels.len()];
            transform.transform_pixels(&pixels, &mut colors);
            Ok(colors)
        };
        let colors =
            convert().map_err(|_| crate::error("Invalid or unsupported image color profile"))?;
        for (p, c) in rgba.pixels_mut().zip(colors) {
            p[0] = c[0];
            p[1] = c[1];
            p[2] = c[2];
        }
    }
    Ok(Frame {
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        rgba: rgba.into_raw(),
    })
}
fn nearest_mode(path: &Path, format: Option<image::ImageFormat>) -> Result<bool> {
    let mut header = [0u8; 64];
    let mut file = File::open(path)?;
    let count = file.read(&mut header)?;
    Ok(match format {
        Some(image::ImageFormat::Png) if count >= 26 => {
            header[25] == 3 || (header[25] == 0 && header[24] == 1)
        }
        Some(image::ImageFormat::Gif) => true,
        Some(image::ImageFormat::Bmp) if count >= 30 => {
            let size = u32::from_le_bytes(header[14..18].try_into().unwrap());
            let bits = if size == 12 {
                u16::from_le_bytes(header[24..26].try_into().unwrap())
            } else {
                u16::from_le_bytes(header[28..30].try_into().unwrap())
            };
            if bits == 8 {
                let stride = if size == 12 { 3 } else { 4 };
                let used = if size >= 40 {
                    u32::from_le_bytes(header[46..50].try_into().unwrap())
                } else {
                    0
                };
                let colors = if used == 0 { 256 } else { used.min(256) };
                let mut palette = vec![0; colors as usize * stride];
                file.seek(SeekFrom::Start(14 + u64::from(size)))?;
                file.read_exact(&mut palette)?;
                !palette
                    .chunks_exact(stride)
                    .enumerate()
                    .all(|(i, p)| p[0] == i as u8 && p[1] == i as u8 && p[2] == i as u8)
            } else {
                bits == 1 || bits == 4
            }
        }
        _ => false,
    })
}
pub fn still(path: &Path) -> Result<Frame> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    let format = reader.format();
    let nearest = nearest_mode(path, format)?;
    let mut decoder = reader.into_decoder()?;
    let (width, height) = decoder.dimensions();
    check(width, height)?;
    let ty = decoder.color_type();
    if ty.bits_per_pixel() / u16::from(ty.channel_count()) > 8 {
        return fail("Convert HDR/high-bit-depth wallpaper to sRGB first");
    }
    let profile = decoder.icc_profile()?;
    let orientation = decoder.orientation()?;
    let mut image = if format == Some(image::ImageFormat::Jpeg) {
        match crate::jpeg::decode(path)? {
            Some(image) => image,
            None => DynamicImage::from_decoder(decoder)?,
        }
    } else {
        DynamicImage::from_decoder(decoder)?
    };
    image.apply_orientation(orientation);
    normalize(image, profile.as_deref(), nearest)
}
fn gif(path: &Path) -> Result<Vec<Frame>> {
    let decoder = image::codecs::gif::GifDecoder::new(BufReader::new(File::open(path)?))?;
    let (width, height) = decoder.dimensions();
    check(width, height)?;
    // Pillow promotes later palette frames to RGB, or RGBA when the first
    // frame declares transparency. That distinction controls integer reduction.
    let mut metadata = gif::DecodeOptions::new().read_info(BufReader::new(File::open(path)?))?;
    let first = metadata.next_frame_info()?.cloned();
    let transparent = first.as_ref().and_then(|frame| frame.transparent);
    let alpha = transparent.is_some();
    let palette = first
        .as_ref()
        .and_then(|frame| frame.palette.as_deref())
        .or_else(|| metadata.global_palette());
    let hidden = transparent.and_then(|index| {
        let offset = usize::from(index) * 3;
        palette.and_then(|colors| colors.get(offset..offset + 3))
    });
    let count = decoder
        .into_frames()
        .try_fold(0_usize, |count, frame| -> Result<usize> {
            let f = frame?;
            check(f.buffer().width(), f.buffer().height())?;
            Ok(count + 1)
        })?;
    if count == 0 {
        return fail("Wallpaper has no visible pixels");
    }
    let mut indexes: Vec<_> = [0.0, 0.33, 0.67, 1.0]
        .into_iter()
        .map(|f| (((count - 1) as f64 * f).round_ties_even() as usize).min(count - 1))
        .collect();
    indexes.sort_unstable();
    indexes.dedup();
    let decoder = image::codecs::gif::GifDecoder::new(BufReader::new(File::open(path)?))?;
    let (width, height) = decoder.dimensions();
    check(width, height)?;
    let mut result = Vec::new();
    for (index, frame) in decoder.into_frames().enumerate() {
        let frame = frame?;
        if indexes.contains(&index) {
            let mut image = DynamicImage::ImageRgba8(frame.into_buffer());
            if index == 0 {
                if let Some(hidden) = hidden {
                    for pixel in image.as_mut_rgba8().unwrap().pixels_mut() {
                        if pixel[3] == 0 {
                            pixel.0[..3].copy_from_slice(hidden);
                        }
                    }
                }
            }
            if index != 0 && !alpha {
                image = DynamicImage::ImageRgb8(image.into_rgb8());
            }
            result.push(normalize(image, None, index == 0)?);
        }
    }
    Ok(result)
}
/// Concurrent pipe reads prevent deadlock; timeout kills and reaps the whole group.
pub fn subprocess(command: &mut Command, timeout: Duration) -> Result<Vec<u8>> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                crate::error("Animated video analysis requires ffmpeg and ffprobe")
            } else {
                e.into()
            }
        })?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let output = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let errors = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });
    let started = Instant::now();
    let mut exit_status = None;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => exit_status = Some(status),
            Ok(None) => {}
            Err(e) => {
                let _ = killpg(Pid::from_raw(child.id() as i32), Signal::SIGKILL);
                let _ = child.wait();
                break Err(e.into());
            }
        }
        if let Some(status) = exit_status {
            if output.is_finished() && errors.is_finished() {
                break Ok(status);
            }
        }
        if crate::CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = killpg(Pid::from_raw(child.id() as i32), Signal::SIGKILL);
            let _ = child.wait();
            break Err(crate::error("Animation decoder cancelled"));
        }
        if started.elapsed() >= timeout {
            let _ = killpg(Pid::from_raw(child.id() as i32), Signal::SIGKILL);
            let _ = child.wait();
            break Err(crate::error("Animation decoder timed out"));
        }
        thread::sleep(Duration::from_millis(5));
    };
    let out = output
        .join()
        .map_err(|_| crate::error("Animation decoder failed"))??;
    let err = errors
        .join()
        .map_err(|_| crate::error("Animation decoder failed"))??;
    let status = status?;
    if !status.success() {
        let text = String::from_utf8_lossy(&err);
        let tail: String = text
            .chars()
            .rev()
            .take(300)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        return fail(&format!("Animation decoder failed: {tail}"));
    }
    Ok(out)
}
pub fn is_video(path: &Path) -> bool {
    path.extension().and_then(|v| v.to_str()).is_some_and(|v| {
        ["mp4", "mkv", "webm", "mov", "avi", "m4v"].contains(&v.to_lowercase().as_str())
    })
}
fn video(path: &Path) -> Result<Vec<Frame>> {
    let metadata: Value = serde_json::from_slice(&subprocess(
        Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration:stream=codec_type,width,height,avg_frame_rate",
                "-of",
                "json",
            ])
            .arg(path),
        Duration::from_secs(10),
    )?)?;
    let streams: Vec<_> = metadata["streams"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| s["codec_type"] == "video")
        .collect();
    if streams.is_empty()
        || streams.iter().any(|s| {
            s["width"].as_u64().unwrap_or(0) * s["height"].as_u64().unwrap_or(0) > MAX_PIXELS
        })
    {
        return fail("Animation has no supported video stream or exceeds the pixel limit");
    }
    let duration = metadata["format"]["duration"]
        .as_str()
        .unwrap_or("")
        .parse::<f64>()?;
    if !duration.is_finite() || duration <= 0.0 {
        return fail("Video has no usable duration");
    }
    let rate = streams[0]["avg_frame_rate"].as_str().unwrap_or("30/1");
    let (n, d) = rate.split_once('/').unwrap_or(("30", "1"));
    let denominator = d.parse::<f64>()?;
    let rate = if denominator == 0.0 {
        30.0
    } else {
        n.parse::<f64>()? / denominator
    };
    let rate = if rate.is_finite() && rate > 0.0 {
        rate
    } else {
        30.0
    };
    let last = (duration - 1.5 / rate).max(0.0);
    let folder = tempfile::tempdir()?;
    let mut result = Vec::new();
    for (i, fraction) in [0.12, 0.38, 0.62, 0.87].into_iter().enumerate() {
        let target = folder.path().join(format!("{i}.png"));
        subprocess(
            Command::new("ffmpeg")
                .args(["-v", "error", "-ss"])
                .arg((duration * fraction).min(last).to_string())
                .arg("-i")
                .arg(path)
                .args([
                    "-frames:v",
                    "1",
                    "-vf",
                    "scale=128:128:force_original_aspect_ratio=decrease",
                    "-y",
                ])
                .arg(&target),
            Duration::from_secs(20),
        )?;
        if !target.is_file() {
            return fail(
                "Video decoder produced no sample; convert this clip to a supported format",
            );
        }
        result.push(still(&target)?);
    }
    Ok(result)
}
pub fn frames(path: &Path) -> Result<Vec<Frame>> {
    if is_video(path) {
        video(path)
    } else if path
        .extension()
        .is_some_and(|v| v.eq_ignore_ascii_case("gif"))
    {
        gif(path)
    } else {
        Ok(vec![still(path)?])
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deadline_reaps_descendants() {
        let folder = tempfile::tempdir().unwrap();
        let pidfile = folder.path().join("child");
        let start = Instant::now();
        let result = subprocess(
            Command::new("sh")
                .args(["-c", "sleep 30 & echo $! > \"$1\"; wait", "--"])
                .arg(&pidfile),
            Duration::from_millis(200),
        );
        assert!(result.unwrap_err().to_string().contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(3));
        let pid = std::fs::read_to_string(pidfile).unwrap();
        // A killed descendant can close its pipes before Linux reports Z.
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            match std::fs::read_to_string(format!("/proc/{}/status", pid.trim())) {
                Ok(status) => {
                    if status
                        .lines()
                        .any(|line| line.starts_with("State:") && line.contains('Z'))
                    {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
                Err(e) => panic!("cannot inspect descendant: {e}"),
            }
            assert!(Instant::now() < deadline, "descendant is still running");
            thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn deadline_covers_pipes_after_parent_exit() {
        let start = Instant::now();
        let result = subprocess(
            Command::new("sh").args(["-c", "sleep 30 & exit 0"]),
            Duration::from_millis(200),
        );
        assert!(result.unwrap_err().to_string().contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(3));
    }
}
