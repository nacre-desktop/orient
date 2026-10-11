"""Portable role policies for faithful, readable image-derived themes."""

import math

from .colour import clean, color, contrast, foreground, hue_distance, lch, readable
from .accessibility import difference, text_pairs

DEFAULT_SEED = "47a99a"
VARIANTS = (
    "tonalspot",
    "vibrant",
    "expressive",
    "fidelity",
    "fruitsalad",
    "monochrome",
    "neutral",
    "rainbow",
    "content",
)


SEMANTIC_SEEDS = {
    "rosewater": "d8ada7",
    "flamingo": "dd8d81",
    "pink": "d965aa",
    "mauve": "a86fd0",
    "red": "df303e",
    "maroon": "b64b60",
    "peach": "e68d50",
    "yellow": "d4b14b",
    "green": "289563",
    "teal": "309f95",
    "sky": "55accc",
    "sapphire": "398ab7",
    "blue": "477fe0",
    "lavender": "8f84d5",
}


def _readable_roles(colors, levels):
    # A complete opaque text-role matrix, including Dim/terminal consumers.
    all_backgrounds = [colors[k] for k in levels] + [colors["frame"]]
    for role in (
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
    ):
        colors[role] = readable(colors[role], all_backgrounds)
    for role in ("primary", "secondary", "tertiary", "error", "success"):
        colors["on" + role.title()] = foreground(colors[role])
    colors["onBackground"] = colors["text"] = colors["onSurface"]
    colors["subtext0"] = colors["subtext1"] = colors["onSurfaceVariant"]
    colors["surfaceTint"] = colors["primary"]
    validate(colors)
    return colors


def _semantic_roles(colors, hue, backgrounds):
    # Legacy adapters are role names only; named ANSI colors keep their meaning.
    semantic = SEMANTIC_SEEDS
    for role, value in semantic.items():
        light, c, h = lch(value)
        # At most four degrees: error remains red, success green, links blue.
        themed = color(light, c, h + 4 * math.sin(math.radians(hue - h)))
        colors[role] = readable(themed, backgrounds)
    for role, value in {
        "text": "onSurface",
        "subtext1": "onSurfaceVariant",
        "subtext0": "onSurfaceVariant",
        "overlay2": "outline",
        "overlay1": "outline",
        "overlay0": "outlineVariant",
        "surface2": "surfaceContainerHighest",
        "surface1": "surfaceContainerHigh",
        "surface0": "surfaceContainer",
        "base": "surface",
        "mantle": "surfaceContainerLowest",
        "crust": "surfaceContainerLowest",
        "klink": "blue",
        "kvisited": "mauve",
        "knegative": "error",
        "kneutral": "yellow",
        "kpositive": "success",
    }.items():
        colors[role] = colors[value]
    for role in ("klink", "kvisited", "knegative", "kneutral", "kpositive"):
        colors[role + "Selection"] = readable(colors[role], [colors["primary"]], 4.5)
    ansi = ("surface", "red", "green", "yellow", "blue", "mauve", "teal", "onSurface")
    for i, role in enumerate(ansi):
        colors[f"term{i}"] = colors[role]
        colors[f"term{i + 8}"] = colors["onSurfaceVariant"] if i == 0 else colors[role]


def supporting_sources(seed, candidates, harmony=False):
    """Select real pigment families; Harmony favors coverage and related hues."""
    records = [dict(c) if isinstance(c, dict) else {"hex": clean(c)} for c in candidates]
    records = [c for c in records if clean(c["hex"]) != clean(seed)]
    if not harmony:
        chosen = []
        for candidate in records:
            value = clean(candidate["hex"])
            if all(hue_distance(lch(value)[2], lch(other)[2]) >= 18 for other in [seed, *chosen]):
                chosen.append(value)
            if len(chosen) == 2:
                break
        return chosen
    main = lch(seed)
    dominant = max((c.get("coverage", 0) for c in candidates if isinstance(c, dict)), default=0)
    ranked = []
    for candidate in records:
        coverage = candidate.get("coverage")
        if coverage is not None and coverage < max(0.04, dominant * 0.05):
            continue
        value = clean(candidate["hex"])
        light, chroma, hue = lch(value)
        distance = hue_distance(main[2], hue)
        if coverage is not None and distance > 100 and coverage < 0.12:
            continue
        # Strong contrasts belong when they occupy a substantial part of the image.
        related = 1.0 if distance <= 80 else 0.55
        if coverage is not None and coverage >= 0.15:
            related = max(related, 0.90)
        olive = 85 <= hue <= 145 and light < 0.62 and 0.012 <= chroma < 0.12
        weight = candidate.get("score", coverage or 1) * related * (0.28 if olive else 1)
        ranked.append((weight, value))
    ranked.sort(key=lambda item: (-item[0], item[1]))
    chosen = []
    for _, value in ranked:
        if all(hue_distance(lch(value)[2], lch(other)[2]) >= 24 for other in chosen):
            chosen.append(value)
        if len(chosen) == 2:
            break
    return chosen


