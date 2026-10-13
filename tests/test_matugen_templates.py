"""Color-template compatibility, exact mode context and read-only CLI/export."""

import json
import os
from pathlib import Path
import sys
import tempfile
import subprocess
import unittest

from orient.template_engine import render_values
from orient.engine import default_palette, from_image
from orient.export import render, export
from PIL import Image


class MatugenTemplateTests(unittest.TestCase):
    def test_standard_roles_formats_whitespace_and_legacy_tokens(self):
        values = {
            "primary": "224466",
            "onPrimaryContainer": "ffffff",
            "overtone": "335577",
            "mode": "dark",
        }
        self.assertEqual(
            render_values(
                "#{{primary}} {{ colors.on_primary_container.default.hex }} {{colors.source_color.default.hex_stripped}}",
                values,
            ),
            "#224466 #ffffff 335577",
        )
        self.assertEqual(
            render_values(
                "{{ colors.primary.default.rgb }} / {{ colors.primary.default.rgba }}",
                values,
            ),
            "rgb(34, 68, 102) / rgba(34, 68, 102, 1.0)",
        )
        self.assertEqual(
            render_values(
                "{{ colors.primary.default.red }},{{ colors.primary.default.green }},{{ colors.primary.default.blue }},{{ colors.primary.default.alpha }}",
                values,
            ),
            "34,68,102,255",
        )
        self.assertEqual(
            render_values("{{colors.primary.default.hsl}} {{colors.primary.default.hsla}}", values),
            "hsl(210, 50%, 26.667%) hsla(210, 50%, 26.667%, 1.0)",
        )
        self.assertEqual(render_values("{{ colors.primary.hex }}", values), "#224466")
        self.assertEqual(
            render_values(
                "{{mode}} {{is_dark_mode}} {{custom.font}} {{image}}",
                values,
                image="/tmp/photo.png",
                custom={"font": "Noto Sans"},
            ),
            "dark true Noto Sans /tmp/photo.png",
        )

    def test_explicit_modes_are_never_silently_substituted(self):
        values = {"primary": "224466", "mode": "dark"}
        text = "{{colors.primary.dark.hex}} {{colors.primary.light.hex}}"
        with self.assertRaisesRegex(ValueError, "companion"):
            render_values(text, values)
        self.assertEqual(
            render_values(text, values, schemes={"light": {"primary": "aabbcc"}}),
            "#224466 #aabbcc",
        )

    def test_escaped_tokens_json_braces_and_unsupported_logic(self):
        values = {"primary": "224466", "mode": "dark"}
        result = render_values('{"nested":{"accent":"{{ colors.primary.default.hex }}"}}', values)
        self.assertEqual(json.loads(result)["nested"]["accent"], "#224466")
        self.assertEqual(
            render_values(r"\{{ colors.primary.default.hex }}", values),
            "{{ colors.primary.default.hex }}",
        )
        for text in (
            "<* include 'secret' *>",
            "{{colors.primary.default.hex | invert}}",
            "{{ colors.nope.default.hex }}",
            "{{palettes.primary._99.hex}}",
            "{{ colors.primary.default.xyz }}",
            "{{ {{primary}} }}",
            "{{broken",
        ):
            with self.subTest(text=text), self.assertRaises(ValueError):
                render_values(text, values)

    def test_color_map_loops_and_chained_name_filters(self):
        values = {
            "primary": "336699",
            "onSurface": "abcdef",
            "overtone": "112233",
            "term0": "445566",
            "mode": "dark",
        }
        template = '<* for label, pigment in colors *>${{label | replace: "_", "-"}}=rgba({{pigment.default.hex_stripped}}ff);<* endfor *>'
        self.assertEqual(
            render_values(template, values),
            "$on-surface=rgba(abcdefff);$primary=rgba(336699ff);$source-color=rgba(112233ff);",
        )
        self.assertNotIn("term0", render_values(template, values))
        self.assertEqual(
            render_values('{{ "a|b,c" | replace: "|", "," | replace: ",", ":" }}', values),
            "a:b:c",
        )
        self.assertEqual(
            render_values(
                "<* for n, v in colors *>{{n}}<* endfor *><* for n, v in colors *>{{v.dark.hex}}<* endfor *>",
                values,
            ),
            "on_surfaceprimarysource_color#abcdef#336699#112233",
        )

    def test_five_filters_match_public_black_box_examples(self):
        values = {"primary": "336699", "secondary": "ffdad4", "mode": "dark"}
        cases = {
            "hex | lighten: 20": "#6699cc",
            "hex | lighten: -20": "#1a334d",
            "hex | auto_lightness: 20": "#6699cc",
            'hex | saturate: 20, "hsl"': "#1f66ad",
            "hex | saturate: -20, hsl": "#476685",
            'hex | saturate: 20, "hsv"': "#145799",
            "rgba | set_alpha: 0.2": "rgba(51, 102, 153, 0.2)",
            "hex_alpha | set_alpha: 0.2": "#33669933",
            "rgba | set_alpha: 0.2 | lighten: 20": "rgba(102, 153, 204, 0.2)",
            "rgba | set_lightness: 60": "rgba(102, 153, 204, 1)",
            "hsla | set_alpha: 0.5": "hsla(210, 50%, 40%, 0.5)",
        }
        for expression, expected in cases.items():
            with self.subTest(expression=expression):
                self.assertEqual(
                    render_values("{{colors.primary.default." + expression + "}}", values),
                    expected,
                )
        self.assertEqual(
            render_values("{{colors.secondary.default.hex | auto_lightness:20}}", values),
            "#ff826e",
        )
        self.assertEqual(
            render_values("{{colors.primary.default.hex|lighten:500}}", values),
            "#ffffff",
        )
        self.assertEqual(
            render_values("{{colors.primary.default.hex|lighten:-500}}", values),
            "#000000",
        )

    def test_loop_modes_and_filter_errors_are_explicit(self):
        values = {"primary": "336699", "mode": "dark"}
        template = "<* for name, value in colors *>{{value . light . hex | lighten: 5}}<* endfor *>"
        with self.assertRaisesRegex(ValueError, "companion"):
            render_values(template, values)
        self.assertEqual(
            render_values(template, values, schemes={"light": {"primary": "6699cc"}}),
            "#79a6d2",
        )
        for text in (
            "<* endfor *>",
            "<* for n, v in colors *>missing end",
            "<* for n, v in base16 *>{{n}}<* endfor *>",
            "<* for n, n in colors *><* endfor *>",
            "{{primary | set_alpha: .5}}",
            "{{primary | saturate: 20, rgb}}",
            "{{primary | lighten: nan}}",
            "{{primary | lighten: 1e999}}",
            '{{primary | replace: "x"}}',
            '{{primary | replace: "unclosed}}',
            "{{primary | exec: 'touch /tmp/file'}}",
            "{{primary | lighten: 20 + 10}}",
        ):
            with self.subTest(text=text), self.assertRaises(ValueError):
                render_values(text, values)
        with self.assertRaisesRegex(ValueError, "input limit"):
            render_values("x" * (1024 * 1024 + 1), values)
        nested = "<* for n,v in colors *>" * 5 + "x" + "<* endfor *>" * 5
        with self.assertRaisesRegex(ValueError, "nesting"):
            render_values(nested, values)

    def test_expansion_and_replacement_are_bounded(self):
        values = {"primary" + str(i): "336699" for i in range(20)}
        values["mode"] = "dark"
        nested = "<* for n,v in colors *>" * 4 + "x" + "<* endfor *>" * 4
        with self.assertRaisesRegex(ValueError, "operation"):
            render_values(nested, values)
        with self.assertRaisesRegex(ValueError, "output limit"):
            render_values(
                '{{custom.label | replace:"x","xxxx"}}',
                values,
                custom={"label": "x" * (3 * 1024 * 1024)},
            )

    def test_loop_companion_query_and_failed_export_are_read_only(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            image = root / "scene.png"
            Image.new("RGB", (32, 32), "#336699").save(image)
            palette = from_image(image, cache_dir=root / "cache")
            template = "<* for name, pigment in colors *>{{pigment.light.hex | lighten:10}}<* endfor *>"
            actual = render(template, palette)
            companion = from_image(image, "light", cache_dir=root / "cache")
            self.assertEqual(
                actual,
                render(template, palette, schemes={"light": companion["colours"]}),
            )
            templates = root / "templates"
            templates.mkdir()
            (templates / "first.css.in").write_text(template)
            (templates / "last.css.in").write_text("{{primary | unsupported}}")
            target = root / "export"
            with self.assertRaises(ValueError):
                export(palette, target, templates)
            self.assertFalse(target.exists())

    def test_last_first_checks_generate_valid_json(self):
        template = '<* if {{is_dark_mode}} *>[<* for name, value in colors *>"{{name}}"<* if {{loop.last}} *><* else *>,<* endif *><* endfor *>]<* else *>"light"<* endif *>'
        self.assertEqual(
            json.loads(
                render_values(
                    template,
                    {"primary": "336699", "onSurface": "eeeeee", "mode": "dark"},
                )
            ),
            ["on_surface", "primary"],
        )
        self.assertEqual(
            json.loads(render_values(template, {"primary": "336699", "mode": "light"})),
            "light",
        )
        self.assertEqual(
            render_values(
                "<* for n,v in colors *><* if {{loop.first}} *>first<* else *>next<* endif *><* endfor *>",
                {"primary": "336699", "mode": "dark"},
            ),
            "first",
        )
        self.assertEqual(render_values("<* for n,v in colors *>x<* endfor *>", {"mode": "dark"}), "")

    def test_nested_map_loops_restore_outer_last_state(self):
        data = {
            "palettes": {
                "first": {"_0": "000000", "_100": "ffffff"},
                "second": {"_0": "112233"},
            }
        }
        template = '[<* for family, shades in palettes *>[<* for tone, pigment in shades *>"{{family}}{{tone}}={{pigment.hex}}"<* if {{loop.last}} *><* else *>,<* endif *><* endfor *>]<* if {{loop.last}} *><* else *>,<* endif *><* endfor *>]'
        result = json.loads(render_values(template, {"mode": "dark"}, data=data))
        self.assertEqual(result, [["first_0=#000000", "first_100=#ffffff"], ["second_0=#112233"]])
        with self.assertRaisesRegex(ValueError, "context"):
            render_values(template, {"mode": "dark"})

    def test_export_contexts_have_complete_tones_and_base16_modes(self):
        dark = default_palette("dark")
        light = default_palette("light")
        before = json.dumps(dark, sort_keys=True)
        template = '{"tones":[<* for family, shades in palettes *><* for tone, pigment in shades *><* if {{loop.first}} *><* else *>,<* endif *>"{{pigment.hex}}"<* endfor *><* if {{loop.last}} *><* else *>,<* endif *><* endfor *>],"base":[<* for name, color in base16 *>"{{color.light.hex}}"<* if {{loop.last}} *><* else *>,<* endif *><* endfor *>]}'
        result = json.loads(render(template, dark, schemes={"light": light["colours"]}))
        self.assertEqual(len(result["tones"]), 606)
        self.assertEqual(len(result["base"]), 16)
        self.assertEqual(result["tones"][0], "#000000")
        self.assertEqual(result["tones"][100], "#ffffff")
        self.assertEqual(result["base"][0], "#" + light["colours"]["surface"])
        self.assertEqual(before, json.dumps(dark, sort_keys=True))
        self.assertEqual(
            render("{{base16.base05.dark.hex}}", dark),
            "#" + dark["colours"]["onSurface"],
        )
        with self.assertRaisesRegex(ValueError, "companion"):
            render("{{base16.base05.light.hex}}", dark)
        from orient.colour import lch

        self.assertGreater(
            lch(render("{{base16.base07.default.hex_stripped}}", dark))[0],
            lch(dark["colours"]["onSurface"])[0],
        )
        self.assertLess(
            lch(render("{{base16.base07.default.hex_stripped}}", light))[0],
            lch(light["colours"]["onSurface"])[0],
        )

    def test_malformed_conditions_and_untrusted_maps_fail(self):
        values = {"primary": "336699", "mode": "dark"}
        for template in (
            "<* else *>",
            "<* endif *>",
            "<* if {{is_dark_mode}} *>missing",
            "<* if {{mode}} *>x<* endif *>",
            "<* if {{loop.last}} *>x<* endif *>",
            "<* if {{is_dark_mode}} *><* else *><* else *><* endif *>",
            "<* for n,v in custom *>x<* endfor *>",
        ):
            with self.subTest(template=template), self.assertRaises(ValueError):
                render_values(template, values)
        with self.assertRaises(ValueError):
            render_values("text", values, data={"custom": {}})

    def test_extra_contexts_are_lazy_and_cli_exports_valid_json(self):
        from orient.template_engine import requested_contexts

        self.assertEqual(requested_contexts('"base16 palettes" {{primary}}'), set())
        self.assertEqual(requested_contexts(r"\{{palettes.primary._99.hex}}"), set())
        self.assertEqual(
            requested_contexts("<* for n,v in palettes *>{{v._99.hex}}<* endfor *>"),
            {"palettes"},
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            palette = root / "palette.json"
            palette.write_text(json.dumps(default_palette("light")))
            template = root / "studio.json"
            template.write_text(
                '{"base":[<* for n,v in base16 *>"{{v.default.hex}}"<* if {{loop.last}} *><* else *>,<* endif *><* endfor *>]}'
            )
            before = palette.read_bytes()
            result = subprocess.run(
                [sys.executable, "-m", "orient", "render", str(palette), str(template)],
                env={
                    **os.environ,
                    "PYTHONPATH": "",
                    "HOME": str(root),
                    "XDG_CACHE_HOME": str(root / "cache"),
                },
                capture_output=True,
                text=True,
                check=True,
            )
            self.assertEqual(len(json.loads(result.stdout)["base"]), 16)
            self.assertEqual(before, palette.read_bytes())
            self.assertFalse((root / ".local/state/nacre").exists())

    def test_all_builtin_templates_and_imported_matugen_css_export(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            template = root / "templates"
            template.mkdir()
            (template / "theme.css.in").write_text(
                ":root{--fg:{{ colors.on_surface.default.hex }};--bg:{{colors.surface.default.hex}};}"
            )
            palette = default_palette("light")
            files = export(palette, root / "export", template)
            self.assertEqual(len(files), 1)
            self.assertNotIn("{{", Path(files[0]).read_text())
            with self.assertRaises(ValueError):
                export(palette, root / "export", template)
            export(palette, root / "builtins")
            self.assertEqual(len(list((root / "builtins").rglob("*.*"))), 13)

    def test_opposite_mode_can_be_generated_from_unchanged_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            image = root / "scene.png"
            Image.new("RGB", (64, 40), "#ad4867").save(image)
            palette = from_image(image, cache_dir=root / "cache")
            text = "{{ colors.primary.dark.hex }} {{ colors.primary.light.hex }}"
            result = render(text, palette)
            expected = from_image(image, "light", cache_dir=root / "cache")
            self.assertEqual(
                result,
                "#" + palette["colours"]["primary"] + " #" + expected["colours"]["primary"],
            )
            Image.new("RGB", (64, 40), "#338899").save(image)
            with self.assertRaisesRegex(ValueError, "changed"):
                render(text, palette)

    def test_standalone_render_accepts_plain_file_and_companion_without_publication(
        self,
    ):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            p = root / "palette.json"
            p.write_text(json.dumps(default_palette("dark")))
            other = root / "light.json"
            other.write_text(json.dumps(default_palette("light")))
            template = root / "theme.css"
            template.write_text("p{color:{{colors.primary.light.hex}}}")
            env = {
                **os.environ,
                "HOME": str(root),
                "XDG_CACHE_HOME": str(root / "cache"),
                "PYTHONPATH": "",
            }
            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "orient",
                    "render",
                    str(p),
                    str(template),
                    "--companion",
                    str(other),
                ],
                env=env,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn("{{", result.stdout)
            self.assertTrue(result.stdout.startswith("p{color:#"))
            self.assertFalse((root / ".config/nacre").exists())
            self.assertFalse((root / ".local/state/nacre").exists())
