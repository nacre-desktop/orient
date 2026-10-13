"""Linux CLI wall time and per-child peak RSS, including interpreter/startup."""

import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).parents[2]


def sample(command, env, rss_helper):
    start = time.perf_counter()
    with subprocess.Popen(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env) as process:
        _, status, _ = os.wait4(process.pid, 0)
        process.returncode = os.waitstatus_to_exitcode(status)
        if process.returncode:
            raise RuntimeError(f"Benchmark command exited {process.returncode}")
    elapsed = time.perf_counter() - start
    # A tiny native parent avoids counting inherited Python harness pages.
    with tempfile.TemporaryDirectory() as temporary:
        measured_env = {**env, "XDG_CACHE_HOME": str(Path(temporary) / "cache")}
        peak = int(subprocess.check_output([str(rss_helper), *command], env=measured_env, text=True).strip())
    return {"seconds": elapsed, "peak_rss_kib": peak}


def measure(binary, reference_source, repeats, rss_helper):
    result = {"binary_bytes": binary.stat().st_size, "startup": [], "cache_miss": []}
    commands = [
        ("python_before", [sys.executable, "-m", "orient"], {"PYTHONPATH": str(reference_source)}),
        ("python_rust_core", [sys.executable, "-m", "orient"], {"PYTHONPATH": ""}),
        ("rust_binary", [str(binary)], {"PYTHONPATH": ""}),
    ]
    for label, command, extra in commands:
        values = [sample([*command, "--version"], {**os.environ, **extra}, rss_helper) for _ in range(repeats)]
        result["startup"].append(
            {
                "implementation": label,
                "runs": repeats,
                "median_ms": statistics.median(v["seconds"] for v in values) * 1000,
                "peak_rss_kib": max(v["peak_rss_kib"] for v in values),
            }
        )
        for image in sorted((ROOT / "tests/orient-gallery/images").iterdir()):
            values = []
            for _ in range(3):
                with tempfile.TemporaryDirectory() as cache:
                    values.append(
                        sample(
                            [*command, "palette", str(image)],
                            {**os.environ, **extra, "XDG_CACHE_HOME": cache},
                            rss_helper,
                        )
                    )
            result["cache_miss"].append(
                {
                    "implementation": label,
                    "image": image.name,
                    "runs": 3,
                    "median_ms": statistics.median(v["seconds"] for v in values) * 1000,
                    "peak_rss_kib": max(v["peak_rss_kib"] for v in values),
                }
            )
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("reference_source", type=Path)
    parser.add_argument("--repeats", type=int, default=10)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as temporary:
        helper = Path(temporary) / "rss-helper"
        subprocess.run(["cc", "-O2", str(Path(__file__).with_name("rss_helper.c")), "-o", str(helper)], check=True)
        measure(args.binary.resolve(), args.reference_source.resolve(), args.repeats, helper)
