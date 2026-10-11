"""Independent engine/public-format invariants and reproducible gallery."""

import importlib.util
import json
import os
from pathlib import Path
import sys
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).parents[1]
sys.path.insert(0, str(ROOT / "src"))
from PIL import Image
from orient.engine import from_image
from orient.colour import color, lch, hue_distance
from orient.accessibility import simulate, difference
from orient.export import export, render
from orient.extract import sample_color, analyze
from orient.palette import PERSONALITIES


class Orient3Tests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)
        env = patch.dict(
            os.environ,
            {
                "HOME": str(self.home),
                "XDG_CACHE_HOME": str(self.home / ".cache"),
                "XDG_STATE_HOME": str(self.home / ".state"),
                "XDG_CONFIG_HOME": str(self.home / ".config"),
            },
        )
        env.start()
        self.addCleanup(env.stop)

    def scene(self, base, accent, name="scene.png"):
        p = self.home / name
        image = Image.new("RGB", (128, 80), "#" + base)
        image.paste("#" + accent, (48, 26, 80, 54))
        image.save(p)
        return p

    def test_natural_body_switch_preserves_accents_and_source_compatibility(self):
        p = self.scene("28160f", "2870db")
        for mode in ("dark", "light"):
            natural = from_image(p, mode, accent="2870db")
            source = from_image(p, mode, accent="2870db", background_from_wallpaper=True)
            legacy = from_image(p, mode, accent="2870db", personality="source")
            self.assertEqual(source["colours"], legacy["colours"])
            self.assertEqual(source["input"], legacy["input"])
            self.assertEqual(source["colours"]["overtone"], natural["colours"]["overtone"])
            self.assertEqual(natural["input"]["personality"], "natural")
            self.assertLess(
                hue_distance(lch(natural["colours"]["surface"])[2], lch("2870db")[2]),
                10,
            )
            self.assertNotEqual(natural["colours"]["surface"], source["colours"]["surface"])
            self.assertEqual(natural["provenance"]["surface"]["original"], "2870db")
            self.assertTrue(natural["accessibility"]["textPasses"])
            self.assertTrue(source["accessibility"]["textPasses"])

    def test_canyon_shade_populations_beat_one_sky_cluster(self):
        path = self.home / "canyon.png"
        image = Image.new("RGB", (128, 80), "#202020")
        for i, light in enumerate((0.30, 0.41, 0.52, 0.63)):
            image.paste("#" + color(light, 0.10, 35), (i * 8, 0, (i + 1) * 8, 80))
        image.paste("#" + color(0.63, 0.10, 245), (100, 0, 110, 80))
        image.save(path)
        result = analyze(path)
        self.assertLess(hue_distance(lch(result["seed"])[2], 35), 8)
        main = result["candidates"][0]
        self.assertGreaterEqual(len(main["members"]), 4)
        self.assertAlmostEqual(main["coverage"], 0.25, delta=0.01)
        self.assertAlmostEqual(
            sum(c["coverage"] for c in main["regions"]["cells"]),
            main["coverage"],
            delta=0.0001,
        )
        pop = from_image(path, personality="pop")
        self.assertLess(hue_distance(lch(pop["source"]["selected"])[2], 245), 8)
        # Explicit sky choice stays honored even though the rock is recommended.
        picked = from_image(path, accent=color(0.63, 0.10, 245))
        self.assertLess(hue_distance(lch(picked["colours"]["overtone"])[2], 245), 3)

    def test_natural_supports_use_different_real_families(self):
        path = self.home / "jungle.png"
        image = Image.new("RGB", (128, 80), "#87373c")
        for i, light in enumerate((0.3, 0.4, 0.5, 0.6)):
            image.paste("#" + color(light, 0.10, 19), (i * 24, 0, (i + 1) * 24, 80))
        image.paste("#b46549", (96, 0, 112, 80))
        image.paste("#dbcf79", (112, 0, 128, 80))
        image.save(path)
        for mode in ("dark", "light"):
            result = from_image(path, mode, personality="pigment")
            hues = [lch(s["sourceColor"])[2] for s in result["roleSources"].values()]
            self.assertTrue(any(hue_distance(h, 19) < 8 for h in hues))
            self.assertTrue(any(hue_distance(h, lch("b46549")[2]) < 8 for h in hues))
            self.assertTrue(any(hue_distance(h, lch("dbcf79")[2]) < 8 for h in hues))
            self.assertTrue(all(s["type"] == "observed" for s in result["roleSources"].values()))
            self.assertTrue(result["accessibility"]["textPasses"])

    def test_light_accents_lift_shadow_pigments_without_changing_sources_or_body(self):
        p = self.home / "shadow-pigments.png"
        image = Image.new("RGB", (128, 80), "#202020")
        for i, hue in enumerate((20, 85, 250)):
            image.paste("#" + color(0.24, 0.06, hue), (i * 40, 0, (i + 1) * 40, 80))
        image.save(p)
        for personality in PERSONALITIES:
            for sampled_body in (False, True):
                data = from_image(
                    p,
                    "light",
                    personality=personality,
                    background_from_wallpaper=sampled_body,
                )
                for role in ("primary", "secondary", "tertiary"):
                    light, chroma, hue = lch(data["colours"][role])
                    self.assertGreaterEqual(light, 0.43, (personality, role, light))
                    self.assertLessEqual(light, 0.60)
                    self.assertLess(
                        hue_distance(hue, lch(data["roleSources"][role]["sourceColor"])[2]),
                        5,
                    )
                self.assertTrue(data["accessibility"]["textPasses"])
                self.assertGreater(lch(data["colours"]["surface"])[0], 0.94)

    def test_pale_warm_light_accents_keep_color_and_contrast(self):
        from orient.palette import generate

        for seed in ("dbc5b9", "dbcf79", "eebd9a"):
            colors = generate(seed, "light")
            source_light, source_chroma, source_hue = lch(seed)
            light, chroma, hue = lch(colors["primary"])
            self.assertGreater(chroma, source_chroma * 0.85)  # sRGB gamut can cap saturated yellow.
            self.assertLess(hue_distance(hue, source_hue), 5)
            self.assertGreater(light, 0.43)
            self.assertEqual(colors["overtone"], seed)
            self.assertTrue(__import__("orient.accessibility", fromlist=["audit"]).audit(colors)["textPasses"])

    def test_light_neutral_accents_are_distinct_grays_not_black(self):
        for seed in ("070707", "808080", "eeeeee"):
            p = self.scene(seed, seed)
            data = from_image(p, "light")
            colors = data["colours"]
            values = [lch(colors[role]) for role in ("primary", "secondary", "tertiary")]
            self.assertTrue(all(0.34 <= light <= 0.55 and chroma < 0.005 for light, chroma, hue in values))
            self.assertEqual(len({colors[r] for r in ("primary", "secondary", "tertiary")}), 3)
            for vision in ("normal", "protanopia", "deuteranopia", "tritanopia"):
                self.assertGreater(difference(colors["primary"], colors["secondary"], vision), 0.045)
                self.assertGreater(difference(colors["secondary"], colors["tertiary"], vision), 0.045)
            self.assertTrue(data["accessibility"]["textPasses"])

    def test_pop_uses_a_contrasting_minority_not_a_bright_main_shade(self):
        path = self.home / "pop-scene.png"
        image = Image.new("RGB", (128, 80), "#87373c")
        image.paste("#ed3041", (30, 24, 62, 56))
        image.paste("#dbcf79", (90, 18, 110, 38))
        image.save(path)
        for mode in ("dark", "light"):
            natural = from_image(path, mode)
            pop = from_image(path, mode, personality="pop")
            self.assertLess(hue_distance(lch(pop["source"]["selected"])[2], lch("dbcf79")[2]), 8)
            self.assertGreater(
                hue_distance(
                    lch(pop["source"]["selected"])[2],
                    lch(natural["source"]["selected"])[2],
                ),
                48,
            )
            self.assertEqual(pop["roleSources"]["primary"]["type"], "observed")
            self.assertLess(pop["roleSources"]["primary"]["coverage"], 0.15)
            self.assertTrue(pop["accessibility"]["textPasses"])
        # A pinned main accent is never replaced by the Pop heuristic.
        self.assertEqual(
            from_image(path, personality="pop", accent="4473af")["source"]["selected"],
            "4473af",
        )
        self.assertEqual(
            from_image(path, personality="pop", overrides={"primary": "4473af"})["source"]["selected"],
            "4473af",
        )

    def test_pop_prefers_small_sign_over_broad_contrasting_desert(self):
        p = self.home / "colorado-sign.png"
        image = Image.new("RGB", (128, 80), "#5b9dc1")
        image.paste("#cca07d", (0, 60, 128, 80))
        image.paste("#5f302b", (56, 35, 72, 55))
        image.save(p)
        palette = from_image(p, personality="pop")
        self.assertLess(hue_distance(lch(palette["source"]["selected"])[2], lch("5f302b")[2]), 8)
        self.assertLess(palette["roleSources"]["primary"]["coverage"], 0.05)

    def test_pop_fallback_does_not_invent_detail_in_single_family_or_gray_images(self):
        for base, detail in (("87373c", "ed3041"), ("303030", "909090")):
            p = self.scene(base, detail)
            natural = from_image(p, personality="pigment")
            pop = from_image(p, personality="pop")
            self.assertEqual(pop["source"]["selected"], natural["source"]["selected"])
        with self.assertRaises(ValueError):
            from_image(p, background_from_wallpaper="yes")

    def test_source_body_comes_from_shadow_not_accent(self):
        p = self.scene("28160f", "2870db")
        data = from_image(p, background_from_wallpaper=True)
        self.assertLess(hue_distance(lch(data["colours"]["surface"])[2], lch("28160f")[2]), 15)
        self.assertLess(
            hue_distance(
                lch(from_image(p, accent="df2020", background_from_wallpaper=True)["colours"]["surface"])[2],
                lch("28160f")[2],
            ),
            15,
        )
        q = self.scene("07152b", "ed566e", "navy.png")
        self.assertLess(
            hue_distance(
                lch(from_image(q, background_from_wallpaper=True)["colours"]["surface"])[2],
                lch("07152b")[2],
            ),
            15,
        )

    def test_light_body_from_bright_population_and_every_role_has_provenance(self):
        p = self.scene("eee3ce", "276ba0")
        data = from_image(p, "light", background_from_wallpaper=True)
        self.assertGreater(lch(data["colours"]["surface"])[0], 0.94)
        self.assertLess(hue_distance(lch(data["colours"]["surface"])[2], lch("eee3ce")[2]), 20)
        self.assertEqual(set(data["colours"]), set(data["provenance"]))
        self.assertTrue(data["accessibility"]["textPasses"])
        self.assertEqual(data["formatVersion"], 1)

    def test_personalities_and_pearl_are_readable_and_distinct(self):
        p = self.scene("202c4a", "ec75ba")
        palettes = {name: from_image(p, personality=name) for name in PERSONALITIES}
        self.assertEqual(len({p["colours"]["primary"] for p in palettes.values()}) >= 4, True)
        for value in palettes.values():
            self.assertTrue(value["accessibility"]["textPasses"])
        q = self.scene("493017", "40996a", "other.png")
        self.assertEqual(palettes["pearl"]["colours"], from_image(q, personality="pearl")["colours"])
        self.assertNotEqual(
            from_image(p, personality="tide", hour=8)["colours"]["surface"],
            from_image(p, personality="tide", hour=20)["colours"]["surface"],
        )

    def test_salience_coverage_locations_and_small_crop_stability(self):
        p = self.scene("253452", "e19858")
        data = from_image(p, personality="pigment")
        candidate = next(
            c for c in data["source"]["candidates"] if hue_distance(lch(c["hex"])[2], lch("e19858")[2]) < 10
        )
        self.assertGreater(candidate["salience"], 1)
        self.assertAlmostEqual(candidate["coverage"], 32 * 28 / (128 * 80), delta=0.03)
        self.assertAlmostEqual(candidate["location"]["x"], 0.5, delta=0.03)
        self.assertTrue(candidate["regions"]["cells"])
        image = Image.open(p)
        q = self.home / "crop.png"
        image.crop((4, 2, 124, 78)).save(q)
        self.assertLess(
            difference(data["source"]["selected"], from_image(q)["source"]["selected"]),
            0.05,
        )

    def test_animated_frames_and_semantic_terminal_hues(self):
        fixture = ROOT / "tests/orient-gallery/images/moving-lantern.gif"
        data = from_image(fixture)
        self.assertEqual(data["source"]["sample"]["frames"], 4)
        for mode in ("dark", "light"):
            c = from_image(fixture, mode)["colours"]
            self.assertLess(hue_distance(lch(c["red"])[2], lch("df303e")[2]), 8)
            self.assertLess(hue_distance(lch(c["green"])[2], lch("289563")[2]), 8)

    @unittest.skipUnless(shutil.which("ffmpeg") and shutil.which("ffprobe"), "Video decoder unavailable")
    def test_short_video_samples_stay_inside_last_decodable_frame(self):
        target = self.home / "short.mp4"
        subprocess.run(
            [
                "ffmpeg",
                "-v",
                "error",
                "-i",
                str(ROOT / "tests/orient-gallery/images/moving-lantern.gif"),
                "-pix_fmt",
                "yuv420p",
                "-y",
                str(target),
            ],
            check=True,
            capture_output=True,
            timeout=10,
        )
        data = from_image(target)
        self.assertEqual(data["source"]["sample"]["frames"], 4)
        self.assertTrue(data["accessibility"]["textPasses"])

    def test_export_json_toml_templates_no_overwrite_and_no_publication(self):
        import tomllib

        p = self.scene("192b42", "ac6abd")
        data = from_image(p)
        target = self.home / "export"
        files = export(data, target)
        self.assertGreaterEqual(len(files), 12)
        json.loads((target / "vscode-theme.json").read_text())
        json.loads((target / "firefox/manifest.json").read_text())
        tomllib.loads((target / "alacritty.toml").read_text())
        with self.assertRaises(ValueError):
            export(data, target)
        with self.assertRaises(ValueError):
            render("{{not_a_role}}", data)
        self.assertFalse((self.home / ".config/nacre").exists())
        self.assertFalse((self.home / ".state/nacre").exists())

    def test_installed_runtime_symlink_can_export_builtins(self):
        import orient.export as exporter

        alias = self.home / "active-runtime"
        alias.symlink_to(Path(exporter.__file__).parent, target_is_directory=True)
        data = from_image(self.scene("203450", "aa679b"))
        with patch.object(exporter, "__file__", str(alias / "export.py")):
            paths = exporter.export(data, self.home / "export-symlink")
        self.assertEqual(len(paths), 13)

    def test_vision_simulation_neutrals_and_safe_adjustments(self):
        for vision in ("protanopia", "deuteranopia", "tritanopia"):
            value = simulate("808080", vision)
            self.assertLess(
                max(int(value[i : i + 2], 16) for i in (0, 2, 4)) - min(int(value[i : i + 2], 16) for i in (0, 2, 4)),
                2,
            )
        p = self.scene("253452", "e19858")
        a = from_image(p, overrides={"secondary": "109950"}, brightness=0.05, saturation=1.2)
        self.assertTrue(a["accessibility"]["textPasses"])
        with self.assertRaises(ValueError):
            from_image(p, saturation=2)
        with self.assertRaises(ValueError):
            sample_color(p, 1.2, 0.5)

    def test_gallery_matches_reviewed_palette_snapshots(self):
        path = ROOT / "tests/orient-gallery/snapshot.py"
        spec = importlib.util.spec_from_file_location("gallery_snapshot", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        self.assertEqual(module.snapshot(), json.loads(path.with_name("palettes.json").read_text()))
