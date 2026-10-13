"""Freeze the Python reference before porting. Never regenerate to bless a Rust mismatch."""

import argparse
from copy import deepcopy
import gzip
import importlib.util
import json
from pathlib import Path
import random
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
from PIL import Image
from orient import ENGINE_ID, __version__
from orient import classic, engine, export, extract, palette, template_engine

SEED = 8675309


def scrub(value):
    if isinstance(value, dict):
        return {k: scrub(v) for k, v in value.items() if k not in ("engine", "path")}
    if isinstance(value, (list, tuple)):
        return [scrub(v) for v in value]
    return value


def pack(value):
    """Intern repeated structures losslessly before gzip (its window is only 32 KiB)."""
    pool, indexes = [], {}

    def visit(item):
        if isinstance(item, dict):
            packed = {key: visit(child) for key, child in item.items()}
        elif isinstance(item, (list, tuple)):
            packed = [visit(child) for child in item]
        else:
            return item
        key = json.dumps(packed, separators=(",", ":"), allow_nan=False)
        if len(key) < 20:
            return packed
        if key not in indexes:
            indexes[key] = len(pool)
            pool.append(packed)
        return {"$ref": indexes[key]}

    root = visit(value)
    return {"pool": pool, "root": root}


def unpack(value):
    def visit(item):
        if isinstance(item, dict):
            if set(item) == {"$ref"}:
                return visit(value["pool"][item["$ref"]])
            return {key: visit(child) for key, child in item.items()}
        if isinstance(item, list):
            return [visit(child) for child in item]
        return item

    return visit(value["root"])


def outcome(call):
    try:
        return {"value": scrub(call())}
    except (ValueError, OSError) as error:
        return {"error": type(error).__name__, "message": str(error)}


def synthetics(folder):
    rng = random.Random(SEED)
    result = []
    for kind in ("gradient", "noise", "split", "near-gray", "mono", "saturated", "alpha", "tiny"):
        for index in range(5):
            size = ((1, 1), (2, 2), (1, 2), (2, 1), (3, 3))[index] if kind == "tiny" else (173, 109)
            first, second = [tuple(rng.randrange(256) for _ in range(3)) for _ in range(2)]
            pixels = []
            for y in range(size[1]):
                for x in range(size[0]):
                    alpha = 255
                    if kind == "gradient":
                        t = x / max(1, size[0] - 1)
                        rgb = tuple(round(a + (b - a) * t) for a, b in zip(first, second, strict=True))
                    elif kind == "noise":
                        rgb = tuple(rng.randrange(256) for _ in range(3))
                    elif kind == "split":
                        rgb = first if x < size[0] * (index + 1) / 6 else second
                    elif kind == "near-gray":
                        level = rng.randrange(30, 221)
                        rgb = tuple(max(0, min(255, level + rng.randrange(-index - 1, index + 2))) for _ in range(3))
                    elif kind == "mono":
                        rgb = ((0, 0, 0), (255, 255, 255), (128, 128, 128), first, (1, 2, 3))[index]
                    elif kind == "saturated":
                        rgb = ((255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 0, 255), (0, 255, 255))[index]
                    elif kind == "alpha":
                        rgb = first if x < size[0] // 2 else second
                        alpha = (0, 1, 64, 128, 255)[(x + y + index) % 5] if index < 4 else 0
                    else:
                        rgb = first if (x + y) % 2 == 0 else second
                    pixels.append((*rgb, alpha))
            image = Image.new("RGBA", size)
            image.putdata(pixels)
            path = folder / f"{kind}-{index}.png"
            image.save(path)
            result.append(path)
    return result


def options():
    # All modes/personalities, with orthogonal nudges instead of a cartesian product.
    result = [{"mode": mode, "personality": p} for mode in ("dark", "light") for p in palette.PERSONALITIES]
    result += [
        {"accent": "ff3344"},
        {"overrides": {"primary": "1122cc", "secondary": "cc9922", "tertiary": "22aa66"}},
        {"background_from_wallpaper": True},
        {"background_from_wallpaper": False},
        {"brightness": -0.12},
        {"brightness": 0.12},
        {"saturation": 0.5},
        {"saturation": 1.5},
        *[{"time_tint": True, "hour": hour} for hour in (0, 7, 18, 23)],
        {"smart": True},
        {"harmony": True},
    ]
    return result


