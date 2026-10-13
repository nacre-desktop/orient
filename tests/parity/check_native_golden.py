"""Compare native decoding and the full image option matrix to frozen Python output."""

import gzip
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from make_golden import scrub, synthetics, unpack

ROOT = Path(__file__).parents[2]
BINARY = ROOT / "target/release/orient"
GOLDEN = Path(__file__).with_name("golden")


@unittest.skipUnless(BINARY.is_file(), "Build the standalone binary")
class NativeGoldenParity(unittest.TestCase):
    def test_original_image_outputs(self):
        reference = unpack(json.loads(gzip.decompress((GOLDEN / "outputs.json.gz").read_bytes())))

        def query(operation, **payload):
            process = subprocess.run(
                [str(BINARY), "--rpc"],
                input=json.dumps({"operation": operation, "payload": payload}),
                text=True,
                capture_output=True,
                timeout=30,
            )
            reply = json.loads(process.stdout)
            if "error" in reply:
                failure = reply["error"]
                kind = {"key": "KeyError", "value": "ValueError", "io": "OSError"}[failure["kind"]]
                return {"error": kind, "message": failure["message"]}
            self.assertEqual(process.returncode, 0, process.stderr[-300:])
            return {"value": scrub(reply["result"])}

        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            inputs = {path.stem: path for path in synthetics(root)}
            gallery = ROOT / "tests/orient-gallery/images"
            inputs.update({path.stem: path for path in gallery.iterdir()})
            for case in reference["images"]:
                path = str(inputs[case["name"]])
                self.assertEqual(query("analyze", path=path), case["analysis"], case["name"])
                for sample in case["samples"]:
                    x, y = sample["point"]
                    self.assertEqual(query("sample_color", path=path, x=x, y=y), sample["expected"], case["name"])
                for record in case["palettes"]:
                    options = {**record["options"], "cache_dir": str(root / "cache")}
                    actual = query("from_image", path=path, options=options)
                    # Pillow versions may encode identical generated pixels differently.
                    if "value" in actual and not inputs[case["name"]].is_relative_to(gallery):
                        actual["value"]["source"]["digest"] = record["expected"]["value"]["source"]["digest"]
                    self.assertEqual(actual, record["expected"], (case["name"], record["options"]))


if __name__ == "__main__":
    unittest.main()
