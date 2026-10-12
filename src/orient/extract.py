"""Bounded perceptual clustering with true coverage and separate salience."""

from collections import Counter
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
from .colour import color, lab_from_rgb, lch

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


def distance(a, b, light_weight=1.0):
    return math.sqrt((a[0] - b[0]) ** 2 * light_weight + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2)


def pigment_families(records):
    """Aggregate shade populations before ranking; retain fine clusters separately.

    Compare chromatic OKLab directions at a common strength so a shadow and
    highlight of one pigment share coverage. Complete-link membership prevents
    chains of neighboring hues from swallowing a genuinely different family.
    """
    groups = []
    for record in sorted(records, key=lambda r: (-r["coverage"], r["hex"])):
        if record["chroma"] < 0.012 or not 0.12 < record["lightness"] < 0.96:
            continue
        light, chroma, hue = lch(record["hex"])
        angle = math.radians(hue)
        vector = (0.0, 0.1 * math.cos(angle), 0.1 * math.sin(angle))
        compatible = [group for group in groups if all(distance(vector, v) < 0.028 for v in group["vectors"])]
        group = compatible[0] if compatible else {"members": [], "vectors": []}
        if not compatible:
            groups.append(group)
        group["members"].append(record)
        group["vectors"].append(vector)
    families = []
    for group in groups:
        members = group["members"]
        coverage = sum(r["coverage"] for r in members)
        if coverage < 0.0025:
            continue
        pigment = [r["coverage"] * (0.25 + 0.75 * min(r["lightness"] / 0.65, 1)) for r in members]
        mass = sum(pigment)
        labs = [lab_from_rgb(tuple(int(r["hex"][k : k + 2], 16) / 255 for k in (0, 2, 4))) for r in members]
        light, a, b = (sum(lab[k] * w for lab, w in zip(labs, pigment, strict=True)) / mass for k in range(3))
        chroma = math.hypot(a, b)
        salience = sum(r["salience"] * r["coverage"] for r in members) / coverage
        cells = Counter()
        for r in members:
            for cell in r["regions"]["cells"]:
                cells[cell["index"]] += cell["coverage"]
        score = coverage**0.7 * min(chroma / 0.12, 1.5) ** 0.8 * (0.5 + 0.5 * min(light / 0.55, 1)) * salience
        families.append(
            {
                "hex": color(light, chroma, math.degrees(math.atan2(b, a))),
                "coverage": round(coverage, 6),
                "chroma": round(chroma, 6),
                "lightness": round(light, 6),
                "salience": round(salience, 4),
                "score": round(score, 7),
                "spread": round(len(cells) / (GRID[0] * GRID[1]), 4),
                "location": {
                    k: round(sum(r["location"][k] * r["coverage"] for r in members) / coverage, 4) for k in ("x", "y")
                },
                "regions": {
                    "columns": GRID[0],
                    "rows": GRID[1],
                    "cells": [{"index": i, "coverage": round(w, 6)} for i, w in cells.most_common()],
                },
                "members": [r["hex"] for r in members],
            }
        )
    return sorted(families, key=lambda r: (-r["score"], r["hex"]))


