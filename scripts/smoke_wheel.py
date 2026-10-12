"""Exercise an installed distribution outside the checkout, without source imports."""

import argparse
from importlib import metadata, resources
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import sysconfig
import tempfile


def smoke(fixture, expected_version):
    import orient
    from orient.engine import default_palette, from_image
    from orient.export import export, render
    from orient.extract import analyze, sample_color

    installed = Path(orient.__file__).resolve()
    if not installed.is_relative_to(Path(sys.prefix).resolve()):
        raise AssertionError(f"Orient must be installed in the active environment: {installed}")
    assert metadata.version("orient-colors") == orient.__version__ == expected_version
    templates = resources.files("orient") / "templates"
    names = sorted(str(p.relative_to(Path(str(templates)))) for p in Path(str(templates)).rglob("*.in"))
    assert names == [
        "alacritty.toml.in",
        "btop.theme.in",
        "discord.css.in",
        "firefox/manifest.json.in",
        "foot.ini.in",
        "gtk.css.in",
        "hyprland.lua.in",
        "kitty.conf.in",
        "neovim.lua.in",
        "obsidian.css.in",
        "rofi.rasi.in",
        "spotify/color.ini.in",
        "vscode-theme.json.in",
    ], names
    assert "spotify/color.ini.in" in names and "firefox/manifest.json.in" in names
    cli = Path(sysconfig.get_path("scripts")) / "orient"
    with tempfile.TemporaryDirectory(prefix="orient-wheel-") as folder:
        root = Path(folder)
        env = dict(os.environ, XDG_CACHE_HOME=str(root / "cache"))
        for key in ("PYTHONPATH", "DISPLAY", "WAYLAND_DISPLAY", "HYPRLAND_INSTANCE_SIGNATURE"):
            env.pop(key, None)

        def run(*args):
            result = subprocess.run(
                [str(cli), *map(str, args)], cwd=root, env=env, capture_output=True, text=True, timeout=60
            )
            if result.returncode:
                raise AssertionError(result.stderr)
            return result.stdout

        assert run("--version").strip() == orient.ENGINE_ID
        palette = json.loads(run("palette", fixture))
        assert palette["formatVersion"] == 1
        assert palette["mode"] == "dark" and palette["engine"] == orient.ENGINE_ID
        assert all(re.fullmatch(r"[0-9a-f]{6}", value) for value in palette["colours"].values())
        assert json.loads(run("analyze", fixture))["candidates"]
        palette_file = root / "palette.json"
        palette_file.write_text(json.dumps(palette))
        output = json.loads(run("export", palette_file, root / "themes"))["files"]
        assert len(output) == len(names)
        assert all(Path(path).is_file() for path in output)
        template = root / "colors.css.in"
        template.write_text(":root { --accent: #{{primary}}; }")
        assert run("render", palette_file, template) == ":root { --accent: #" + palette["colours"]["primary"] + "; }"
        # README and stable-library entry points must work with the installed wheel too.
        library_palette = from_image(fixture, mode="dark", personality="natural", cache_dir=root / "library-cache")
        assert library_palette["colours"] == palette["colours"]
        assert default_palette()["formatVersion"] == 1
        assert analyze(fixture)["candidates"]
        assert re.fullmatch(r"[0-9a-f]{6}", sample_color(fixture, 0.5, 0.5))
        assert render("#{{primary}}", library_palette) == "#" + palette["colours"]["primary"]
        assert len(export(library_palette, root / "library-themes")) == len(names)
    print(
        f"Installed wheel smoke passed: Python {sys.version.split()[0]}, Orient {expected_version}, {len(names)} templates"
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fixture", type=Path)
    parser.add_argument("--version", required=True)
    args = parser.parse_args()
    smoke(args.fixture.resolve(strict=True), args.version)