PERSONALITIES = ("natural", "pigment", "harmony", "pop", "mist", "vivid", "pearl", "tide")


def background_policy(personality, background_from_wallpaper=None):
    """Normalize retired Source choices without losing their background policy."""
    if personality == "source":
        personality = "natural"
        if background_from_wallpaper is None:
            background_from_wallpaper = True
    if personality not in PERSONALITIES:
        raise ValueError("Unknown palette personality")
    if background_from_wallpaper is None:
        background_from_wallpaper = personality not in ("natural", "pigment")
    if type(background_from_wallpaper) is not bool:
        raise ValueError("Background from wallpaper must be a boolean")
    return personality, background_from_wallpaper


def _accent_family(
    colors,
    backgrounds,
    role,
    source,
    strength,
    *,
    brightness,
    saturation,
    personality,
    variant,
    harmony,
    dark,
    soft,
    chroma,
):
    light, c, h = lch(source)
    light = max(0.0, min(1.0, light + brightness))
    c = c * strength * saturation * (1.35 if personality == "vivid" else 1.0)
    if variant == "monochrome" and role in ("primary", "secondary", "tertiary"):
        c = 0
    if (
        harmony
        and role in ("primary", "secondary", "tertiary")
        and 85 <= h <= 145
        and light < 0.62
        and 0.012 <= c < 0.12
    ):
        light = 0.70
    if role == "primary":
        c = chroma
    # Lift dark pigments without turning the same saturation into a gray tint.
    # Relative OKLCH strength grows with lightness, bounded by sRGB gamut.
    lifted = max(light, 0.60) if dark and not soft and c >= 0.012 else light
    lifted_chroma = c * min(1.8, lifted / max(light, 0.12))
    if dark and not soft and variant != "monochrome" and c >= 0.012:
        # Subtle real hues need enough strength to remain distinct when lifted.
        lifted_chroma = max(lifted_chroma, min(0.065, c * 3.0))
    if not soft and variant != "vibrant":
        lifted_chroma = min(lifted_chroma, 0.19 if role == "primary" else 0.16)
    if not dark and role in ("primary", "secondary", "tertiary"):
        # Shadow pigments need a usable middle tone in light mode too.
        # Keep source hue/chroma; contrast may make a small final adjustment.
        lifted = max(0.52, min(0.58, light))
        lifted_chroma = c
        # Pale warm pigments lose their warmth at text-safe middle tones.
        # Give just their UI accents extra chroma; source hue stays intact.
        if light >= 0.65 and 45 <= h <= 110 and 0.012 <= c <= 0.12:
            lifted = 0.62
            lifted_chroma = min(0.14, c * 1.28 + 0.01)
        if c < 0.012 or variant == "monochrome":
            lifted = {"primary": 0.50, "secondary": 0.41, "tertiary": 0.35}[role]
    accent = readable(color(lifted, lifted_chroma, h), backgrounds)
    container_chroma = min(max(c * 0.65, min(0.045, c * 1.7)), 0.095) if dark and not soft else min(c * 0.42, 0.085)
    container = color(0.35 if dark else 0.88, container_chroma, h)
    colors[role] = accent
    colors["on" + role.title()] = foreground(accent)
    colors[role + "Dim"] = readable(color(lch(accent)[0] + (-0.025 if dark else 0.025), c, h), backgrounds)
    colors[role + "Container"] = container
    colors["on" + role.title() + "Container"] = readable(
        color(0.93 if dark else 0.22, min(c * 0.3, 0.05), h), [container]
    )
    if role in ("primary", "secondary", "tertiary"):
        fixed = color(0.87, min(c, 0.14), h)
        fixed_dim = color(0.78, min(c, 0.14), h)
        colors[role + "Fixed"] = fixed
        colors[role + "FixedDim"] = fixed_dim
        colors["on" + role.title() + "Fixed"] = readable(color(0.20, min(c, 0.07), h), [fixed, fixed_dim])
        colors["on" + role.title() + "FixedVariant"] = readable(color(0.35, min(c, 0.07), h), [fixed, fixed_dim])
        colors[role + "PaletteKeyColor"] = color(light, c, h)
        colors[role + "_paletteKeyColor"] = colors[role + "PaletteKeyColor"]


