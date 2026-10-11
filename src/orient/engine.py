"""Portable palette queries: deterministic input, private cache, no publisher."""

import hashlib
from pathlib import Path
from . import ENGINE_ID, FORMAT_VERSION
from .colour import clean, lch, hue_distance
from .extract import analyze
from .classic import analyze as classic_analyze
from .palette import DEFAULT_SEED, SEMANTIC_SEEDS, background_policy, generate, validate
from .accessibility import audit
from .storage import cache_key, read, roots, write_json


def _role_metadata(analysis, colors, seed, mode, personality, overrides, accent, background_from_wallpaper):
    sources = {}
    for role in ("primary", "secondary", "tertiary"):
        actual = overrides.get(role) or (
            seed if role == "primary" else colors["orient2" if role == "secondary" else "orient3"]
        )
        if personality == "pearl" and role == "primary":
            actual = colors["overtone"]
        match = next(
            (c for c in [*analysis["candidates"], *analysis["families"], *analysis["clusters"]] if c["hex"] == actual),
            None,
        )
        sources[role] = {
            "sourceColor": actual,
            "type": "signature"
            if personality == "pearl"
            else "picked"
            if role in overrides or role == "primary" and accent
            else "observed"
            if match
            else "derived",
            "coverage": match["coverage"] if match else 0.0,
            "location": match.get("location") if match else None,
            "regions": match.get("regions") if match else None,
        }
    provenance = {}
    body_source = analysis["body"]["dark" if mode == "dark" else "light"]["hex"] if background_from_wallpaper else seed
    semantic_sources = dict(SEMANTIC_SEEDS, error=SEMANTIC_SEEDS["red"], success=SEMANTIC_SEEDS["green"])
    ansi_sources = ("surface", "red", "green", "yellow", "blue", "mauve", "teal", "onSurface")
    for role, value in colors.items():
        parent = next((name for name in ("primary", "secondary", "tertiary") if name.lower() in role.lower()), "body")
        original = sources[parent]["sourceColor"] if parent in sources else body_source
        kind = "derived"
        if role in semantic_sources:
            original, kind = semantic_sources[role], "semantic"
        elif role.startswith("term"):
            index = int(role[4:]) % 8
            parent = ansi_sources[index]
            original, kind = semantic_sources.get(parent), "semantic" if 1 <= index <= 6 else "derived"
        elif role.startswith("on") or role in (
            "text",
            "subtext0",
            "subtext1",
            "outline",
            "klinkSelection",
            "kvisitedSelection",
            "knegativeSelection",
            "kneutralSelection",
            "kpositiveSelection",
        ):
            original, kind = None, "contrast"
        provenance[role] = {
            "parent": parent,
            "type": kind,
            "original": original,
            "adjusted": original is None or value != original,
        }
    for role, source_record in sources.items():
        provenance[role] = {
            "parent": role,
            "type": source_record["type"],
            "original": source_record["sourceColor"],
            "adjusted": colors[role] != source_record["sourceColor"],
        }
    return sources, provenance


def _pop_seed(analysis, seed, accent, overrides, personality):
    if personality == "pop" and not accent and "primary" not in overrides:
        # Choose a contrasting minority pigment, never a brighter shade of main.
        main_hue = lch(analysis["seed"])[2]
        dominant = next((c for c in analysis["families"] if c["hex"] == analysis["seed"]), None)
        main_coverage = dominant["coverage"] if dominant else 1.0
        minorities = [
            c
            for c in analysis["families"]
            if 0.004 <= c["coverage"] <= min(0.15, main_coverage * 0.65)
            and c["chroma"] >= 0.025
            and 0.2 <= c["lightness"] <= 0.9
            and hue_distance(main_hue, lch(c["hex"])[2]) >= 48
        ]
        if minorities:
            seed = max(
                minorities,
                key=lambda c: (
                    c["salience"]
                    * min(c["chroma"] / 0.12, 1.5) ** 0.8
                    * c["coverage"] ** 0.25
                    * (0.5 + 0.5 * min(hue_distance(main_hue, lch(c["hex"])[2]) / 120, 1)),
                    c["hex"],
                ),
            )["hex"]
    return seed


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


