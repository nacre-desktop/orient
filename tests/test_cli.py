"""Installed/standalone command behavior has no desktop publication side effects."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from PIL import Image
from orient import __version__, FORMAT_VERSION


class PortableCliTests(unittest.TestCase):
    def test_palette_and_export_need_no_nacre_or_display(self):
        with tempfile.TemporaryDirectory() as folder:
            home = Path(folder)
            image = home / "image.png"
            Image.new("RGB", (40, 30), "#47a99a").save(image)
            env = {
                **os.environ,
                "HOME": str(home),
                "XDG_CONFIG_HOME": str(home / "config"),
                "XDG_STATE_HOME": str(home / "state"),
                "XDG_CACHE_HOME": str(home / "cache"),
                "PYTHONPATH": "",
            }
            for key in ("WAYLAND_DISPLAY", "DISPLAY", "HYPRLAND_INSTANCE_SIGNATURE"):
                env.pop(key, None)
            result = subprocess.run(
                [sys.executable, "-m", "orient", "palette", str(image)],
                env=env,
                capture_output=True,
                text=True,
                timeout=15,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            data = json.loads(result.stdout)
            self.assertEqual(data["formatVersion"], FORMAT_VERSION)
            self.assertEqual(data["mode"], "dark")
            self.assertFalse((home / "config").exists())
            self.assertFalse((home / "state").exists())
            self.assertTrue(data["engine"].startswith("orient-" + __version__))

    def test_public_version_and_metadata_agree(self):
        import tomllib

        root = Path(__file__).parents[1]
        metadata = tomllib.loads((root / "pyproject.toml").read_text())
        self.assertEqual(metadata["project"]["version"], __version__)
        self.assertEqual((root / "VERSION").read_text().strip(), __version__.replace(".dev", "-dev."))
