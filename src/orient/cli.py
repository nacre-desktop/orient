"""Standalone, read-only Orient CLI. Publication belongs to the caller."""

import argparse
import json
from pathlib import Path
import sys
from . import ENGINE_ID
from .engine import from_image
from .extract import analyze, sample_color
from .palette import PERSONALITIES
from .export import export, render


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", action="version", version=ENGINE_ID)
    commands = parser.add_subparsers(dest="command", required=True)
    analysis = commands.add_parser("analyze")
    analysis.add_argument("image")
    sampling = commands.add_parser("sample")
    sampling.add_argument("image")
    sampling.add_argument("x", type=float)
    sampling.add_argument("y", type=float)
    palette = commands.add_parser("palette")
    palette.add_argument("image")
    palette.add_argument("--mode", choices=("dark", "light"), default="dark")
    palette.add_argument("--personality", choices=PERSONALITIES, default="natural")
    palette.add_argument("--background-from-wallpaper", action=argparse.BooleanOptionalAction, default=None)
    palette.add_argument("--accent")
    for role in ("primary", "secondary", "tertiary"):
        palette.add_argument("--" + role)
    palette.add_argument("--brightness", type=float, default=0.0)
    palette.add_argument("--saturation", type=float, default=1.0)
    palette.add_argument("--hour", type=int, default=12)
    exports = commands.add_parser("export")
    exports.add_argument("palette", type=Path)
    exports.add_argument("directory", type=Path)
    exports.add_argument("--templates", type=Path)
    rendering = commands.add_parser("render", help="render an Orient or Matugen color template to stdout")
    rendering.add_argument("palette", type=Path)
    rendering.add_argument("template", type=Path)
    rendering.add_argument("--companion", type=Path, help="opposite-mode palette JSON")
    rendering.add_argument("--variables", type=Path, help="JSON object for custom.KEYWORD tokens")
    args = parser.parse_args(argv)
    try:
        if args.command == "analyze":
            result = analyze(args.image)
        elif args.command == "sample":
            result = {"hex": sample_color(args.image, args.x, args.y)}
        elif args.command == "render":
            palette_data = json.loads(args.palette.read_text())
            schemes = {}
            if args.companion:
                companion = json.loads(args.companion.read_text())
                if companion.get("mode") not in ("dark", "light") or companion["mode"] == palette_data["mode"]:
                    raise ValueError("Companion palette must use the opposite mode")
                schemes[companion["mode"]] = companion["colours"]
            custom = json.loads(args.variables.read_text()) if args.variables else None
            if custom is not None and not isinstance(custom, dict):
                raise ValueError("Custom variables must be a JSON object")
            sys.stdout.write(render(args.template.read_text(), palette_data, schemes=schemes, custom=custom))
            return 0
        elif args.command == "export":
            result = {"files": export(json.loads(args.palette.read_text()), args.directory, args.templates)}
        else:
            result = from_image(
                args.image,
                mode=args.mode,
                personality=args.personality,
                background_from_wallpaper=args.background_from_wallpaper,
                accent=args.accent,
                overrides={
                    role: getattr(args, role) for role in ("primary", "secondary", "tertiary") if getattr(args, role)
                },
                brightness=args.brightness,
                saturation=args.saturation,
                hour=args.hour,
            )
        print(json.dumps(result))
        return 0
    except (OSError, ValueError, KeyError) as error:
        print("orient: " + str(error), file=sys.stderr)
        return 1
