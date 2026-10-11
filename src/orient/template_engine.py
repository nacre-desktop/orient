"""Pure role rendering with Matugen color-token compatibility; no code execution."""

import ast
import colorsys
import math
import re

TOKEN = re.compile(r"(\\)?\{\{([^{}]*?)\}\}")
PIECE = re.compile(r"(\\)?\{\{([^{}]*?)\}\}|<\*([\s\S]*?)\*>")
MAX_TEMPLATE = 1024 * 1024
MAX_OUTPUT = 8 * MAX_TEMPLATE
COLOR = re.compile(r"[0-9a-fA-F]{6}")


def split_quoted(text, separator):
    parts, start, quote, escaped = [], 0, None, False
    for index, char in enumerate(text):
        if escaped:
            escaped = False
        elif char == "\\" and quote:
            escaped = True
        elif quote:
            if char == quote:
                quote = None
        elif char in "\"'":
            quote = char
        elif char == separator:
            parts.append(text[start:index].strip())
            start = index + 1
    if quote:
        raise ValueError("Unclosed template string")
    return parts + [text[start:].strip()]


def literal(text):
    if text.startswith(('"', "'")):
        try:
            value = ast.literal_eval(text)
        except (ValueError, SyntaxError) as error:
            raise ValueError("Invalid template string argument") from error
        if isinstance(value, str):
            return value
    elif text in ("hsl", "hsv"):
        return text
    elif re.fullmatch(r"[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?", text):
        value = float(text)
        if math.isfinite(value):
            return value
    raise ValueError("Expected a finite number or quoted string argument: " + text)


def number(value):
    return f"{value:.3f}".rstrip("0").rstrip(".")


def clamp(value, low=0.0, high=1.0):
    return min(high, max(low, value))


def css_color(text):
    """Return normalized RGBA channels and the original serialization format."""
    if re.fullmatch(r"#?[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?", text):
        raw = text.removeprefix("#")
        rgb = [int(raw[i : i + 2], 16) / 255 for i in (0, 2, 4)]
        alpha = int(raw[6:], 16) / 255 if len(raw) == 8 else 1.0
        fmt = "hex_alpha" if len(raw) == 8 else "hex" if text.startswith("#") else "hex_stripped"
        return (*rgb, alpha), fmt
    match = re.fullmatch(r"(rgba?|hsla?)\(([^()]*)\)", text)
    if not match:
        raise ValueError("Color filter requires a CSS color expression")
    fmt, body = match.groups()
    parts = [part.strip() for part in body.split(",")]
    if len(parts) != (4 if fmt.endswith("a") else 3):
        raise ValueError("Invalid CSS color channels")
    try:
        raw_channels = [float(part.removesuffix("%")) for part in parts]
        if not all(math.isfinite(channel) for channel in raw_channels):
            raise ValueError("Non-finite CSS color channel")
        if fmt.startswith("rgb"):
            rgb = [clamp(channel / 255) for channel in raw_channels[:3]]
        else:
            if not all(part.endswith("%") for part in parts[1:3]):
                raise ValueError("HSL channels require percentages")
            hue = float(parts[0]) % 360 / 360
            saturation, lightness = [clamp(float(part[:-1]) / 100) for part in parts[1:3]]
            rgb = colorsys.hls_to_rgb(hue, lightness, saturation)
        alpha = clamp(float(parts[3])) if len(parts) == 4 else 1.0
        channels = (*rgb, alpha)
        if not all(math.isfinite(channel) for channel in channels):
            raise ValueError("Non-finite CSS color channel")
    except ValueError as error:
        raise ValueError("Invalid CSS color: " + text) from error
    return channels, fmt


def serialize(channels, fmt):
    red, green, blue, alpha = channels
    rgb = [int(clamp(channel) * 255 + 0.5 + 1e-9) for channel in (red, green, blue)]
    if fmt.startswith("hex"):
        text = "".join(f"{channel:02x}" for channel in rgb)
        if fmt == "hex_alpha":
            return "#" + (text + f"{int(clamp(alpha) * 255 + 0.5):02x}").upper()
        return ("#" if fmt == "hex" else "") + text
    if fmt.startswith("rgb"):
        body = ", ".join(map(str, rgb))
    else:
        hue, lightness, saturation = colorsys.rgb_to_hls(red, green, blue)
        body = f"{number(hue * 360)}, {number(saturation * 100)}%, {number(lightness * 100)}%"
    if fmt.endswith("a"):
        body += ", " + number(alpha)
    return fmt + "(" + body + ")"


