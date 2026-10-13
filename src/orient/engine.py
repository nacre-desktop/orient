"""Portable palette queries: deterministic input, private cache, no publisher."""

import hashlib
from pathlib import Path
from . import ENGINE_ID, FORMAT_VERSION
from .colour import clean
from .extract import analyze
from .classic import analyze as classic_analyze
from .palette import validate
from ._bridge import call
from .storage import cache_key, read, roots, write_json


def _image_analysis(path, folder):
    analysis_key = cache_key(path, {"analysis": 3})
    analysis_path = folder / ("analysis-" + analysis_key + ".json")
    try:
        analysis = read(analysis_path, {})
        if (
            not isinstance(analysis, dict)
            or analysis.get("engine") != ENGINE_ID
            or not analysis.get("candidates")
            or not analysis.get("body")
        ):
            analysis = {}
    except (OSError, ValueError, TypeError):
        analysis = {}
    if not analysis:
        analysis = analyze(path)
        hasher = hashlib.sha256()
        with path.open("rb") as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b""):
                hasher.update(block)
        analysis = dict(analysis, engine=ENGINE_ID, digest=hasher.hexdigest(), path=str(path))
        if cache_key(path, {"analysis": 3}) != analysis_key:
            raise ValueError("Wallpaper changed during analysis")
        try:
            write_json(analysis_path, analysis)
        except OSError:
            pass
    return analysis


def from_image(
    path,
    mode="dark",
    variant="tonalspot",
    flavour="default",
    accent=None,
    smart=False,
    harmony=False,
    personality=None,
    overrides=None,
    brightness=0.0,
    saturation=1.0,
    hour=12,
    cache_dir=None,
    background_from_wallpaper=None,
    time_tint=False,
):
    path = Path(path).expanduser().resolve(strict=True)
    options = dict(
        mode=mode,
        variant=variant,
        flavour=flavour,
        accent=accent,
        smart=bool(smart),
        harmony=bool(harmony),
        personality=personality,
        overrides=overrides,
        brightness=brightness,
        saturation=saturation,
        hour=hour,
        background_from_wallpaper=background_from_wallpaper,
        time_tint=bool(time_tint),
    )
    settings = call("settings", options=options)
    personality = settings["personality"]
    identity = cache_key(path, settings)
    folder = Path(cache_dir) if cache_dir is not None else roots()[2] / "palettes"
    cache = folder / (identity + ".json")
    try:
        cached = read(cache, {})
        if (
            isinstance(cached, dict)
            and cached.get("formatVersion") == FORMAT_VERSION
            and cached.get("engine") == ENGINE_ID
            and cached.get("input") == settings
            and cached.get("name") == "dynamic"
            and cached.get("variant") == variant
            and cached.get("flavour") == flavour
            and cached.get("mode") in ("light", "dark")
            and (smart or cached["mode"] == mode)
        ):
            validate(cached["colours"])
            source = cached["source"]
            if isinstance(source, dict) and isinstance(source.get("digest"), str) and len(source["digest"]) == 64:
                clean(source["selected"])
                return cached
    except (OSError, ValueError, KeyError, TypeError):
        pass
    analysis = _image_analysis(path, folder)
    classic = None
    if personality == "natural":
        classic_key = cache_key(path, {"classic": 1})
        classic_path = folder / ("classic-" + classic_key + ".json")
        classic = read(classic_path, {})
        if not classic.get("candidates") or classic.get("engine") != ENGINE_ID:
            classic = dict(classic_analyze(path), engine=ENGINE_ID)
            write_json(classic_path, classic)
    data = call("from_analysis", analysis=analysis, classic=classic, settings=settings, hour=hour)
    if cache_key(path, settings) != identity:
        raise ValueError("Wallpaper changed during extraction; select it again")
    try:
        write_json(cache, data)
    except OSError:
        pass
    return data


def palette_options(analysis, mode, variant, flavour, harmony=False, personality=None, background_from_wallpaper=None):
    return call(
        "palette_options",
        analysis=analysis,
        options=dict(
            mode=mode,
            variant=variant,
            flavour=flavour,
            harmony=bool(harmony),
            personality=personality,
            background_from_wallpaper=background_from_wallpaper,
        ),
    )


def default_palette(mode="dark", variant="tonalspot", flavour="default"):
    return call("default_palette", options=dict(mode=mode, variant=variant, flavour=flavour))
