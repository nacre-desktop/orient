"""Render application themes in both modes and parse their actual output formats.

Install tests/template-parsers.txt and fish/tmux for this suite (CI does so).
Ghostty and zathurarc have no available standalone/headless format parser in this
harness: their dedicated parser tests explicitly skip; rendering/key checks run.
"""

import configparser
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib
import unittest

import kdl
from luaparser import ast
import tinycss2
from tree_sitter_language_pack import get_parser
import yaml

from orient.engine import default_palette
from orient.export import export, render

ROOT = Path(__file__).resolve().parents[1]
TEMPLATES = ROOT / "src/orient/templates"
BINARY = Path(os.environ.get("ORIENT_BINARY", ROOT / "target/release/orient")).resolve()
TARGETS = {
    "ghostty.conf": "ghostty",
    "wezterm.lua": "lua",
    "niri.kdl": "kdl",
    "waybar.css": "css",
    "mako.conf": "mako",
    "swaync.css": "css",
    "fuzzel.ini": "ini",
    "helix.toml": "toml",
    "zed-theme.json": "json",
    "starship.toml": "toml",
    "tmux.conf": "tmux",
    "fish.fish": "fish",
    "zathurarc": "zathura",
    "qt5ct/colors/orient.conf": "ini",
    "qt6ct/colors/orient.conf": "ini",
    "hyprlock.conf": "hyprlang",
    "dunst.conf": "ini",
    "yazi-theme.toml": "toml",
    "lazygit.yml": "yaml",
    "cava.conf": "ini",
}


def samples():
    """Frozen public gallery colors, plus the defaults; never regenerate goldens."""
    gallery = json.loads((ROOT / "tests/orient-gallery/palettes.json").read_text())
    for mode in ("dark", "light"):
        yield "default/" + mode, default_palette(mode)
        for name, schemes in gallery.items():
            yield name + "/" + mode, {"mode": mode, "colours": schemes[mode + "-natural"]["colours"]}


def ini(text, header=""):
    # Keep hashes in values, disable interpolation, reject duplicate keys.
    parser = configparser.ConfigParser(interpolation=None)
    parser.read_string(header + text)
    return parser


