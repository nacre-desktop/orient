"""Standalone decoder and CLI contracts; Python still owns Pillow decode parity."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from PIL import Image, ImageCms
from orient import ENGINE_ID
from orient.engine import default_palette
from orient.export import render

BINARY = Path(os.environ.get("ORIENT_BINARY", Path(__file__).parents[1] / "target/release/orient")).resolve()


@unittest.skipUnless(BINARY.is_file(), "Build orient-cli to test the standalone binary")
class RustCliTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def run_cli(self, *args):
        return subprocess.run(
            [str(BINARY), *map(str, args)],
            env={**os.environ, "XDG_CACHE_HOME": str(self.root / "cache")},
            capture_output=True,
            text=True,
            timeout=30,
        )

    def image(self, format_name="PNG", **kwargs):
        suffix = {"JPEG": "jpg", "TIFF": "tiff"}.get(format_name, format_name.lower())
        path = self.root / ("color." + suffix)
        if format_name == "WEBP":
            kwargs.setdefault("lossless", True)
        Image.new("RGB", (24, 16), "#336699").save(path, format=format_name, **kwargs)
        return path

    def test_formats_and_samples(self):
        for format_name in ("PNG", "JPEG", "WEBP", "GIF", "BMP", "TIFF"):
            with self.subTest(format_name=format_name):
                path = self.image(format_name)
                result = self.run_cli("analyze", path)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertTrue(json.loads(result.stdout)["candidates"])
                result = self.run_cli("sample", path, "0.5", "0.5")
                self.assertEqual(result.returncode, 0, result.stderr)
                if format_name != "JPEG":
                    self.assertEqual(json.loads(result.stdout)["hex"], "336699")

    def test_exif_orientation_icc_and_transparency(self):
        image = Image.new("RGB", (2, 3), "#ff0000")
        image.paste("#0000ff", (1, 0, 2, 3))
        exif = Image.Exif()
        exif[274] = 6
        path = self.root / "oriented.png"
        image.save(path, exif=exif)
        result = self.run_cli("analyze", path)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["sample"], {"width": 3, "height": 2, "frames": 1})
        profile = ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB")).tobytes()
        path = self.image(icc_profile=profile)
        result = self.run_cli("sample", path, "0.5", "0.5")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["hex"], "336699")
        path = self.image(icc_profile=b"broken profile")
        self.assertEqual(self.run_cli("analyze", path).returncode, 1)
        path = self.root / "transparent.png"
        Image.new("RGBA", (1, 1), (1, 2, 3, 0)).save(path)
        self.assertEqual(self.run_cli("analyze", path).returncode, 1)
        self.assertEqual(self.run_cli("sample", path, 0, 0).returncode, 1)
        Image.new("I;16", (1, 1), 1234).save(path)
        self.assertEqual(self.run_cli("analyze", path).returncode, 1)

    def test_pixel_limit_before_decode(self):
        import struct
        import zlib

        def chunk(kind, data):
            return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

        header = struct.pack(">IIBBBBB", 50_001, 1000, 8, 2, 0, 0, 0)
        path = self.root / "oversized.png"
        path.write_bytes(
            b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(b"")) + chunk(b"IEND", b"")
        )
        result = self.run_cli("analyze", path)
        self.assertEqual(result.returncode, 1)
        self.assertIn("50 megapixel", result.stderr)

    def test_temporal_gif_and_video(self):
        path = self.root / "animation.gif"
        frames = [Image.new("RGB", (24, 16), color) for color in ("red", "green", "blue", "yellow")]
        frames[0].save(path, save_all=True, append_images=frames[1:], duration=100, loop=0)
        result = self.run_cli("analyze", path)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["sample"]["frames"], 4)
        import shutil

        if shutil.which("ffmpeg"):
            video = self.root / "video.mp4"
            encoded = subprocess.run(
                ["ffmpeg", "-v", "error", "-f", "lavfi", "-i", "color=c=red:s=24x16:r=10:d=1", "-y", str(video)],
                capture_output=True,
                timeout=30,
            )
            self.assertEqual(encoded.returncode, 0, encoded.stderr)
            result = self.run_cli("analyze", video)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["sample"]["frames"], 4)

    def test_flags_exit_codes_cache_render_export(self):
        self.assertEqual(self.run_cli("--version").stdout.strip(), ENGINE_ID)
        self.assertEqual(self.run_cli().returncode, 2)
        self.assertEqual(self.run_cli("-V").returncode, 2)
        self.assertEqual(self.run_cli("--vers").stdout.strip(), ENGINE_ID)
        self.assertEqual(self.run_cli("palette", self.image(), "--hour", "9999999999999999999999").returncode, 1)
        self.assertEqual(self.run_cli("palette", self.image(), "--mode", "wrong").returncode, 2)
        self.assertEqual(self.run_cli("sample", self.image(), "-0.1", "0.5").returncode, 1)
        result = self.run_cli(
            "palette",
            self.image(),
            "--personality",
            "tide",
            "--brightness",
            "-0.1",
            "--saturation",
            "1.5",
            "--hour",
            "23",
            "--primary",
            "ff3366",
            "--background-from-wallpaper",
            "--no-background-from-wallpaper",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        palette = json.loads(result.stdout)
        self.assertFalse(palette["input"]["background_from_wallpaper"])
        self.assertEqual(palette["input"]["hour"], 23)
        self.assertEqual(palette["roleSources"]["primary"]["type"], "picked")
        caches = list((self.root / "cache/orient/palettes").glob("*.json"))
        self.assertTrue(caches)
        self.assertTrue(all(path.stat().st_mode & 0o777 == 0o600 for path in caches))
        palette_path = self.root / "palette.json"
        fixed = default_palette()
        palette_path.write_text(json.dumps(fixed))
        template = self.root / "template.in"
        text = "#{{primary}} {{colors.primary.default.rgba | set_alpha:0.2}}"
        template.write_text(text)
        result = self.run_cli("render", palette_path, template)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, render(text, fixed))
        directory = self.root / "themes"
        result = self.run_cli("export", palette_path, directory)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(json.loads(result.stdout)["files"]), 13)
        self.assertEqual(self.run_cli("export", palette_path, directory).returncode, 1)
