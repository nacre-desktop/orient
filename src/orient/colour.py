"""sRGB/OKLab equations and hue-preserving gamut/contrast operations.

Matrices: Björn Ottosson's published OKLab definition (public-domain equations),
https://bottosson.github.io/posts/oklab/ . Nacre's gamut/contrast policies are local.
"""

import math
import re


def clean(value):
    if not isinstance(value, str):
        raise ValueError("Expected a six-digit RGB color")
    value = value.removeprefix("#")
    if not re.fullmatch("[0-9a-fA-F]{6}", value):
        raise ValueError("Expected a six-digit RGB color")
    return value.lower()


def rgb(value):
    value = clean(value)
    return tuple(int(value[i : i + 2], 16) / 255 for i in (0, 2, 4))


def linear(channel):
    return channel / 12.92 if channel <= 0.04045 else ((channel + 0.055) / 1.055) ** 2.4


def encoded(channel):
    return 12.92 * channel if channel <= 0.0031308 else 1.055 * channel ** (1 / 2.4) - 0.055


def luminance(value):
    return sum(linear(v) * w for v, w in zip(rgb(value), (0.2126, 0.7152, 0.0722), strict=True))


def contrast(a, b):
    low, high = sorted((luminance(a), luminance(b)))
    return (high + 0.05) / (low + 0.05)


def lab_from_rgb(channels):
    r, g, b = (linear(v) for v in channels)
    long_wave = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b) ** (1 / 3)
    m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b) ** (1 / 3)
    s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b) ** (1 / 3)
    return (
        0.2104542553 * long_wave + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * long_wave - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * long_wave + 0.7827717662 * m - 0.8086757660 * s,
    )


def lch(value):
    light, a, b = lab_from_rgb(rgb(value))
    return light, math.hypot(a, b), math.degrees(math.atan2(b, a)) % 360


def lab_to_linear(light, a, b):
    long_wave = (light + 0.3963377774 * a + 0.2158037573 * b) ** 3
    m = (light - 0.1055613458 * a - 0.0638541728 * b) ** 3
    s = (light - 0.0894841775 * a - 1.2914855480 * b) ** 3
    return (
        4.0767416621 * long_wave - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * long_wave + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * long_wave - 0.7034186147 * m + 1.7076147010 * s,
    )


def color(light, chroma, hue):
    """Find the strongest in-sRGB chroma at this fixed lightness and hue."""
    light = min(1.0, max(0.0, light))
    chroma = max(0.0, chroma)
    angle = math.radians(hue)

    def channels(c):
        return lab_to_linear(light, c * math.cos(angle), c * math.sin(angle))

    values = channels(chroma)
    if not all(-1e-7 <= v <= 1.0000001 for v in values):
        low, high = 0.0, chroma
        for _ in range(20):
            mid = (low + high) / 2
            if all(-1e-7 <= v <= 1.0000001 for v in channels(mid)):
                low = mid
            else:
                high = mid
        values = channels(low)
    return "".join(f"{round(encoded(min(1.0, max(0.0, v))) * 255):02x}" for v in values)


def foreground(background):
    return max(("101014", "ffffff"), key=lambda c: contrast(c, background))


def readable(value, backgrounds, minimum=4.5):
    """Smallest lightness change meeting every background's contrast requirement."""
    value = clean(value)
    if all(contrast(value, bg) >= minimum for bg in backgrounds):
        return value
    light, chroma, hue = lch(value)
    targets = sorted((0.0, 1.0), key=lambda t: abs(t - light))
    choices = []
    for target in targets:
        end = color(target, chroma, hue)
        if not all(contrast(end, bg) >= minimum for bg in backgrounds):
            continue
        low, high = 0.0, 1.0
        for _ in range(18):
            mid = (low + high) / 2
            sample = color(light + (target - light) * mid, chroma, hue)
            if all(contrast(sample, bg) >= minimum + 0.015 for bg in backgrounds):
                high = mid
            else:
                low = mid
        result = color(light + (target - light) * high, chroma, hue)
        choices.append((abs(lch(result)[0] - light), result))
    if not choices:
        raise ValueError("No foreground can meet contrast on these backgrounds")
    return min(choices)[1]


def hue_distance(a, b):
    return abs((a - b + 180) % 360 - 180)
