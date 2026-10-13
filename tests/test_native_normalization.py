"""Pixel-level contracts for the native Pillow-compatible thumbnail arithmetic."""

import json
from pathlib import Path
import random
import os
import signal
import subprocess
import tempfile
import time
import unittest

from PIL import Image
from orient.extract import frames, normalize

ROOT = Path(__file__).parents[1]
EXAMPLE = ROOT / "target/release/examples/normalized_frames"
BINARY = ROOT / "target/release/orient"


@unittest.skipUnless(EXAMPLE.is_file(), "Build the native normalization diagnostic")
class NativeNormalizationTests(unittest.TestCase):
    def test_lossless_codecs_and_animated_mode_promotion(self):
        rng = random.Random(99)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for codec, mode in (("BMP", "P"), ("BMP", "L"), ("TIFF", "RGB"), ("WEBP", "RGB"), ("WEBP", "RGBA")):
                bands = Image.getmodebands(mode)
                image = Image.frombytes(mode, (513, 325), rng.randbytes(513 * 325 * bands))
                if mode == "P":
                    image.putpalette([i % 256 for i in range(768)])
                path = root / f"{mode}.{codec.lower()}"
                image.save(path, lossless=True)
                subprocess.run([str(EXAMPLE), str(path), str(root / "decoded")], check=True, capture_output=True)
                actual = Image.open(root / "decoded/0.png").convert("RGBA")
                with self.subTest(codec=codec, mode=mode):
                    self.assertEqual(actual.tobytes(), normalize(path).tobytes())
            for alpha in (False, True):
                items = []
                for _ in range(4):
                    image = Image.frombytes("P", (1024, 768), rng.randbytes(1024 * 768))
                    image.putpalette([i % 256 for i in range(768)])
                    items.append(image)
                path = root / "animated.gif"
                options = {"transparency": 0} if alpha else {}
                items[0].save(path, save_all=True, append_images=items[1:], duration=100, **options)
                subprocess.run([str(EXAMPLE), str(path), str(root / "decoded")], check=True, capture_output=True)
                for index, expected in enumerate(frames(path)):
                    actual = Image.open(root / f"decoded/{index}.png").convert("RGBA")
                    with self.subTest(alpha=alpha, frame=index):
                        self.assertEqual(actual.tobytes(), expected.tobytes())

    def test_rpc_cancellation_kills_decoder_descendants(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            probe = root / "ffprobe"
            probe.write_text('#!/bin/sh\nsleep 30 &\necho $! > "$ORIENT_TEST_CHILD"\nwait\n')
            probe.chmod(0o700)
            path = root / "test.mp4"
            path.touch()
            child = root / "child"
            process = subprocess.Popen(
                [str(BINARY), "--rpc"],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                env={
                    **os.environ,
                    "PATH": str(root) + os.pathsep + os.environ["PATH"],
                    "ORIENT_TEST_CHILD": str(child),
                },
                start_new_session=True,
            )
            try:
                process.stdin.write(json.dumps({"operation": "analyze", "payload": {"path": str(path)}}).encode())
                process.stdin.close()
                deadline = time.monotonic() + 5
                while not child.exists() and time.monotonic() < deadline:
                    time.sleep(0.01)
                self.assertTrue(child.exists(), "decoder child did not start")
                os.killpg(process.pid, signal.SIGINT)
                process.wait(timeout=3)
                reply = json.loads(process.stdout.read())
                self.assertIn("cancelled", reply["error"]["message"])
                status = Path("/proc") / child.read_text().strip() / "status"
                if status.exists():
                    self.assertTrue(
                        any(line.startswith("State:") and "Z" in line for line in status.read_text().splitlines())
                    )
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()
                process.stdout.close()
                process.stderr.close()

    def test_pixels_across_modes_sizes_and_integer_reduction(self):
        rng = random.Random(1729)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for mode in ("RGB", "RGBA", "L", "LA", "P"):
                for index, size in enumerate(((513, 325), (1024, 768), (128, 129), (1, 513), (31, 4000))):
                    image = Image.new(mode, size)
                    bands = len(image.getbands())
                    pixels = [
                        tuple(rng.randrange(256) for _ in range(bands)) if bands > 1 else rng.randrange(256)
                        for _ in range(size[0] * size[1])
                    ]
                    image.putdata(pixels)
                    if mode == "P":
                        image.putpalette([i % 256 for i in range(768)])
                    path = root / f"{mode}-{index}.png"
                    image.save(path)
                    subprocess.run([str(EXAMPLE), str(path), str(root / "decoded")], check=True, capture_output=True)
                    expected = normalize(path)
                    actual = Image.open(root / "decoded/0.png").convert("RGBA")
                    with self.subTest(mode=mode, size=size):
                        self.assertEqual(actual.size, expected.size)
                        self.assertEqual(actual.tobytes(), expected.tobytes())

    def test_native_json_interface_and_engine_identity(self):
        from orient.engine import default_palette

        def query(operation, payload):
            result = subprocess.run(
                [str(BINARY), "--rpc"],
                input=json.dumps({"operation": operation, "payload": payload}),
                capture_output=True,
                text=True,
                timeout=30,
            )
            return result, json.loads(result.stdout)

        process, info = query("info", {})
        self.assertEqual(process.returncode, 0)
        self.assertEqual(info["result"]["backend"], "rust")
        self.assertEqual(info["engine"], info["result"]["engine"])
        self.assertEqual(info["engine"], subprocess.check_output([str(BINARY), "--version"], text=True).strip())
        process, palette = query("default_palette", {"options": {"mode": "dark"}})
        self.assertEqual(process.returncode, 0)
        self.assertEqual(palette["result"]["colours"], default_palette()["colours"])
        process, failed = query("clean", {"value": "bad"})
        self.assertEqual(process.returncode, 1)
        self.assertEqual(failed["error"]["kind"], "value")