def generate(
    seed,
    mode="dark",
    variant="tonalspot",
    flavour="default",
    companions=(),
    harmony=False,
    body=None,
    personality=None,
    overrides=None,
    brightness=0.0,
    saturation=1.0,
    hour=12,
    background_from_wallpaper=None,
    time_tint=False,
):
    personality = personality or ("harmony" if harmony else "natural")
    personality, background_from_wallpaper = background_policy(personality, background_from_wallpaper)
    if not -0.15 <= brightness <= 0.15 or not 0 <= saturation <= 1.6 or not 0 <= hour <= 23:
        raise ValueError("Color adjustment is outside its safe bounds")
    overrides = overrides or {}
    if set(overrides) - {"primary", "secondary", "tertiary"}:
        raise ValueError("Unknown accent role")
    seed = clean(overrides.get("primary", seed))
    if personality == "pearl":
        seed = clean(overrides.get("primary", "d99fb7"))
    if personality == "mist":
        variant = "neutral"
    elif personality == "vivid":
        variant = "vibrant"
    if mode not in ("light", "dark") or variant not in VARIANTS or flavour not in ("default", "hard"):
        raise ValueError("Unsupported palette mode, variant or flavour")
    # Background policy is independent of the accent personality.
    body = body if background_from_wallpaper else None
    dark = mode == "dark"
    _, chroma, hue = lch(seed)
    chroma *= saturation * (1.35 if personality == "vivid" else 1.0)
    if personality == "mist":
        chroma = min(chroma, 0.035)
    soft = variant in ("neutral", "content")
    if variant == "monochrome":
        chroma = 0.0
    elif soft:
        chroma = min(chroma * 0.55, 0.10)
    elif variant != "vibrant":
        # Wallpaper accents have a comfortable color-strength ceiling. Explicit
        # Vivid presets remain vivid; the decorative source color is kept below.
        chroma = min(chroma, 0.19)
    # Body tint stays modest: color belongs in accents, not a global cast.
    tint = min(chroma * 0.18, 0.028)
    if flavour == "hard":
        tint *= 0.5
    levels = {
        "surface": 0.18 if dark else 0.975,
        "surfaceDim": 0.18 if dark else 0.87,
        "surfaceBright": 0.27 if dark else 0.985,
        "surfaceContainerLowest": 0.15 if dark else 0.995,
        "surfaceContainerLow": 0.19 if dark else 0.955,
        "surfaceContainer": 0.215 if dark else 0.935,
        "surfaceContainerHigh": 0.24 if dark else 0.91,
        "surfaceContainerHighest": 0.255 if dark else 0.885,
    }
    body_hue, body_chroma = hue, tint
    if body and personality != "pearl":
        _, observed_chroma, body_hue = lch(body["dark" if dark else "light"]["hex"])
        body_chroma = min(observed_chroma, 0.045 if dark else 0.035)
    if personality in ("mist", "pearl"):
        body_chroma *= 0.18 if personality == "mist" else 0
    if personality == "tide" or (time_tint and personality != "pearl"):
        body_hue = 55 if hour >= 17 or hour < 5 else 250
        body_chroma = min(0.022, max(0.012, body_chroma))
    colors = {role: color(light, body_chroma, body_hue) for role, light in levels.items()}
    frame_chroma = min(0.055, max(chroma * 0.5, min(0.028, chroma * 1.4)))
    colors["frame"] = color(0.225 if dark else 0.97, body_chroma if body else frame_chroma, body_hue)
    backgrounds = [
        colors["frame"],
        colors["surface"],
        colors["surfaceContainerHighest"],
        colors["surfaceBright" if dark else "surfaceDim"],
    ]
    colors.update(
        background=colors["surface"],
        surfaceVariant=colors["surfaceContainerHighest"],
        onSurface=readable(color(0.94 if dark else 0.20, tint, hue), backgrounds),
        onSurfaceVariant=readable(color(0.76 if dark else 0.40, tint, hue), backgrounds),
        shadow="000000",
        scrim="000000",
    )
    colors["onBackground"] = colors["onSurface"]
    colors["outline"] = readable(color(0.58 if dark else 0.54, tint, hue), backgrounds, 3.0)
    colors["outlineVariant"] = color(0.37 if dark else 0.79, tint, hue)
    inverse_chroma, inverse_hue = tint, hue
    if body and personality != "pearl":
        _, inverse_chroma, inverse_hue = lch(body["light" if dark else "dark"]["hex"])
        inverse_chroma = min(inverse_chroma, 0.035)
    colors["inverseSurface"] = color(0.96 if dark else 0.15, inverse_chroma, inverse_hue)
    colors["inverseOnSurface"] = readable(color(0.25 if dark else 0.92, tint, hue), [colors["inverseSurface"]])

    def family(role, source, strength=1.0):
        _accent_family(
            colors,
            backgrounds,
            role,
            source,
            strength,
            brightness=brightness,
            saturation=saturation,
            personality=personality,
            variant=variant,
            harmony=harmony,
            dark=dark,
            soft=soft,
            chroma=chroma,
        )

    sources = supporting_sources(seed, companions, harmony)
    if personality == "harmony":
        source_l, source_c, source_h = lch(seed)
        sources = [
            color(source_l + 0.04, source_c * 0.8, source_h - 20),
            color(source_l + 0.08, source_c * 0.7, source_h + 24),
        ]
    elif personality == "pearl":
        sources = ["aaa6d5", "a5c5b4"]
    secondary = sources[0] if sources else color(lch(seed)[0], chroma * 0.70, hue)
    tertiary = sources[1] if len(sources) > 1 else color(lch(seed)[0], chroma * 0.45, hue)
    family("primary", seed)
    secondary = clean(overrides.get("secondary", secondary))
    tertiary = clean(overrides.get("tertiary", tertiary))
    family("secondary", secondary, 0.85 if not soft else 0.5)
    family("tertiary", tertiary, 0.85 if not soft else 0.5)
    family("error", "df303e")
    family("success", "289563")
    # Dark-mode accents have headroom to separate in lightness. Light mode keeps
    # its middle-tone policy; diagnostics report chromatic simulation limits.
    for role, offset in (("secondary", 0.09), ("tertiary", 0.16)):
        previous = [colors["primary"]] + ([colors["secondary"]] if role == "tertiary" else [])
        if (
            dark
            and min(
                difference(colors[role], p, v)
                for p in previous
                for v in ("normal", "protanopia", "deuteranopia", "tritanopia")
            )
            < 0.045
            and role not in overrides
        ):
            light, c, h = lch(colors[role])
            target = (
                min(0.94, lch(colors["primary"])[0] + offset)
                if dark
                else max(0.38, lch(colors["primary"])[0] - offset * 0.5)
            )
            colors[role] = readable(color(target, c, h), backgrounds)
            colors["on" + role.title()] = foreground(colors[role])
    colors["surfaceTint"] = colors["primary"]
    # Rich accents for decoration can retain the source even when text must brighten.
    colors["overtone"] = seed if variant != "monochrome" else color(lch(seed)[0], 0, hue)
    colors["orient1"] = colors["overtone"]
    colors["orient2"] = color(lch(secondary)[0], 0, hue) if variant == "monochrome" else clean(secondary)
    colors["orient3"] = color(lch(tertiary)[0], 0, hue) if variant == "monochrome" else clean(tertiary)
    colors["inversePrimary"] = readable(colors["overtone"], [colors["inverseSurface"]])
    colors["neutralPaletteKeyColor"] = color(0.55, body_chroma, body_hue)
    colors["neutralVariantPaletteKeyColor"] = color(0.55, min(body_chroma * 2, 0.03), body_hue)
    colors["neutral_paletteKeyColor"] = colors["neutralPaletteKeyColor"]
    colors["neutral_variant_paletteKeyColor"] = colors["neutralVariantPaletteKeyColor"]
    colors["errorPaletteKeyColor"] = "df303e"

    _semantic_roles(colors, hue, backgrounds)
    return _readable_roles(colors, levels)