def analyze(path):
    path = Path(path).expanduser().resolve(strict=True)
    images = frames(path)
    bins = {}
    total = 0.0
    for image in images:
        pixels = list((image.get_flattened_data() if hasattr(image, "get_flattened_data") else image.getdata()))
        for i, (r, g, b, alpha) in enumerate(pixels):
            if alpha == 0:
                continue
            weight = alpha / 255 / len(images)
            x, y = i % image.width, i // image.width
            nx, ny = (x + 0.5) / image.width, (y + 0.5) / image.height
            center = math.exp(-(((nx - 0.5) / 0.42) ** 2 + ((ny - 0.5) / 0.42) ** 2))
            detail = 0.0
            for j in (i + 1 if x + 1 < image.width else i, i + image.width if y + 1 < image.height else i):
                detail += sum(abs(pixels[j][k] - v) for k, v in enumerate((r, g, b))) / 1530
            salience = 1 + 0.28 * center + min(0.45, detail * 2.5)
            key = (r >> 3, g >> 3, b >> 3)
            record = bins.setdefault(
                key, {"weight": 0.0, "rgb": [0.0, 0.0, 0.0], "salience": 0.0, "cells": Counter(), "xy": [0.0, 0.0]}
            )
            record["weight"] += weight
            record["salience"] += salience * weight
            for k, value in enumerate((r, g, b)):
                record["rgb"][k] += value * weight
            record["xy"][0] += nx * weight
            record["xy"][1] += ny * weight
            record["cells"][min(GRID[0] - 1, int(nx * GRID[0])) + min(GRID[1] - 1, int(ny * GRID[1])) * GRID[0]] += (
                weight
            )
            total += weight
    if not total:
        raise ValueError("Wallpaper has no visible pixels")
    groups = []
    # Fixed ordering, full perceptual coordinates, and bounded threshold; no hue bins.
    for record in sorted(bins.values(), key=lambda r: (-r["weight"], r["rgb"])):
        weight = record["weight"]
        lab = lab_from_rgb(tuple(v / weight / 255 for v in record["rgb"]))

        def pigment_direction(value):
            strength = math.hypot(value[1], value[2])
            return (0.0, value[1] * 0.1 / strength, value[2] * 0.1 / strength)

        def compatible(other, lab=lab):
            chromatic = math.hypot(lab[1], lab[2]) >= 0.012
            other_chromatic = math.hypot(other[1], other[2]) >= 0.012
            if chromatic != other_chromatic:
                return False
            return not chromatic or distance(pigment_direction(lab), pigment_direction(other)) < 0.028

        eligible = [(distance(lab, g["lab"], 0.45), g) for g in groups if compatible(g["lab"])]
        best = min(eligible, key=lambda item: item[0]) if eligible else None
        limit = max(0.012, min(0.045, math.hypot(lab[1], lab[2]) * 0.4))
        if best and best[0] < limit:
            group = best[1]
        else:
            group = {
                "weight": 0.0,
                "sum": [0.0, 0.0, 0.0],
                "salience": 0.0,
                "cells": Counter(),
                "xy": [0.0, 0.0],
                "lab": lab,
            }
            groups.append(group)
        group["weight"] += weight
        group["salience"] += record["salience"]
        group["cells"].update(record["cells"])
        for k in range(3):
            group["sum"][k] += lab[k] * weight
        for k in range(2):
            group["xy"][k] += record["xy"][k]
        group["lab"] = tuple(v / group["weight"] for v in group["sum"])
    records = []
    for group in groups:
        light, a, b = group["lab"]
        chroma = math.hypot(a, b)
        hue = math.degrees(math.atan2(b, a)) % 360
        coverage = group["weight"] / total
        salience = group["salience"] / group["weight"]
        score = coverage**0.7 * min(chroma / 0.12, 1.5) ** 0.8 * (0.5 + 0.5 * min(light / 0.55, 1)) * salience
        records.append(
            {
                "hex": color(light, chroma, hue),
                "coverage": round(coverage, 6),
                "chroma": round(chroma, 6),
                "score": round(score, 7),
                "salience": round(salience, 4),
                "lightness": round(light, 6),
                "spread": round(len(group["cells"]) / (GRID[0] * GRID[1]), 4),
                "location": {
                    "x": round(group["xy"][0] / group["weight"], 4),
                    "y": round(group["xy"][1] / group["weight"], 4),
                },
                "regions": {
                    "columns": GRID[0],
                    "rows": GRID[1],
                    "cells": [
                        {"index": i, "coverage": round(w / total, 6)}
                        for i, w in group["cells"].most_common(GRID[0] * GRID[1])
                    ],
                },
            }
        )
    records.sort(key=lambda r: (-r["score"], r["hex"]))
    families = pigment_families(records)
    candidates = []
    # Reserve options for different families before offering neighboring tones.
    for separation in (48, 18):
        for record in families:
            hue = lch(record["hex"])[2]
            if all(abs((hue - lch(c["hex"])[2] + 180) % 360 - 180) >= separation for c in candidates):
                candidates.append(record)
            if len(candidates) == 5:
                break
        if len(candidates) == 5:
            break
    neutral = not candidates
    if neutral:
        dominant = max(records, key=lambda r: r["coverage"])
        value = dominant["hex"]
        candidates = [dict(dominant, hex=value, chroma=0.0, score=0.0)]
    ordered = sorted(records, key=lambda r: r["lightness"])

    def population_tone(low):
        walk = ordered if low else ordered[::-1]
        selected, mass = [], 0.0
        for item in walk:
            selected.append(item)
            mass += item["coverage"]
            if mass >= 0.25:
                break
        return max(selected, key=lambda r: r["coverage"])

    return {
        "seed": candidates[0]["hex"],
        "candidates": candidates,
        "families": families,
        "clusters": sorted(records, key=lambda r: -r["coverage"])[:64],
        "body": {"dark": population_tone(True), "light": population_tone(False)},
        "meanLightness": round(sum(r["lightness"] * r["coverage"] for r in records), 6),
        "neutral": neutral,
        "sample": {"width": images[0].width, "height": images[0].height, "frames": len(images)},
        "reason": "perceptual shade clusters; aggregated pigment-family coverage with bounded salience",
    }


def sample_color(path, x, y):
    if not all(isinstance(v, (int, float)) and math.isfinite(v) and 0 <= v <= 1 for v in (x, y)):
        raise ValueError("Pick coordinates between zero and one")
    image = normalize(Path(path)) if Path(path).suffix.lower() not in VIDEO else frames(Path(path))[0]
    r, g, b, alpha = image.getpixel(
        (min(image.width - 1, int(x * image.width)), min(image.height - 1, int(y * image.height)))
    )
    if not alpha:
        raise ValueError("The selected point is transparent")
    return f"{r:02x}{g:02x}{b:02x}"
