# Engine boundary

The Cargo workspace contains `orient-core` (pure deterministic calculations,
with unsafe code forbidden) and `orient-py` (PyO3 abi3 bindings for Python 3.13+).
The package is built with maturin. All crates are MIT and unpublished.

Python modules retain the public signatures and delegate calculations to Rust.
Pillow normalization and temporal frame sampling, ffmpeg/ffprobe subprocesses,
cache I/O, the Python CLI and exclusive export writes remain in Python. The
core accepts bounded RGBA frames and returns palettes, diagnostics and text.
Engine identity combines the unchanged package version with a build-time source
hash, invalidating old caches once. No desktop state is published by the engine.

`tests/parity/golden` freezes the original Python outputs and normalized pixels.
Rust tests and the installed-package checker compare parsed values exactly,
excluding only engine fingerprints and source paths. Golden fixtures are never
regenerated to accept a porting mismatch. See `tests/parity/README.md`.

For development, install the package before running the Python tests:

```sh
python -m pip install . ruff==0.16.10
ruff format --check .
ruff check .
python -m unittest discover -s tests
python tests/parity/check_golden.py -q
cargo fmt --check
cargo clippy -- -D warnings
cargo test
python -m build
```

`python -m build` creates an sdist and builds its wheel. Test that wheel with
`scripts/smoke_wheel.py` in a clean virtual environment before distribution.
The Arch workflow similarly validates the newly built sdist; the recipe's
release URL/checksum continues to identify the published dev5 archive until
the maintainer prepares a future release.