def validate(colors):
    pairs = [
        ("onSurface", "surface"),
        ("onSurfaceVariant", "surfaceContainerHighest"),
        ("inverseOnSurface", "inverseSurface"),
    ]
    for role in ("primary", "secondary", "tertiary", "error", "success"):
        pairs.extend(
            (
                ("on" + role.title(), role),
                ("on" + role.title() + "Container", role + "Container"),
                (role, "surfaceContainerHighest"),
            )
        )
    pairs = text_pairs(colors)
    for fg, bg in pairs:
        if contrast(colors[fg], colors[bg]) < 4.5:
            raise ValueError(f"Unreadable generated roles: {fg}/{bg}")
    if not REQUIRED_ROLES <= colors.keys():
        raise ValueError("Incomplete palette roles")
    for value in colors.values():
        clean(value)


# Compatibility vocabulary captured from the installed CLI output.
REQUIRED_ROLES = frozenset(
    (
        "background",
        "base",
        "blue",
        "crust",
        "error",
        "errorContainer",
        "errorDim",
        "errorPaletteKeyColor",
        "flamingo",
        "green",
        "inverseOnSurface",
        "inversePrimary",
        "inverseSurface",
        "klink",
        "klinkSelection",
        "knegative",
        "knegativeSelection",
        "kneutral",
        "kneutralSelection",
        "kpositive",
        "kpositiveSelection",
        "kvisited",
        "kvisitedSelection",
        "lavender",
        "mantle",
        "maroon",
        "mauve",
        "neutralPaletteKeyColor",
        "neutralVariantPaletteKeyColor",
        "neutral_paletteKeyColor",
        "neutral_variant_paletteKeyColor",
        "onBackground",
        "onError",
        "onErrorContainer",
        "onPrimary",
        "onPrimaryContainer",
        "onPrimaryFixed",
        "onPrimaryFixedVariant",
        "onSecondary",
        "onSecondaryContainer",
        "onSecondaryFixed",
        "onSecondaryFixedVariant",
        "onSuccess",
        "onSuccessContainer",
        "onSurface",
        "onSurfaceVariant",
        "onTertiary",
        "onTertiaryContainer",
        "onTertiaryFixed",
        "onTertiaryFixedVariant",
        "outline",
        "outlineVariant",
        "overlay0",
        "overlay1",
        "overlay2",
        "peach",
        "pink",
        "primary",
        "primaryContainer",
        "primaryDim",
        "primaryFixed",
        "primaryFixedDim",
        "primaryPaletteKeyColor",
        "primary_paletteKeyColor",
        "red",
        "rosewater",
        "sapphire",
        "scrim",
        "secondary",
        "secondaryContainer",
        "secondaryDim",
        "secondaryFixed",
        "secondaryFixedDim",
        "secondaryPaletteKeyColor",
        "secondary_paletteKeyColor",
        "shadow",
        "sky",
        "subtext0",
        "subtext1",
        "success",
        "successContainer",
        "surface",
        "surface0",
        "surface1",
        "surface2",
        "surfaceBright",
        "surfaceContainer",
        "surfaceContainerHigh",
        "surfaceContainerHighest",
        "surfaceContainerLow",
        "surfaceContainerLowest",
        "surfaceDim",
        "surfaceTint",
        "surfaceVariant",
        "teal",
        "term0",
        "term1",
        "term10",
        "term11",
        "term12",
        "term13",
        "term14",
        "term15",
        "term2",
        "term3",
        "term4",
        "term5",
        "term6",
        "term7",
        "term8",
        "term9",
        "tertiary",
        "tertiaryContainer",
        "tertiaryDim",
        "tertiaryFixed",
        "tertiaryFixedDim",
        "tertiaryPaletteKeyColor",
        "tertiary_paletteKeyColor",
        "text",
        "yellow",
    )
)
