"""Independent engine invariants, image behavior and CLI publication boundaries."""

import colorsys
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

try:
    from PIL import Image, ImageCms
except ImportError:
    Image = None

SOURCE = Path(__file__).resolve().parents[1] / "src"
sys.path.insert(0, str(SOURCE))
if Image:
    from orient.colour import color, contrast, hue_distance, lch
    from orient.engine import from_image
    from orient.extract import analyze
    from orient.palette import generate, supporting_sources


@unittest.skipUnless(Image, "Pillow unavailable")
class OrientTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)
        self.environment = patch.dict(
            os.environ,
            {
                "HOME": str(self.home),
                "XDG_CONFIG_HOME": str(self.home / ".config"),
                "XDG_STATE_HOME": str(self.home / ".local/state"),
                "XDG_CACHE_HOME": str(self.home / ".cache"),
            },
        )
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def image(self, seed, name="image.png"):
        path = self.home / name
        Image.new("RGB", (128, 80), "#" + seed).save(path)
        return path

    def test_color_round_trip_and_fixed_hue_gamut(self):
        for seed in (
            "d01818",
            "ff3030",
            "ff791f",
            "4275ff",
            "11d96c",
            "ffffff",
            "000000",
        ):
            result = color(*lch(seed))
            self.assertTrue(all(abs(int(seed[i : i + 2], 16) - int(result[i : i + 2], 16)) <= 1 for i in (0, 2, 4)))
        mapped = color(0.6, 0.7, 30)
        self.assertLess(hue_distance(lch(mapped)[2], 30), 1)

    def test_saturated_red_stays_red_instead_of_pastel(self):
        source = "d01818"
        for mode in ("light", "dark"):
            data = from_image(self.image(source), mode)
            c = data["colours"]
            self.assertEqual(c["overtone"], source)
            self.assertLess(hue_distance(lch(c["primary"])[2], lch(source)[2]), 2)
            saturation = colorsys.rgb_to_hsv(*(int(c["primary"][i : i + 2], 16) / 255 for i in (0, 2, 4)))[1]
            self.assertGreater(saturation, 0.65)
            self.assertGreaterEqual(contrast(c["primary"], c["surfaceContainerHighest"]), 4.5)

    def test_red_orange_magenta_blue_green_are_distinct(self):
        for seed in ("d01818", "fa6a10", "cc22aa", "255fdf", "109f50"):
            analysis = analyze(self.image(seed))
            self.assertLess(hue_distance(lch(seed)[2], lch(analysis["seed"])[2]), 2)

    def test_coverage_beats_tiny_neon_detail(self):
        path = self.image("bb2020")
        image = Image.open(path)
        image.paste("#00ff00", (0, 0, 8, 8))
        image.save(path)
        self.assertLess(hue_distance(lch(analyze(path)["seed"])[2], lch("bb2020")[2]), 3)

    def test_neutral_background_does_not_hide_meaningful_subject(self):
        path = self.image("181818")
        image = Image.open(path)
        image.paste("#b02020", (40, 20, 88, 60))
        image.save(path)
        analysis = analyze(path)
        self.assertFalse(analysis["neutral"])
        self.assertLess(hue_distance(lch(analysis["seed"])[2], lch("b02020")[2]), 3)

    def test_candidate_override_keeps_identity_between_modes(self):
        path = self.image("bb2020")
        for mode in ("light", "dark"):
            data = from_image(path, mode, accent="2255aa")
            self.assertEqual(data["source"]["selected"], "2255aa")
            self.assertEqual(data["colours"]["overtone"], "2255aa")

    def test_transparent_rgb_does_not_invent_color(self):
        path = self.home / "alpha.png"
        image = Image.new("RGBA", (128, 80), (0, 255, 0, 0))
        image.paste((180, 20, 20, 255), (40, 20, 88, 60))
        image.save(path)
        self.assertLess(hue_distance(lch(analyze(path)["seed"])[2], lch("b41414")[2]), 3)
        Image.new("RGBA", (64, 64), (255, 0, 0, 0)).save(path)
        with self.assertRaises(ValueError):
            analyze(path)

    def test_grayscale_has_no_fabricated_candidate_hues(self):
        for seed in ("eeeeee", "101010", "888888"):
            data = analyze(self.image(seed))
            self.assertTrue(data["neutral"])
            self.assertEqual(len(data["candidates"]), 1)
            self.assertEqual(data["candidates"][0]["chroma"], 0)

    def test_profile_and_orientation_normalization(self):
        path = self.home / "profile.png"
        profile = ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB")).tobytes()
        image = Image.new("RGB", (80, 120), "#d01818")
        exif = image.getexif()
        exif[274] = 6
        image.save(path, icc_profile=profile, exif=exif)
        self.assertLess(hue_distance(lch(analyze(path)["seed"])[2], lch("d01818")[2]), 3)
        image.save(path, icc_profile=b"bad profile")
        with self.assertRaises(ValueError):
            analyze(path)

    def test_unsafe_or_invalid_inputs_are_errors(self):
        path = self.home / "bad.png"
        path.write_bytes(b"not an image")
        with self.assertRaises(OSError):
            analyze(path)
        image = self.image("ff3030")
        with patch("orient.extract.MAX_PIXELS", 10):
            with self.assertRaises(ValueError):
                analyze(image)

    def test_cache_determinism_and_invalidation(self):
        path = self.image("d01818")
        first = from_image(path)
        with patch(
            "orient.engine.analyze",
            side_effect=AssertionError("warm cache must not extract"),
        ):
            self.assertEqual(from_image(path), first)
        self.image("2255aa")
        second = from_image(path)
        self.assertNotEqual(first["source"]["digest"], second["source"]["digest"])
        self.assertNotEqual(first["source"]["selected"], second["source"]["selected"])
        self.assertNotEqual(from_image(path, "light")["colours"], second["colours"])

    def test_corrupt_cache_recomputed(self):
        path = self.image("d01818")
        expected = from_image(path)
        cache = next((self.home / ".cache/orient/palettes").glob("*.json"))
        for broken in ([], None, {"engine": "orient-2.0.0", "colours": None}):
            cache.write_text(json.dumps(broken))
            self.assertEqual(from_image(path), expected)
        broken = dict(expected, source=[])
        cache.write_text(json.dumps(broken))
        self.assertEqual(from_image(path), expected)

    def test_all_required_roles_and_contrast_across_hues(self):
        roles = json.loads(Path(__file__).with_name("orient-roles.json").read_text())
        for hue in range(0, 360, 20):
            seed = color(0.6, 0.24, hue)
            for mode in ("light", "dark"):
                c = generate(seed, mode)
                self.assertTrue(set(roles) <= c.keys())
                for fg in (
                    "primary",
                    "secondary",
                    "tertiary",
                    "onSurface",
                    "onSurfaceVariant",
                ):
                    for bg in (
                        "surface",
                        "surfaceContainerHighest",
                        "frame",
                        "surfaceBright" if mode == "dark" else "surfaceDim",
                    ):
                        self.assertGreaterEqual(contrast(c[fg], c[bg]), 4.5)
                for role in ("Primary", "Secondary", "Tertiary", "Error", "Success"):
                    self.assertGreaterEqual(contrast(c["on" + role], c[role.lower()]), 4.5)
                    self.assertGreaterEqual(
                        contrast(c["on" + role + "Container"], c[role.lower() + "Container"]),
                        4.5,
                    )
                self.assertGreaterEqual(contrast(c["outline"], c["surfaceContainerHighest"]), 3)

    def test_muted_scene_keeps_warm_blue_and_lilac_families(self):
        path = self.home / "muted-scene.png"
        image = Image.new("RGB", (128, 80), "#242625")
        seeds = [color(0.40, 0.025, h) for h in (65, 130, 270, 320)]
        for index, seed in enumerate(seeds):
            image.paste("#" + seed, (index * 32, 0, (index + 1) * 32, 80))
        image.save(path)
        data = from_image(path)
        self.assertFalse(data["source"]["neutral"])
        options = data["source"]["options"]
        self.assertGreaterEqual(len(options), 4)
        self.assertEqual(len({o["accent"] for o in options}), len(options))
        accents = [lch(data["colours"][role])[2] for role in ("primary", "secondary", "tertiary")]
        self.assertGreater(
            min(hue_distance(accents[a], accents[b]) for a, b in ((0, 1), (0, 2), (1, 2))),
            35,
        )
        for option in options:
            self.assertEqual(len(option["swatches"]), 3)
            self.assertGreaterEqual(contrast(data["colours"]["onSurface"], option["surface"]), 4.5)

    def test_balanced_neon_is_softer_while_vivid_and_source_remain_vivid(self):
        seed = "d82dd1"
        balanced = generate(seed)
        vivid = generate(seed, variant="vibrant")
        self.assertEqual(balanced["overtone"], seed)
        self.assertLess(lch(balanced["primary"])[1], lch(vivid["primary"])[1] - 0.025)
        self.assertLessEqual(lch(balanced["primary"])[1], 0.195)
        self.assertGreaterEqual(contrast(balanced["onPrimary"], balanced["primary"]), 4.5)

    def test_frame_tint_varies_with_image_and_stays_readable(self):
        warm, cool = generate("c04020"), generate("3568aa")
        self.assertNotEqual(warm["frame"], cool["frame"])
        for data in (warm, cool):
            self.assertGreater(lch(data["frame"])[0], lch(data["surface"])[0])
            self.assertGreater(lch(data["frame"])[1], lch(data["surface"])[1])
            self.assertGreaterEqual(contrast(data["onSurface"], data["frame"]), 4.5)

    def test_harmony_skips_tiny_distant_patch_but_keeps_the_picker_choice(self):
        path = self.image("bb2020")
        image = Image.open(path)
        image.paste("#fa8020", (0, 0, 24, 80))
        image.paste("#2255dd", (50, 20, 58, 28))
        image.save(path)
        natural = from_image(path)
        harmonic = from_image(path, harmony=True)
        self.assertFalse(natural["input"]["harmony"])
        self.assertTrue(harmonic["input"]["harmony"])
        self.assertEqual(
            natural["source"]["options"][0]["accent"],
            harmonic["source"]["options"][0]["accent"],
        )
        for role in ("secondary", "tertiary"):
            self.assertLess(hue_distance(lch(harmonic["colours"][role])[2], lch("bb2020")[2]), 80)
        self.assertTrue(
            any(hue_distance(lch(c["accent"])[2], lch("2255dd")[2]) < 5 for c in harmonic["source"]["options"])
        )
        with patch("orient.engine.analyze", side_effect=AssertionError("cached")):
            self.assertEqual(from_image(path, harmony=True), harmonic)
            self.assertEqual(from_image(path), natural)

    def test_harmony_prefers_warm_companion_over_weak_olive_and_keeps_big_contrast(
        self,
    ):
        olive, warm, blue = color(0.40, 0.07, 120), color(0.6, 0.13, 65), "2255dd"
        candidates = [
            {"hex": olive, "coverage": 0.25, "score": 0.6},
            {"hex": warm, "coverage": 0.20, "score": 0.45},
            {"hex": blue, "coverage": 0.18, "score": 0.40},
        ]
        self.assertEqual(supporting_sources("bb2020", candidates, True), [warm, blue])
        for mode in ("dark", "light"):
            explicit = generate(olive, mode, companions=candidates, harmony=True)
            self.assertEqual(explicit["overtone"], olive)
            if mode == "dark":
                self.assertGreaterEqual(lch(explicit["primary"])[0], 0.65)
            self.assertGreaterEqual(contrast(explicit["primary"], explicit["surfaceContainerHighest"]), 4.5)
            self.assertLess(hue_distance(lch(explicit["primary"])[2], 120), 2)

    def test_containers_keep_tint_and_harmony_respects_monochrome_and_contrast(self):
        for seed in ("b62626", "2266bb", color(0.4, 0.025, 65)):
            colors = generate(seed)
            self.assertGreater(lch(colors["primaryContainer"])[1], 0.025)
            for mode in ("dark", "light"):
                c = generate(seed, mode, companions=["2255dd", "777b32"], harmony=True)
                for role in ("Primary", "Secondary", "Tertiary"):
                    self.assertGreaterEqual(
                        contrast(c["on" + role + "Container"], c[role.lower() + "Container"]),
                        4.5,
                    )
        mono = generate(
            "bb2020",
            companions=["2255dd", "228844"],
            variant="monochrome",
            harmony=True,
        )
        for role in (
            "primary",
            "secondary",
            "tertiary",
            "orient1",
            "orient2",
            "orient3",
        ):
            self.assertLess(lch(mono[role])[1], 0.001)

    def test_licensed_scene_samples_keep_roles_readable_and_sources_real(self):
        scenes = json.loads(Path(__file__).with_name("fixtures").joinpath("orient-scenes.json").read_text())
        for scene in scenes:
            a = scene["analysis"]
            candidates = a["candidates"]
            source_colors = {c["hex"] for c in candidates}
            chosen = supporting_sources(a["seed"], candidates, True)
            self.assertTrue(set(chosen) <= source_colors)
            for mode in ("dark", "light"):
                colors = generate(a["seed"], mode, companions=candidates, harmony=True)
                for role in ("Primary", "Secondary", "Tertiary"):
                    self.assertGreaterEqual(
                        contrast(colors[role.lower()], colors["surfaceContainerHighest"]),
                        4.5,
                        scene["name"],
                    )
                    self.assertGreaterEqual(
                        contrast(
                            colors["on" + role + "Container"],
                            colors[role.lower() + "Container"],
                        ),
                        4.5,
                        scene["name"],
                    )