def expression_cases():
    # Capture the actual calls in the existing template tests, including failures.
    spec = importlib.util.spec_from_file_location("reference_templates", ROOT / "tests/test_matugen_templates.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    cases = []
    original = template_engine.render_values

    def capture(*args, **kwargs):
        call = {"args": deepcopy(args), "kwargs": deepcopy(kwargs)}
        if len(json.dumps(call)) < 20_000:
            call["expected"] = outcome(lambda: original(*args, **kwargs))
            if call not in cases:
                cases.append(call)
        return original(*args, **kwargs)

    names = [
        name
        for name in unittest.defaultTestLoader.getTestCaseNames(module.MatugenTemplateTests)
        if name not in ("test_loop_companion_query_and_failed_export_are_read_only",)
        and "cli" not in name
        and "source" not in name
    ]
    with patch.object(module, "render_values", capture), patch.object(export, "render_values", capture):
        suite = unittest.TestSuite(module.MatugenTemplateTests(name) for name in names)
        result = unittest.TestResult()
        suite.run(result)
    if not result.wasSuccessful():
        raise RuntimeError("Reference template tests failed: " + str(result.errors + result.failures))
    return cases


def make(destination):
    if importlib.util.find_spec("orient._core") is not None:
        raise RuntimeError("Generate reference fixtures only with the original Python implementation")
    destination.mkdir(parents=True, exist_ok=True)
    cases = {"schema": 1, "seed": SEED, "reference": {"version": __version__, "engine": ENGINE_ID}, "images": []}
    matrix = options()
    with tempfile.TemporaryDirectory(prefix="orient-reference-") as temporary:
        folder = Path(temporary)
        paths = sorted((ROOT / "tests/orient-gallery/images").iterdir()) + synthetics(folder)
        for path in paths:
            name = path.stem
            decoded = extract.frames(path)
            frame_names = []
            for index, image in enumerate(decoded):
                filename = f"{name}-{index}.png"
                image.save(destination / filename)
                frame_names.append(filename)
            case = {
                "name": name,
                "frames": frame_names,
                "analysis": outcome(lambda path=path: extract.analyze(path)),
                "classic": outcome(lambda path=path: classic.analyze(path)),
                "samples": [
                    {"point": [x, y], "expected": outcome(lambda x=x, y=y, path=path: extract.sample_color(path, x, y))}
                    for x, y in ((0, 0), (0.5, 0.5), (1, 1), (0.125, 0.875))
                ],
                "palettes": [],
            }
            for settings in matrix:
                case["palettes"].append(
                    {
                        "options": settings,
                        "expected": outcome(
                            lambda path=path, settings=settings: engine.from_image(
                                path, cache_dir=folder / "cache", **settings
                            )
                        ),
                    }
                )
            cases["images"].append(case)
            print(f"Recorded {name}: {len(decoded)} frames, {len(matrix)} palettes", flush=True)
    cases["defaults"] = [
        {
            "options": {"mode": m, "variant": v, "flavour": f},
            "expected": outcome(lambda m=m, v=v, f=f: engine.default_palette(m, v, f)),
        }
        for m in ("dark", "light")
        for v in palette.VARIANTS
        for f in ("default", "hard")
    ]
    cases["generated"] = [
        {
            "args": [s, m],
            "kwargs": {"personality": p},
            "expected": outcome(lambda s=s, m=m, p=p: palette.generate(s, m, personality=p)),
        }
        for s in ("47a99a", "000000", "ffffff", "ff0080", "808182")
        for m in ("dark", "light")
        for p in ("natural", "pearl", "vivid")
    ]
    cases["templates"] = []
    for mode in ("dark", "light"):
        sample = engine.default_palette(mode)
        for path in sorted((ROOT / "src/orient/templates").rglob("*.in")):
            cases["templates"].append(
                {
                    "name": str(path.relative_to(ROOT / "src/orient/templates")),
                    "mode": mode,
                    "text": path.read_text(),
                    "expected": outcome(lambda path=path, sample=sample: export.render(path.read_text(), sample)),
                }
            )
    cases["expressions"] = expression_cases()
    data = json.dumps(pack(cases), separators=(",", ":"), allow_nan=False).encode()
    (destination / "outputs.json.gz").write_bytes(gzip.compress(data, mtime=0))
    manifest = {
        "schema": 1,
        "seed": SEED,
        "reference": cases["reference"],
        "images": len(cases["images"]),
        "palettes": sum(len(c["palettes"]) for c in cases["images"]),
        "defaults": len(cases["defaults"]),
        "generated": len(cases["generated"]),
        "templates": len(cases["templates"]),
        "expressions": len(cases["expressions"]),
        "ignored_fields": ["engine", "path"],
    }
    (destination / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    size = sum(p.stat().st_size for p in destination.iterdir())
    if size > 5_000_000:
        raise RuntimeError(f"Golden fixtures exceed budget: {size} bytes")
    print(f"Golden fixtures: {size:,} bytes; {len(data):,} uncompressed JSON bytes")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "tests/parity/golden")
    make(parser.parse_args().output)
