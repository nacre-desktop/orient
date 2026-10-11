"""Deterministic role snapshots; no live theme or per-user state writes."""

from pathlib import Path
import sys
import json
import tempfile

SOURCE = Path(__file__).resolve().parents[2] / "src"
sys.path.insert(0, str(SOURCE))
from orient.engine import from_image
from orient.palette import PERSONALITIES


def snapshot():
    result = {}
    with tempfile.TemporaryDirectory() as cache:
        for path in sorted(Path(__file__).with_name("images").glob("*.png")):
            result[path.stem] = {}
            for mode in ("dark", "light"):
                policies = [(name, None) for name in PERSONALITIES] + [("natural", True)]
                for personality, background in policies:
                    p = from_image(
                        path,
                        mode=mode,
                        personality=personality,
                        hour=18,
                        cache_dir=cache,
                        background_from_wallpaper=background,
                    )
                    result[path.stem][mode + "-" + personality + ("-wallpaper-background" if background else "")] = {
                        "colours": p["colours"],
                        "minimumContrast": p["accessibility"]["minimumTextContrast"],
                        "bodySources": {k: v["hex"] for k, v in p["source"]["body"].items()},
                        "seed": p["source"]["selected"],
                    }
    return result


if __name__ == "__main__":
    Path(__file__).with_name("palettes.json").write_text(json.dumps(snapshot(), sort_keys=True, indent=2) + "\n")