def palette_options(analysis, mode, variant, flavour, harmony=False, personality=None, background_from_wallpaper=None):
    result = []
    for index, candidate in enumerate(analysis["candidates"]):
        seed = candidate["hex"]
        colors = generate(
            seed,
            mode,
            variant,
            flavour,
            analysis["candidates"],
            harmony=harmony,
            body=analysis.get("body"),
            personality=personality,
            background_from_wallpaper=background_from_wallpaper,
        )
        _, chroma, hue = lch(seed)
        names = (
            (25, "Rose"),
            (55, "Copper"),
            (95, "Gold"),
            (130, "Olive"),
            (170, "Green"),
            (205, "Teal"),
            (245, "Blue"),
            (285, "Indigo"),
            (325, "Lilac"),
            (360, "Rose"),
        )
        name = "Neutral" if chroma < 0.012 else next(label for end, label in names if hue < end)
        if any(item["name"] == name for item in result):
            name += " tone"
        result.append(
            {
                "accent": seed,
                "name": name,
                "swatches": [colors[k] for k in ("primary", "secondary", "tertiary")],
                "surface": colors["frame"],
                "recommended": index == 0,
                "coverage": candidate["coverage"],
                "location": candidate.get("location"),
                "regions": candidate.get("regions"),
            }
        )
    return result


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
    personality = personality or ("harmony" if harmony else "natural")
    personality, background_from_wallpaper = background_policy(personality, background_from_wallpaper)
    overrides = {role: clean(value) for role, value in (overrides or {}).items()}
    settings = {
        "mode": mode,
        "variant": variant,
        "flavour": flavour,
        "accent": clean(accent) if accent else None,
        "smart": bool(smart),
        "harmony": personality == "harmony",
        "personality": personality,
        "background_from_wallpaper": background_from_wallpaper,
        "overrides": overrides,
        "brightness": brightness,
        "saturation": saturation,
        "hour": hour if personality == "tide" or time_tint else 12,
        "time_tint": bool(time_tint),
    }
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
    if personality == "natural":
        classic_key = cache_key(path, {"classic": 1})
        classic_path = folder / ("classic-" + classic_key + ".json")
        classic = read(classic_path, {})
        if not classic.get("candidates") or classic.get("engine") != ENGINE_ID:
            classic = dict(classic_analyze(path), engine=ENGINE_ID)
            write_json(classic_path, classic)
        analysis = dict(
            analysis,
            seed=classic["seed"],
            candidates=classic["candidates"],
            neutral=classic["neutral"],
            reason=classic["reason"],
            selectionPolicy="classic-natural",
        )
    if smart:
        mode = "light" if analysis["meanLightness"] >= 0.68 else "dark"
    seed = overrides.get("primary") or settings["accent"] or analysis["seed"]
    seed = _pop_seed(analysis, seed, accent, overrides, personality)
    effective_variant = (
        "neutral"
        if analysis["neutral"]
        and not accent
        and personality not in ("pearl", "vivid")
        and variant not in ("neutral", "monochrome")
        else variant
    )
    colors = generate(
        seed,
        mode,
        effective_variant,
        flavour,
        analysis["candidates"],
        harmony=personality == "harmony",
        body=analysis["body"],
        personality=personality,
        overrides=overrides,
        brightness=brightness,
        saturation=saturation,
        hour=hour,
        background_from_wallpaper=background_from_wallpaper,
        time_tint=time_tint,
    )
    if cache_key(path, settings) != identity:
        raise ValueError("Wallpaper changed during extraction; select it again")
    sources, provenance = _role_metadata(
        analysis, colors, seed, mode, personality, overrides, accent, background_from_wallpaper
    )
    data = {
        "formatVersion": FORMAT_VERSION,
        "name": "dynamic",
        "mode": mode,
        "variant": variant,
        "flavour": flavour,
        "colours": colors,
        "engine": ENGINE_ID,
        "input": settings,
        "source": {
            **analysis,
            "selected": seed,
            "options": palette_options(
                analysis,
                mode,
                effective_variant,
                flavour,
                personality=personality,
                background_from_wallpaper=background_from_wallpaper,
            ),
        },
        "roleSources": sources,
        "provenance": provenance,
        "accessibility": audit(colors),
    }
    try:
        write_json(cache, data)
    except OSError:
        pass
    return data


def default_palette(mode="dark", variant="tonalspot", flavour="default"):
    colors = generate(DEFAULT_SEED, mode, variant, flavour)
    return {
        "formatVersion": FORMAT_VERSION,
        "name": "default",
        "mode": mode,
        "variant": variant,
        "flavour": flavour,
        "colours": colors,
        "engine": ENGINE_ID,
        "input": {"personality": "natural"},
        "accessibility": audit(colors),
    }