def apply_filter(value, expression):
    name, separator, raw = expression.partition(":")
    name = name.strip()
    args = [literal(arg) for arg in split_quoted(raw, ",")] if separator else []
    if name == "replace":
        if len(args) != 2 or not all(isinstance(arg, str) for arg in args):
            raise ValueError("replace requires two quoted strings")
        count = value.count(args[0])
        if len(value) + count * (len(args[1]) - len(args[0])) > 8 * 1024 * 1024:
            raise ValueError("Template replacement exceeds the output limit")
        return value.replace(*args)
    if name not in ("set_alpha", "lighten", "auto_lightness", "saturate", "set_lightness"):
        raise ValueError("Unsupported Matugen filter: " + name)
    expected = (1, 2) if name == "saturate" else (1,)
    if len(args) not in expected or not isinstance(args[0], float):
        raise ValueError(name + " requires a numeric argument")
    if name == "saturate" and len(args) == 2 and args[1] not in ("hsl", "hsv"):
        raise ValueError("saturate color space must be hsl or hsv")
    channels, fmt = css_color(value)
    red, green, blue, alpha = channels
    amount = args[0]
    if name == "set_alpha":
        if fmt not in ("rgba", "hsla", "hex_alpha"):
            raise ValueError("set_alpha requires rgba, hsla or hex_alpha format")
        return serialize((red, green, blue, clamp(amount)), fmt)
    if name == "saturate" and len(args) == 2 and args[1] == "hsv":
        hue, saturation, brightness = colorsys.rgb_to_hsv(red, green, blue)
        rgb = colorsys.hsv_to_rgb(hue, clamp(saturation + amount / 100), brightness)
    else:
        hue, lightness, saturation = colorsys.rgb_to_hls(red, green, blue)
        if name == "saturate":
            saturation = clamp(saturation + amount / 100)
        elif name == "set_lightness":
            lightness = clamp(amount / 100)
        else:
            if name == "auto_lightness" and lightness >= 0.5:
                amount = -amount
            lightness = clamp(lightness + amount / 100)
        rgb = colorsys.hls_to_rgb(hue, lightness, saturation)
    return serialize((*rgb, alpha), fmt)


def role_name(name, values):
    if name == "source_color":
        return "overtone"
    if name in values:
        return name
    parts = name.split("_")
    return parts[0] + "".join(part[:1].upper() + part[1:] for part in parts[1:])


def format_color(value, format_name):
    if not isinstance(value, str) or not COLOR.fullmatch(value):
        raise ValueError("Missing or invalid template color")
    value = value.lower()
    red, green, blue = (int(value[i : i + 2], 16) for i in (0, 2, 4))
    hue, lightness, saturation = colorsys.rgb_to_hls(red / 255, green / 255, blue / 255)

    def number(n):
        return f"{n:.3f}".rstrip("0").rstrip(".")

    formats = {
        "hex": "#" + value,
        "hex_stripped": value,
        "hex_alpha": "#" + value.upper() + "FF",
        "rgb": f"rgb({red}, {green}, {blue})",
        "rgba": f"rgba({red}, {green}, {blue}, 1.0)",
        "red": str(red),
        "green": str(green),
        "blue": str(blue),
        "alpha": "255",
        "hsl": f"hsl({number(hue * 360)}, {number(saturation * 100)}%, {number(lightness * 100)}%)",
        "hsla": f"hsla({number(hue * 360)}, {number(saturation * 100)}%, {number(lightness * 100)}%, 1.0)",
        "hue": number(hue * 360),
        "saturation": number(saturation * 100),
        "lightness": number(lightness * 100),
    }
    if format_name not in formats:
        raise ValueError("Unsupported Matugen color format: " + format_name)
    return formats[format_name]


def public_colors(values):
    """The Matugen role namespace, excluding Orient/terminal/toolkit aliases."""
    result = {}
    for role, value in values.items():
        if not isinstance(value, str) or not COLOR.fullmatch(value):
            continue
        name = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", role).lower()
        if name.startswith(
            ("primary", "secondary", "tertiary", "error", "on_", "surface", "outline", "inverse", "neutral")
        ) or name in ("background", "shadow", "scrim"):
            result[name] = role
    if "overtone" in values:
        result["source_color"] = "overtone"
    return sorted(result.items())


