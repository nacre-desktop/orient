"""Role audits and approximate full-severity color-vision simulations.

Numeric transforms from Machado, Oliveira & Fernandes (2009), DOI
10.1109/TVCG.2009.113. This independently written evaluator is not a diagnostic
vision test. Display gamut clipping and individual vision can change results.
"""

from itertools import combinations
from .colour import contrast, encoded, lab_from_rgb, linear, rgb

MATRICES = {
    "protanopia": ((0.152286, 1.052583, -0.204868), (0.114503, 0.786281, 0.099216), (-0.003882, -0.048116, 1.051998)),
    "deuteranopia": ((0.367322, 0.860646, -0.227968), (0.280085, 0.672501, 0.047413), (-0.011820, 0.042940, 0.968881)),
    "tritanopia": ((1.255528, -0.076749, -0.178779), (-0.078411, 0.930809, 0.147602), (0.004733, 0.691367, 0.303900)),
}


def simulate(value, vision="normal"):
    if vision == "normal":
        return value
    if vision not in MATRICES:
        raise ValueError("Unknown vision simulation")
    channels = tuple(linear(v) for v in rgb(value))
    result = [max(0.0, min(1.0, sum(v * w for v, w in zip(channels, row, strict=True)))) for row in MATRICES[vision]]
    return "".join(f"{round(encoded(v) * 255):02x}" for v in result)


def difference(a, b, vision="normal"):
    x, y = (lab_from_rgb(rgb(simulate(v, vision))) for v in (a, b))
    return sum((u - v) ** 2 for u, v in zip(x, y, strict=True)) ** 0.5


def text_pairs(colors):
    backgrounds = (
        "frame",
        "surface",
        "surfaceDim",
        "surfaceBright",
        "surfaceContainerLowest",
        "surfaceContainerLow",
        "surfaceContainer",
        "surfaceContainerHigh",
        "surfaceContainerHighest",
    )
    pairs = [
        (fg, bg)
        for fg in (
            "onSurface",
            "onSurfaceVariant",
            "primary",
            "secondary",
            "tertiary",
            "error",
            "success",
            "primaryDim",
            "secondaryDim",
            "tertiaryDim",
            "errorDim",
            "successDim",
            "red",
            "green",
            "blue",
            "yellow",
            "teal",
            "mauve",
            "pink",
            "peach",
            "lavender",
            "sky",
            "sapphire",
            "flamingo",
            "maroon",
            "rosewater",
        )
        for bg in backgrounds
    ]
    for role in ("primary", "secondary", "tertiary", "error", "success"):
        pairs += [("on" + role.title(), role), ("on" + role.title() + "Container", role + "Container")]
    for role in ("primary", "secondary", "tertiary"):
        pairs += [
            (fg, bg)
            for fg in ("on" + role.title() + "Fixed", "on" + role.title() + "FixedVariant")
            for bg in (role + "Fixed", role + "FixedDim")
        ]
    pairs += [("inverseOnSurface", "inverseSurface"), ("inversePrimary", "inverseSurface")]
    return pairs


def audit(colors):
    pairs = [
        {
            "foreground": fg,
            "background": bg,
            "ratio": round(contrast(colors[fg], colors[bg]), 3),
            "passes": contrast(colors[fg], colors[bg]) >= 4.5,
        }
        for fg, bg in text_pairs(colors)
    ]
    accents = ("primary", "secondary", "tertiary")
    distinctions = []
    for vision in ("normal", *MATRICES):
        for a, b in combinations(accents, 2):
            d = difference(colors[a], colors[b], vision)
            distinctions.append({"vision": vision, "roles": [a, b], "distance": round(d, 4), "distinct": d >= 0.045})
    return {
        "text": pairs,
        "textPasses": all(p["passes"] for p in pairs),
        "minimumTextContrast": min(p["ratio"] for p in pairs),
        "distinctions": distinctions,
        "warnings": [
            f"{' / '.join(p['roles'])} are close under {p['vision']}; keep labels or shapes"
            for p in distinctions
            if not p["distinct"]
        ],
        "simulation": "Machado 2009, severity 1; approximate, not a guarantee",
        "simulated": {vision: {role: simulate(colors[role], vision) for role in colors} for vision in MATRICES},
    }
