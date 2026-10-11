"""Nacre's original Natural hue-family selection, retained as a named contract.

Restored from our own Orient implementation at 9c9089f. This policy preserves
minority pigments and the former coverage/spread balance; the perceptual subject-
weighted policy remains separately available as Pigment. Source census and
readability still use current Orient diagnostics/role generation.
"""

from collections import Counter
import math
from pathlib import Path
from .extract import frames
from .colour import color, hue_distance, lab_from_rgb, lch


def analyze(path):
    path = Path(path).expanduser().resolve(strict=True)
    images = frames(path)
    image = images[0]
    bins = {}
    total = 0.0
    for i, (r, g, b, alpha) in enumerate(image.get_flattened_data()):
        if alpha == 0:
            continue
        weight = alpha / 255
        key = (r >> 3, g >> 3, b >> 3)
        entry = bins.setdefault(key, [0.0, 0.0, 0.0, 0.0, Counter()])
        entry[0] += weight
        for channel, value in enumerate((r, g, b), start=1):
            entry[channel] += value * weight
        x, y = i % image.width, i // image.width
        tile = (min(3, x * 4 // image.width), min(3, y * 4 // image.height))
        entry[4][tile] += weight
        total += weight
    if total == 0:
        raise ValueError("Wallpaper has no visible pixels")

    # A bounded thumbnail limits all bins. Keep minority hues rather than
    # dropping blue hair / lantern colors below a global popularity cutoff.
    ordered = sorted(bins.values(), key=lambda e: (-e[0], e[1:4]))
    groups = []
    mean_light = 0.0
    observed = 0.0
    mean_a = 0.0
    mean_b = 0.0
    for weight, r, g, b, tiles in ordered:
        rgb = (r / weight / 255, g / weight / 255, b / weight / 255)
        light, a, bb = lab_from_rgb(rgb)
        chroma = math.hypot(a, bb)
        hue = math.degrees(math.atan2(bb, a)) % 360
        mean_light += light * weight
        observed += weight
        mean_a += a * weight
        mean_b += bb * weight
        match = None
        for group in groups:
            gl, gc, gh = group["lch"]
            neutral = chroma < 0.012 and gc < 0.012
            related = chroma >= 0.012 and gc >= 0.012 and hue_distance(hue, gh) < 16 and abs(chroma - gc) < 0.12
            if (neutral or related) and abs(light - gl) < 0.36:
                match = group
                break
        if match is None:
            match = {"weight": 0.0, "sum": [0.0, 0.0, 0.0], "tiles": Counter(), "lch": (light, chroma, hue)}
            groups.append(match)
        match["weight"] += weight
        # Representative color favors lit pigment over near-black shades; coverage
        # and scoring still use true pixel population, so highlights cannot fake area.
        pigment_weight = weight * (0.25 + 0.75 * min(light / 0.65, 1))
        match["pigment"] = match.get("pigment", 0.0) + pigment_weight
        for i, component in enumerate((light, a, bb)):
            match["sum"][i] += component * pigment_weight
        match["tiles"].update(tiles)
        gl, ga, gb = (v / match["pigment"] for v in match["sum"])
        match["lch"] = (gl, math.hypot(ga, gb), math.degrees(math.atan2(gb, ga)) % 360)

    candidates = []
    for group in groups:
        light, chroma, hue = group["lch"]
        coverage = group["weight"] / total
        if coverage < 0.004 or chroma < 0.012 or light < 0.12 or light > 0.96:
            continue
        spread = sum(w >= total * 0.001 for w in group["tiles"].values()) / 16
        score = coverage**0.60 * (min(chroma / 0.12, 1.5) ** 0.8)
        score *= (0.50 + 0.50 * min(light / 0.55, 1)) * (0.80 + 0.20 * spread)
        candidates.append(
            {
                "hex": color(light, chroma, hue),
                "coverage": round(coverage, 5),
                "chroma": round(chroma, 5),
                "score": round(score, 6),
                "spread": round(spread, 4),
                "salience": round(0.80 + 0.20 * spread, 4),
                "lightness": round(light, 5),
                "regions": {
                    "columns": 4,
                    "rows": 4,
                    "cells": [
                        {"index": y * 4 + x, "coverage": round(w / total, 6)} for (x, y), w in group["tiles"].items()
                    ],
                },
            }
        )
    candidates.sort(key=lambda c: (-c["score"], c["hex"]))
    distinct = []
    # Reserve room for genuinely different hue families before adding close tones.
    for separation in (48, 24):
        for candidate in candidates:
            hue = lch(candidate["hex"])[2]
            if all(hue_distance(hue, lch(c["hex"])[2]) >= separation for c in distinct):
                distinct.append(candidate)
            if len(distinct) == 5:
                break
        if len(distinct) == 5:
            break
    if not distinct:
        # A neutral photograph can still be warm/cool; do not invent a blue accent.
        neutral_a, neutral_b = mean_a / observed, mean_b / observed
        neutral_seed = color(
            0.55, min(math.hypot(neutral_a, neutral_b), 0.025), math.degrees(math.atan2(neutral_b, neutral_a))
        )
        distinct = [{"hex": neutral_seed, "coverage": 1.0, "chroma": 0.0, "score": 0.0, "spread": 1.0}]
    return {
        "seed": distinct[0]["hex"],
        "candidates": distinct,
        "meanLightness": round(mean_light / observed, 5),
        "neutral": distinct[0]["score"] == 0,
        "reason": "coverage, chroma, usable lightness and spatial spread"
        if distinct[0]["score"]
        else "neutral image fallback",
    }
