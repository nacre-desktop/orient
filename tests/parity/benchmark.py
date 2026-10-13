"""One fresh process/cache per sample; report cache-miss time and process peak RSS."""

import argparse
import json
from pathlib import Path
import resource
import statistics
import subprocess
import sys
import tempfile
import time


def child(image):
    from orient.engine import from_image

    with tempfile.TemporaryDirectory() as cache:
        start = time.perf_counter()
        from_image(image, cache_dir=cache)
        elapsed = time.perf_counter() - start
    print(json.dumps({"seconds": elapsed, "peak_rss_kib": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss}))


def measure(repeats):
    images = sorted((Path(__file__).parents[1] / "orient-gallery/images").iterdir())
    rows = []
    for image in images:
        values = [
            json.loads(subprocess.check_output([sys.executable, __file__, "--child", str(image)], text=True))
            for _ in range(repeats)
        ]
        rows.append(
            {
                "image": image.name,
                "median_seconds": statistics.median(v["seconds"] for v in values),
                "peak_rss_kib": max(v["peak_rss_kib"] for v in values),
                "runs": repeats,
            }
        )
    print(json.dumps(rows, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--child", type=Path)
    parser.add_argument("--repeats", type=int, default=3)
    args = parser.parse_args()
    child(args.child) if args.child else measure(args.repeats)
