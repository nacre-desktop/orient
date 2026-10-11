"""Export-only color namespaces; Orient shades, not Matugen's HCT generation."""

from .colour import lch, color


BASE16_ROLES = (
    "surface",
    "surfaceContainerLow",
    "surfaceContainerHighest",
    "onSurfaceVariant",
    "onSurfaceVariant",
    "onSurface",
    "onSurface",
    "onSurface",
    "red",
    "peach",
    "yellow",
    "green",
    "teal",
    "blue",
    "mauve",
    "maroon",
)


def base16_contexts(contexts, active_mode):
    result = {f"base{index:02x}": {} for index in range(16)}
    for mode, values in contexts.items():
        for index, role in enumerate(BASE16_ROLES):
            result[f"base{index:02x}"][mode] = values[role]
        # Distinct neutral foreground steps, with existing readable comments/text.
        dim, chroma, hue = lch(values["onSurfaceVariant"])
        foreground, _, _ = lch(values["onSurface"])
        result["base04"][mode] = color((dim + foreground) / 2, chroma, hue)
        edge = 0.0 if (active_mode if mode == "default" else mode) == "light" else 1.0
        result["base06"][mode] = color(foreground + (edge - foreground) / 3, chroma, hue)
        result["base07"][mode] = color(foreground + (edge - foreground) * 2 / 3, chroma, hue)
    return result


def tonal_contexts(values):
    """Sample fixed OKLCH lightness while retaining each Orient seed's hue/chroma."""
    result = {}
    for family in ("primary", "secondary", "tertiary", "neutral", "neutral_variant", "error"):
        role = "neutralVariant" if family == "neutral_variant" else family
        _, chroma, hue = lch(values[role + "PaletteKeyColor"])
        result[family] = {f"_{tone}": color(tone / 100, chroma, hue) for tone in range(101)}
    return result
