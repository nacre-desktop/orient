"""Report all role deltas between Pillow and the standalone Rust decoder."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

from PIL import Image
from orient.engine import from_image
from orient.extract import frames
from orient.accessibility import difference
from orient.palette import PERSONALITIES, generate

ROOT = Path(__file__).parents[2]


def separation_diagnostics(palette):
    settings = palette["input"]
    source = palette["source"]
    variant = settings["variant"]
    if (
        source["neutral"]
        and not settings["accent"]
        and settings["personality"] not in ("pearl", "vivid")
        and variant not in ("neutral", "monochrome")
    ):
        variant = "neutral"
    # Explicit support-role overrides preserve their source pigments and disable
    # the same small-distance separation branch, exposing its pre-branch values.
    overrides = dict(
        settings["overrides"], secondary=palette["colours"]["orient2"], tertiary=palette["colours"]["orient3"]
    )
    colors = generate(
        source["selected"],
        palette["mode"],
        variant,
        settings["flavour"],
        source["candidates"],
        harmony=settings["harmony"],
        body=source["body"],
        personality=settings["personality"],
        overrides=overrides,
        brightness=settings["brightness"],
        saturation=settings["saturation"],
        hour=settings["hour"],
        background_from_wallpaper=settings["background_from_wallpaper"],
        time_tint=settings["time_tint"],
    )
    result = {}
    for role in ("secondary", "tertiary"):
        previous = [palette["colours"]["primary"]]
        if role == "tertiary":
            previous.append(palette["colours"]["secondary"])
        minimum = min(
            difference(colors[role], value, vision)
            for value in previous
            for vision in ("normal", "protanopia", "deuteranopia", "tritanopia")
        )
        result[role] = {
            "pre_separation_hex": colors[role],
            "final_hex": palette["colours"][role],
            "minimum_pre_separation_distance": minimum,
            "below_0_045": minimum < 0.045,
        }
    return result


def compare(binary, dump_binary, output):
    rows = []
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        paths = sorted((ROOT / "tests/orient-gallery/images").iterdir())
        # Public fixture derivatives exercise codecs absent from the gallery.
        source = Image.open(paths[0]).convert("RGB")
        for name, options in (
            ("coast-day-lossless.webp", {"lossless": True}),
            ("coast-day-lossy.webp", {"quality": 80}),
            ("coast-day.jpg", {"quality": 85}),
        ):
            path = root / name
            source.save(path, **options)
            paths.append(path)
        for path in paths:
            maxima = {}
            cases = []
            changed = set()
            for mode in ("dark", "light"):
                for personality in PERSONALITIES:
                    python = from_image(path, mode=mode, personality=personality, cache_dir=root / "python-cache")
                    rust = json.loads(
                        subprocess.check_output(
                            [str(binary), "palette", str(path), "--mode", mode, "--personality", personality],
                            env={**os.environ, "XDG_CACHE_HOME": str(root / "rust-cache")},
                            text=True,
                        )
                    )
                    deltas = {
                        role: difference(value, rust["colours"][role]) for role, value in python["colours"].items()
                    }
                    for role, delta in deltas.items():
                        maxima[role] = max(maxima.get(role, 0.0), delta)
                    differing = [role for role, value in python["colours"].items() if value != rust["colours"][role]]
                    changed.update(differing)
                    cases.append(
                        {
                            "mode": mode,
                            "personality": personality,
                            "changed_role_count": len(differing),
                            "max_oklab_delta": max(deltas.values()),
                        }
                    )
                    if mode == "dark" and max(deltas.values()) > 0.02:
                        cases[-1]["separation_threshold"] = {
                            "python": separation_diagnostics(python),
                            "rust": separation_diagnostics(rust),
                        }
            diagnosis = {}
            if dump_binary:
                dump = root / path.stem
                subprocess.run([str(dump_binary), str(path), str(dump)], check=True)
                pillow = frames(path)
                rust_frames = [Image.open(dump / f"{i}.png").convert("RGBA") for i in range(len(pillow))]
                diagnosis["normalized_frames_equal"] = all(
                    a.size == b.size and a.tobytes() == b.tobytes() for a, b in zip(pillow, rust_frames, strict=True)
                )
                raw = dump / "raw.png"
                if raw.exists():
                    original = Image.open(path).convert("RGBA")
                    decoded = Image.open(raw).convert("RGBA")
                    raw_equal = original.size == decoded.size and original.tobytes() == decoded.tobytes()
                    diagnosis["raw_pixels_equal"] = raw_equal
                    diagnosis["resize_active"] = max(original.size) > 128
                    diagnosis["icc_active"] = bool(Image.open(path).info.get("icc_profile"))
                    diagnosis["exif_active"] = bool(Image.open(path).getexif().get(274))
                    if (
                        raw_equal
                        and not diagnosis["normalized_frames_equal"]
                        and not diagnosis["icc_active"]
                        and not diagnosis["exif_active"]
                    ):
                        diagnosis["cause"] = "Lanczos thumbnail implementation (raw decoded pixels identical)"
                else:
                    diagnosis["cause"] = (
                        "none" if diagnosis["normalized_frames_equal"] else "GIF frame composition or thumbnail"
                    )
                if path.suffix == ".jpg" and not diagnosis.get("raw_pixels_equal", True):
                    diagnosis["cause"] = (
                        "none; raw diagnostic uses image codec, normalization uses libjpeg-turbo"
                        if diagnosis["normalized_frames_equal"]
                        else "JPEG decoder pixels / Pillow draft differ"
                    )
            row = {
                "image": path.name,
                "cases": cases,
                "changed_roles_any_case": len(changed),
                "max_delta_by_role": maxima,
                "max_oklab_delta": max(maxima.values()),
                "diagnosis": diagnosis,
            }
            rows.append(row)
            print(
                f"{path.name}: {len(changed)} roles changed across 16 cases; max OKLab delta {row['max_oklab_delta']:.6f}; {diagnosis.get('cause', '')}"
            )
    output.write_text(json.dumps(rows, indent=2) + "\n")
    failures = [
        row["image"]
        for row in rows
        if not row["image"].endswith(".jpg")
        and (row["changed_roles_any_case"] or row["diagnosis"].get("normalized_frames_equal") is False)
    ]
    if failures:
        raise AssertionError("Non-JPEG decoder parity failed: " + ", ".join(failures))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--dump-binary", type=Path)
    parser.add_argument("--output", type=Path, default=Path(__file__).with_name("decoder-report.json"))
    args = parser.parse_args()
    compare(args.binary.resolve(), args.dump_binary.resolve() if args.dump_binary else None, args.output)
