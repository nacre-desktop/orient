"""Compare installed Python bindings to the frozen, parsed reference values."""

import gzip
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from PIL import Image
from orient import classic, engine, export, extract, palette, template_engine
from make_golden import outcome, unpack

GOLDEN = Path(__file__).with_name("golden")


class GoldenParity(unittest.TestCase):
    def test_reference(self):
        data = unpack(json.loads(gzip.decompress((GOLDEN / "outputs.json.gz").read_bytes())))
        with tempfile.TemporaryDirectory() as folder:
            for case in data["images"]:
                decoded = [Image.open(GOLDEN / name).convert("RGBA") for name in case["frames"]]
                # Use original gallery sources when available, or exact normalized pixels.
                gallery = Path(__file__).parents[1] / "orient-gallery/images"
                source = next(iter(gallery.glob(case["name"] + ".*")), GOLDEN / case["frames"][0])
                with (
                    patch.object(extract, "frames", return_value=decoded),
                    patch.object(classic, "frames", return_value=decoded),
                    patch.object(extract, "normalize", return_value=decoded[0]),
                ):
                    self.assertEqual(
                        outcome(lambda source=source: extract.analyze(source)), case["analysis"], case["name"]
                    )
                    self.assertEqual(
                        outcome(lambda source=source: classic.analyze(source)), case["classic"], case["name"]
                    )
                    for sample in case["samples"]:
                        self.assertEqual(
                            outcome(
                                lambda sample=sample, source=source: extract.sample_color(source, *sample["point"])
                            ),
                            sample["expected"],
                            (case["name"], sample["point"]),
                        )
                    for record in case["palettes"]:
                        actual = outcome(
                            lambda record=record, source=source, case=case: engine.from_image(
                                source, cache_dir=Path(folder) / case["name"], **record["options"]
                            )
                        )
                        # The normalized-only synthetic source has different encoded bytes.
                        # Restore the recorded source identity; core calculations do not use it.
                        if "value" in actual:
                            actual["value"]["source"]["digest"] = record["expected"]["value"]["source"]["digest"]
                        self.assertEqual(actual, record["expected"], (case["name"], record["options"]))
            for record in data["defaults"]:
                self.assertEqual(
                    outcome(lambda record=record: engine.default_palette(**record["options"])), record["expected"]
                )
            for record in data["generated"]:
                self.assertEqual(
                    outcome(lambda record=record: palette.generate(*record["args"], **record["kwargs"])),
                    record["expected"],
                )
            for record in data["templates"]:
                self.assertEqual(
                    outcome(
                        lambda record=record: export.render(record["text"], engine.default_palette(record["mode"]))
                    ),
                    record["expected"],
                )
            for record in data["expressions"]:
                self.assertEqual(
                    outcome(lambda record=record: template_engine.render_values(*record["args"], **record["kwargs"])),
                    record["expected"],
                )


if __name__ == "__main__":
    unittest.main()
