"""Bounded perceptual clustering with true coverage and separate salience."""

import io
import math
from pathlib import Path
import subprocess
import os
import signal
import tempfile
import warnings
import json

from PIL import Image, ImageCms, ImageOps
from ._bridge import analyze_frames
from . import _core

SAMPLE_EDGE = 128
MAX_PIXELS = 50_000_000
GRID = (12, 8)
VIDEO = {".mp4", ".mkv", ".webm", ".mov", ".avi", ".m4v"}


def normalize(path, frame=0):
    with warnings.catch_warnings():
        warnings.simplefilter("error", Image.DecompressionBombWarning)
        with Image.open(path) as original:
            if original.width * original.height > MAX_PIXELS:
                raise ValueError("Image exceeds the 50 megapixel safety limit")
            if original.mode in ("F", "I", "I;16"):
                raise ValueError("Convert HDR/high-bit-depth wallpaper to sRGB first")
            original.seek(frame)
            original.draft("RGB", (512, 512))
            image = ImageOps.exif_transpose(original)
            image.thumbnail((SAMPLE_EDGE, SAMPLE_EDGE), Image.Resampling.LANCZOS)
            alpha = image.convert("RGBA").getchannel("A")
            profile = original.info.get("icc_profile")
            if profile:
                try:
                    source = ImageCms.ImageCmsProfile(io.BytesIO(profile))
                    image = ImageCms.profileToProfile(
                        image.convert("RGB") if image.mode not in ("RGB", "CMYK", "LAB") else image,
                        source,
                        ImageCms.createProfile("sRGB"),
                        outputMode="RGB",
                    )
                except (ImageCms.PyCMSError, OSError, ValueError) as error:
                    raise ValueError("Invalid or unsupported image color profile") from error
            else:
                image = image.convert("RGB")
            image.putalpha(alpha)
            return image


def decoder(command, timeout=20):
    """Bound external work; cooperative cancellation reaps its process group."""
    process = subprocess.Popen(
        command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=os.name == "posix"
    )
    try:
        output, error = process.communicate(timeout=timeout)
        if process.returncode:
            raise ValueError("Animation decoder failed: " + error.decode(errors="replace")[-300:])
        return output
    except BaseException:
        if process.poll() is None:
            if os.name == "posix":
                os.killpg(process.pid, signal.SIGKILL)
            else:
                process.kill()
            process.wait()
        raise


def frames(path):
    """Four fixed temporal samples; decoding occurs only on cache misses."""
    if path.suffix.lower() in VIDEO:
        try:
            metadata = json.loads(
                decoder(
                    [
                        "ffprobe",
                        "-v",
                        "error",
                        "-show_entries",
                        "format=duration:stream=codec_type,width,height,avg_frame_rate",
                        "-of",
                        "json",
                        str(path),
                    ],
                    timeout=10,
                )
            )
            videos = [stream for stream in metadata.get("streams", []) if stream.get("codec_type") == "video"]
            if not videos or any(
                int(stream.get("width", 0)) * int(stream.get("height", 0)) > MAX_PIXELS for stream in videos
            ):
                raise ValueError("Animation has no supported video stream or exceeds the pixel limit")
            duration = float(metadata["format"]["duration"])
            if not math.isfinite(duration) or duration <= 0:
                raise ValueError("Video has no usable duration")
            rate_text = videos[0].get("avg_frame_rate", "30/1")
            numerator, denominator = rate_text.split("/")
            rate = float(numerator) / float(denominator) if float(denominator) else 30.0
            rate = rate if math.isfinite(rate) and rate > 0 else 30.0
            last_sample = max(0.0, duration - 1.5 / rate)
            result = []
            with tempfile.TemporaryDirectory(prefix="orient-frames-") as folder:
                for i, fraction in enumerate((0.12, 0.38, 0.62, 0.87)):
                    target = Path(folder) / f"{i}.png"
                    decoder(
                        [
                            "ffmpeg",
                            "-v",
                            "error",
                            "-ss",
                            str(min(duration * fraction, last_sample)),
                            "-i",
                            str(path),
                            "-frames:v",
                            "1",
                            "-vf",
                            "scale=128:128:force_original_aspect_ratio=decrease",
                            "-y",
                            str(target),
                        ],
                        timeout=20,
                    )
                    if not target.is_file():
                        raise ValueError("Video decoder produced no sample; convert this clip to a supported format")
                    result.append(normalize(target))
            return result
        except FileNotFoundError as error:
            raise ValueError("Animated video analysis requires ffmpeg and ffprobe") from error
    with Image.open(path) as image:
        count = getattr(image, "n_frames", 1)
    indexes = sorted({min(count - 1, round((count - 1) * f)) for f in (0, 0.33, 0.67, 1)})
    return [normalize(path, i) for i in indexes]


def analyze(path):
    path = Path(path).expanduser().resolve(strict=True)
    return analyze_frames(frames(path))


def sample_color(path, x, y):
    if not all(isinstance(v, (int, float)) and math.isfinite(v) and 0 <= v <= 1 for v in (x, y)):
        raise ValueError("Pick coordinates between zero and one")
    image = normalize(Path(path)) if Path(path).suffix.lower() not in VIDEO else frames(Path(path))[0]
    return _core.sample_pixel(image.width, image.height, image.tobytes(), x, y)