def requested_modes(template):
    modes = set()
    for match in TOKEN.finditer(template):
        if match[1]:
            continue
        base = split_quoted(match[2].strip(), "|")[0]
        base = re.sub(r"\s*\.\s*", ".", base)
        request = re.fullmatch(r"[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)?\.(dark|light)\.[A-Za-z_]\w*", base)
        if request:
            modes.add(request[1])
    return modes


def requested_contexts(template):
    result = set()
    for match in TOKEN.finditer(template):
        if not match[1]:
            base = split_quoted(match[2].strip(), "|")[0]
            name = re.split(r"\s*\.\s*", base)[0]
            if name in ("palettes", "base16"):
                result.add(name)
    for match in PIECE.finditer(template):
        if match[3] is not None:
            loop = re.fullmatch(r"for\s+\w+\s*,\s*\w+\s+in\s+(palettes|base16)(?:\.\w+)*", match[3].strip())
            if loop:
                result.add(loop[1])
    return result


def parse_template(template):
    if len(template) > MAX_TEMPLATE:
        raise ValueError("Template exceeds the 1 MiB input limit")
    pieces, position = [], 0
    for match in PIECE.finditer(template):
        plain = template[position : match.start()]
        if "{{" in plain or "<*" in plain or "{%" in plain:
            raise ValueError("Malformed or unsupported template expression/block")
        pieces.append(("text", plain))
        if match[3] is not None:
            pieces.append(("block", match[3].strip()))
        else:
            pieces.append(("text" if match[1] else "token", "{{" + match[2] + "}}" if match[1] else match[2]))
        position = match.end()
    tail = template[position:]
    if any(marker in tail for marker in ("{{", "<*", "{%")):
        raise ValueError("Malformed or unsupported template expression/block")
    pieces.append(("text", tail))

    def group(index, depth=0, loop_depth=0, stops=()):
        if depth > 16 or loop_depth > 4:
            raise ValueError("Template block/loop nesting exceeds its limit")
        nodes = []
        while index < len(pieces):
            kind, value = pieces[index]
            index += 1
            if kind != "block":
                nodes.append((kind, value))
                continue
            if value in ("endfor", "else", "endif"):
                if value not in stops:
                    raise ValueError("Unexpected " + value + " block")
                return nodes, index, value
            condition = re.fullmatch(r"if\s+\{\{\s*([^{}]+?)\s*\}\}", value)
            if condition:
                yes, index, ending = group(index, depth + 1, loop_depth, ("else", "endif"))
                no = []
                if ending == "else":
                    no, index, ending = group(index, depth + 1, loop_depth, ("endif",))
                nodes.append(("if", (condition[1].strip(), yes, no)))
                continue
            loop = re.fullmatch(
                r"for\s+([A-Za-z_]\w*)\s*,\s*([A-Za-z_]\w*)\s+in\s+([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)", value
            )
            if not loop:
                raise ValueError("Unsupported Matugen block: " + value)
            names = loop.group(1, 2)
            if names[0] == names[1] or any(
                name in ("colors", "palettes", "base16", "custom", "mode", "image", "is_dark_mode", "loop")
                for name in names
            ):
                raise ValueError("Loop bindings must be distinct non-reserved names")
            child, index, _ = group(index, depth + 1, loop_depth + 1, ("endfor",))
            nodes.append(("loop", (names, loop[3], child)))
        if stops:
            raise ValueError("Missing " + "/".join(stops) + " block")
        return nodes, index, None

    return group(0)[0]