class AdoptionTemplateTests(unittest.TestCase):
    def check_css(self, text):
        rules = tinycss2.parse_stylesheet(text, skip_comments=True, skip_whitespace=True)
        self.assertTrue(rules)
        for rule in rules:
            self.assertEqual(rule.type, "qualified-rule")
            declarations = tinycss2.parse_declaration_list(rule.content, skip_comments=True, skip_whitespace=True)
            self.assertTrue(declarations)
            for declaration in declarations:
                self.assertEqual(declaration.type, "declaration")
                self.assertIn(declaration.lower_name, {"color", "background-color", "border-color"})
                values = [token for token in declaration.value if token.type != "whitespace"]
                self.assertEqual(len(values), 1)
                self.assertEqual(values[0].type, "hash")
                self.assertRegex(values[0].value, r"^[0-9a-f]{6}$")

    def check_native(self, name, text, palette):
        executable = shutil.which(name)
        self.assertIsNotNone(executable, "Install " + name + " for the application parser tests")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "theme"
            path.write_text(text)
            # No user configuration, universal variables, or existing tmux server.
            environment = {**os.environ, "HOME": directory, "XDG_CONFIG_HOME": directory}
            environment.pop("TMUX", None)
            if name == "fish":
                result = subprocess.run(
                    [executable, "--no-config", "--no-execute", str(path)],
                    env=environment,
                    capture_output=True,
                    text=True,
                    timeout=10,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
            else:
                command = [executable, "-S", str(root / "socket"), "-f", "/dev/null"]
                try:
                    result = subprocess.run(
                        [*command, "new-session", "-d", "-s", "theme-test", "sleep 30"],
                        env=environment,
                        capture_output=True,
                        text=True,
                        timeout=10,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    result = subprocess.run(
                        [*command, "source-file", str(path)],
                        env=environment,
                        capture_output=True,
                        text=True,
                        timeout=10,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    result = subprocess.run(
                        [*command, "show-options", "-gv", "status-style"],
                        env=environment,
                        capture_output=True,
                        text=True,
                        timeout=10,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(
                        result.stdout.strip(),
                        "fg=#" + palette["colours"]["onSurface"] + ",bg=#" + palette["colours"]["surface"],
                    )
                finally:
                    subprocess.run([*command, "kill-server"], env=environment, capture_output=True, timeout=10)

    def parse_target(self, name, text, palette):
        format_name = TARGETS[name]
        if format_name == "toml":
            data = tomllib.loads(text)
            self.assertTrue(data)
            if name == "helix.toml":
                self.assertEqual(data["ui.background"]["bg"], "#" + palette["colours"]["surface"])
                self.assertEqual(data["ui.text"], "#" + palette["colours"]["onSurface"])
            if name == "yazi-theme.toml":
                self.assertEqual(set(data), {"mgr", "mode", "status", "which"})
                self.assertEqual(data["mode"]["normal_main"]["fg"], "#" + palette["colours"]["onPrimary"])
        elif format_name in {"ini", "mako"}:
            data = ini(text, "[global]\n" if format_name == "mako" else "")
            self.assertTrue(data.sections())
            if name == "fuzzel.ini":
                for value in data["colors"].values():
                    self.assertRegex(value, r"^[0-9a-f]{8}$")
                self.assertEqual(data["colors"]["selection-text"], palette["colours"]["onPrimaryContainer"] + "ff")
            if name.startswith(("qt5ct/", "qt6ct/")):
                self.assertEqual(set(data["ColorScheme"]), {"active_colors", "disabled_colors", "inactive_colors"})
                for group, row in data["ColorScheme"].items():
                    colors = [color.strip() for color in row.split(",")]
                    self.assertEqual(len(colors), 21)
                    for color in colors:
                        self.assertRegex(color, r"^#ff[0-9a-f]{6}$")
                    self.assertEqual(colors[12], "#ff" + palette["colours"]["primary"])
                    self.assertEqual(colors[13], "#ff" + palette["colours"]["onPrimary"])
                    self.assertEqual(colors[9], "#ff" + palette["colours"]["surface"])
                    role = "onSurfaceVariant" if group == "disabled_colors" else "onSurface"
                    self.assertEqual(colors[6], "#ff" + palette["colours"][role])
            if name == "dunst.conf":
                self.assertEqual(set(data.sections()), {"urgency_low", "urgency_normal", "urgency_critical"})
            if name == "cava.conf":
                self.assertEqual(data["color"]["gradient_count"], "3")
                self.assertEqual(data["color"]["gradient"], "1")
        elif format_name == "json":
            data = json.loads(text)
            self.assertEqual(set(data) - {"$schema", "_comment"}, {"name", "author", "themes"})
            theme = data["themes"][0]
            self.assertEqual(theme["appearance"], palette["mode"])
            self.assertEqual(theme["style"]["editor.foreground"], "#" + palette["colours"]["onSurface"])
            self.assertEqual(len(theme["style"]["players"]), 1)
            for suffix in ("black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"):
                self.assertIn("terminal.ansi." + suffix, theme["style"])
                self.assertIn("terminal.ansi.bright_" + suffix, theme["style"])
        elif format_name == "yaml":
            data = yaml.safe_load(text)
            self.assertEqual(set(data), {"gui"})
            self.assertEqual(data["gui"]["theme"]["defaultFgColor"], ["#" + palette["colours"]["onSurface"]])
        elif format_name == "lua":
            self.assertIsNotNone(ast.parse(text))
        elif format_name == "kdl":
            document = kdl.parse(text)
            self.assertEqual([node.name for node in document.nodes], ["layout"])
            self.assertEqual([node.name for node in document.nodes[0].nodes], ["focus-ring", "border"])
            for node in document.nodes[0].nodes:
                self.assertEqual(
                    [child.name for child in node.nodes], ["active-color", "inactive-color", "urgent-color"]
                )
                self.assertEqual(node.nodes[0].args, ["#" + palette["colours"]["primary"]])
        elif format_name == "css":
            self.check_css(text)
        elif format_name == "hyprlang":
            tree = get_parser("hyprlang").parse(text.encode())
            self.assertFalse(tree.root_node.has_error, str(tree.root_node))
            self.assertIn("input-field", text)
        elif format_name in {"fish", "tmux"}:
            self.check_native(format_name, text, palette)
        else:
            self.fail("No parser registered: " + name)

    def test_all_requested_templates_render_in_both_modes(self):
        for label, palette in samples():
            for name in TARGETS:
                with self.subTest(palette=label, target=name):
                    template = (TEMPLATES / (name + ".in")).read_text()
                    self.assertIn("Docs: https://", "\n".join(template.splitlines()[:3]))
                    text = render(template, palette)
                    self.assertNotRegex(text, r"\{\{|<\*")
                    self.assertNotRegex(text, r"/home/|/Users/")
                    if name == "ghostty.conf":
                        colors = re.findall(r"^palette = (\d+)=(#[0-9a-f]{6})$", text, re.MULTILINE)
                        self.assertEqual(
                            colors, [(str(i), "#" + palette["colours"]["term" + str(i)]) for i in range(16)]
                        )
                    if name == "zathurarc":
                        settings = re.findall(r'^set ([\w-]+) "(#[0-9a-f]{6})"$', text, re.MULTILINE)
                        self.assertEqual(len(settings), 18)
                        self.assertEqual(dict(settings)["default-fg"], "#" + palette["colours"]["onSurface"])

    @unittest.skip(
        "Ghostty key=value has no available standalone parser here; rendering and 16 ANSI values are checked"
    )
    def test_ghostty_format_parser(self):
        pass

    @unittest.skip(
        "zathurarc has no available headless application parser here; rendering and documented set keys are checked"
    )
    def test_zathura_format_parser(self):
        pass

    @unittest.skipUnless(BINARY.is_file(), "Build orient-cli to verify embedded template exports")
    def test_native_exports_match_python_for_every_sample_and_template(self):
        expected = {path.relative_to(TEMPLATES).with_suffix("") for path in TEMPLATES.rglob("*.in")}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for index, (label, palette) in enumerate(samples()):
                with self.subTest(palette=label):
                    palette_path = root / "palette.json"
                    palette_path.write_text(json.dumps(palette))
                    output = root / str(index)
                    result = subprocess.run(
                        [str(BINARY), "export", str(palette_path), str(output)],
                        capture_output=True,
                        text=True,
                        timeout=30,
                        env={**os.environ, "XDG_CACHE_HOME": str(root / "cache")},
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(
                        {Path(p).relative_to(output) for p in json.loads(result.stdout)["files"]}, expected
                    )
                    python_output = root / (str(index) + "-python")
                    export(palette, python_output)
                    for path in expected:
                        self.assertEqual((output / path).read_text(), (python_output / path).read_text(), str(path))


def parser_test(name):
    def test(self):
        for label, palette in samples():
            with self.subTest(palette=label):
                self.parse_target(name, render((TEMPLATES / (name + ".in")).read_text(), palette), palette)

    return test


for target, format_name in TARGETS.items():
    if format_name not in {"ghostty", "zathura"}:
        setattr(AdoptionTemplateTests, "test_parse_" + re.sub(r"\W", "_", target), parser_test(target))
