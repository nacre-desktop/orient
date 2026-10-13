# Engine boundary

The Cargo workspace contains `orient-cli` (Unix command-line I/O and decoding),
`orient-core` (pure deterministic calculations,
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

The standalone CLI uses the `image` codecs, LCMS2 for ICC conversion, and a
Lanczos3 thumbnail. GIF/video sampling uses four fixed temporal points. A
50-megapixel check precedes pixel allocation; high bit depth is refused. Video
subprocesses run in separate process groups, killed/reaped after 10 s (probe)
or 20 s (decode). Cache writes are atomic and 0600. Rust cache filenames include
a separate decoder hash/prefix to protect Pillow-backed Python cache entries.

The binary embeds the same 13 templates. It has no Python runtime dependency;
LCMS2 is a native dependency. Arch `orient` supplies the command;
`python-orient-colors` supplies the library and `python -m orient`.

`tests/parity/decoder-report.json` records all roles across both modes and all
personalities for each gallery image. Original PNG pixels decode identically;
thumbnail differences cross the existing 0.045 accent-separation threshold in
dark Harmony (`coast-day`) and Mist (`warm-room`). Their largest differences are
0.078494 and 0.214539 OKLab respectively. This is why Python retains Pillow.
No core thresholds or comparison tolerances changed.

Pillow plugin IDs outside the binary's six-format set (availability depends on
Pillow's build and optional tools): AVIF, BLP, BUFR, CUR, DCX, DDS, DIB, EPS,
FITS, FLI, FTEX, GBR, GRIB, HDF5, ICNS, ICO, IM, IMT, IPTC, JPEG2000, MCIDAS,
MPEG, MSP, PCD, PCX, PIXAR, PPM, PSD, QOI, SGI, SPIDER, SUN, TGA, WMF, XBM,
XPM and XVTHUMB. MPEG video can still use the documented ffmpeg-supported
extensions. Animated WebP/APNG and multipage TIFF currently use their first
frame; CMYK/LAB ICC profiles are unsupported at the RGB decoder boundary.

Reproduce decoding/performance measurements after building the release binary:

```sh
cargo build --release --example normalized_frames
python tests/parity/compare_decoders.py target/release/orient \
  --dump-binary target/release/examples/normalized_frames
python tests/parity/benchmark_cli.py target/release/orient /path/to/original/src
```

The CLI benchmark measures fresh-cache wall time including process startup. Its
small native RSS helper avoids counting inherited Python harness pages.