def render_values(template, values, schemes=None, image=None, custom=None, data=None):
    """Finite color loops and filters; no evaluation, hooks or filesystem access."""
    mode = values.get("mode")
    contexts = dict(schemes or {})
    if mode in ("dark", "light"):
        contexts[mode] = values
    contexts["default"] = values
    roots = {
        "colors": {
            name: {mode: context[role] for mode, context in contexts.items() if role in context}
            for name, role in public_colors(values)
        }
    }
    for namespace, mapping in (data or {}).items():
        if namespace not in ("palettes", "base16") or not isinstance(mapping, dict):
            raise ValueError("Unsupported template map context: " + namespace)
        roots[namespace] = mapping
    nodes = parse_template(template)

    def lookup(expression, scope):
        parts = [part.strip() for part in expression.split(".")]
        value = scope.get(parts[0], roots.get(parts[0]))
        if value is None:
            raise ValueError("Missing template map context: " + parts[0])
        for part in parts[1:]:
            if isinstance(value, dict):
                if part not in value:
                    if part in ("dark", "light"):
                        raise ValueError("Template requires the " + part + " palette; supply a companion palette")
                    raise ValueError("Missing template map value: " + expression)
                value = value[part]
            elif isinstance(value, str) and COLOR.fullmatch(value):
                value = format_color(value, part)
            else:
                raise ValueError("Invalid template map expression: " + expression)
        return value

    def color(role, scheme, fmt):
        if scheme not in ("default", "dark", "light"):
            raise ValueError("Expected default, dark or light color scheme")
        context = contexts.get(scheme)
        if context is None:
            raise ValueError("Template requires the " + scheme + " palette; supply a companion palette")
        return format_color(context.get(role_name(role, context)), fmt)

    def base(expression, scope):
        if expression.startswith(('"', "'")):
            return literal(expression)
        if expression == "mode" and mode in ("dark", "light"):
            return mode
        if expression == "is_dark_mode" and mode in ("dark", "light"):
            return "true" if mode == "dark" else "false"
        if expression == "image":
            if not isinstance(image, str) or "\0" in image:
                raise ValueError("Template requests image but no source image was supplied")
            return image
        if expression.startswith("custom."):
            name = expression[7:]
            value = (custom or {}).get(name)
            if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name) or not isinstance(value, (str, int, float, bool)):
                raise ValueError("Missing or unsupported custom keyword: " + name)
            return str(value).lower() if isinstance(value, bool) else str(value)
        parts = [part.strip() for part in expression.split(".")]
        if parts[0] in scope or parts[0] in ("palettes", "base16"):
            value = lookup(expression, scope)
            if isinstance(value, bool):
                return "true" if value else "false"
            if isinstance(value, (str, int)):
                return str(value)
            raise ValueError("Template expression must produce a scalar: " + expression)
        if parts[0] == "colors":
            if len(parts) == 3:
                parts.insert(2, "default")
            if len(parts) != 4:
                raise ValueError("Expected colors.ROLE.default|dark|light.FORMAT: " + expression)
            return color(parts[1], parts[2], parts[3])
        if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*", expression):
            raise ValueError("Unsupported template expression: " + expression)
        value = values.get(expression)
        if not isinstance(value, str) or not COLOR.fullmatch(value):
            raise ValueError("Missing or invalid color template token: " + expression)
        return value.lower()

    def evaluate(expression, scope):
        pipeline = split_quoted(expression.strip(), "|")
        value = base(pipeline[0], scope)
        for step in pipeline[1:]:
            value = apply_filter(value, step)
        return value

    output, length, operations = [], 0, 0

    def emit(text):
        nonlocal length
        length += len(text)
        if length > MAX_OUTPUT:
            raise ValueError("Template expansion exceeds the 8 MiB output limit")
        output.append(text)

    def visit(nodes, scope):
        nonlocal operations
        for kind, value in nodes:
            operations += 1
            if operations > 100000:
                raise ValueError("Template exceeds the 100000-operation expansion limit")
            if kind == "text":
                emit(value)
            elif kind == "token":
                emit(evaluate(value, scope))
            elif kind == "if":
                expression, yes, no = value
                if expression == "is_dark_mode" and mode not in ("dark", "light"):
                    raise ValueError("Template condition requires a light/dark mode")
                result = mode == "dark" if expression == "is_dark_mode" else lookup(expression, scope)
                if not isinstance(result, bool):
                    raise ValueError("Template condition must be a boolean: " + expression)
                visit(yes if result else no, scope)
            else:
                names, expression, child = value
                mapping = lookup(expression, scope)
                if not isinstance(mapping, dict):
                    raise ValueError("Template loop requires a map: " + expression)
                for index, (name, item) in enumerate(mapping.items()):
                    metadata = {"last": index == len(mapping) - 1, "first": index == 0}
                    visit(child, dict(scope, **{names[0]: name, names[1]: item, "loop": metadata}))

    visit(nodes, {})
    return "".join(output)
