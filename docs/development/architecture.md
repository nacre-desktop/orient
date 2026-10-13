# Engine boundary

The Cargo workspace contains `orient-cli` (Unix command-line I/O and decoding),
`orient-core` (pure deterministic calculations,
with unsafe code forbidden) and `orient-py` (PyO3 abi3 bindings for Python 3.13+).
The package is built with maturin. All crates are MIT and unpublished.

Python modules retain the public signatures and delegate calculations to Rust.
Pillow normalization and temporal frame sampling, ffmpeg/ffprobe subprocesses,
cache I/O, the Python CLI and exclusive export writes remain in Python. The
core accepts bounded RGBA frames and returns palettes, diagnostics and text.
Engine identity combines the package version with a build-time source
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
python tests/parity/check_native_golden.py -q
cargo fmt --check
cargo clippy -- -D warnings
cargo test
python -m build
```

`python -m build` creates an sdist and builds its wheel. Test that wheel with
`scripts/smoke_wheel.py` in a clean virtual environment before distribution.
The Arch workflow similarly validates the newly built sdist; the development recipe
pins a tested Git revision rather than the old Python-only dev5 release archive.
A maintainer must prepare an immutable archive pin for a future release.

The standalone CLI and Rust consumers share the `orient-decoder` library,
which uses the `image` codecs, LCMS2 for ICC conversion, and a
Pillow-compatible integer reduction and Lanczos thumbnail. GIF/video sampling uses four fixed temporal points. A
50-megapixel check precedes pixel allocation; high bit depth is refused. Video
subprocesses run in separate process groups, killed/reaped after 10 s (probe)
or 20 s (decode). Cache writes are atomic and 0600. Rust cache filenames include
a separate decoder hash/prefix to protect Pillow-backed Python cache entries.

The binary embeds the same 13 templates. It has no Python runtime dependency;
LCMS2 is a native dependency. Arch `orient` supplies the command;
`python-orient-colors` supplies the library and `python -m orient`.

`tests/parity/decoder-report.json` records all roles across both modes and all
personalities for each gallery image, plus public lossless/lossy WebP and JPEG
derivatives. The normalized pixels and all palette roles match Pillow for all
160 combinations, including the formerly divergent accent choices.
Native thumbnail arithmetic targets Nacre's pinned Pillow 12.3. Pillow 11
orders resize passes differently for extremely tall transparent images; two
frozen pixel references keep the native contract independent of CI's Pillow version.
Pixel tests cover RGB, RGBA, grayscale, grayscale-alpha and indexed PNG modes, BMP/TIFF/WebP and animated GIF mode promotion,
integer reduction, narrow images and rounding boundaries. The resize arithmetic
was independently implemented from Pillow's documented/source behavior; no
Pillow code is linked into the binary. RGB/grayscale JPEG uses bundled libjpeg-turbo with the same 512px draft request
before EXIF/thumbnail processing. ICC conversion and
additional animation formats can still differ. No core thresholds or comparison
tolerances changed.

Local consumers can send one JSON request to `orient --rpc` on stdin:
`{"operation":"info","payload":{}}`. Responses carry `interfaceVersion`,
`engine` and either `result` or an `error` with `kind` and `message`. The interface
exposes the full engine options and pure core operations, including raw template
rendering. Core dispatch is shared with PyO3. Consumers must verify the interface
version and fingerprint. Native identity includes both core and decoder source
hashes; Python identity includes the core hash only. SIGINT/SIGTERM cancellation
also kills and reaps active ffmpeg/ffprobe process groups.

Long-lived consumers can use `orient --rpc --serve`. Each stdin line carries
`id` (an integer or string), `operation` and `payload`; each stdout line repeats
that ID and the same response envelope. Replies flush immediately, operation
errors leave the server available, and EOF exits. Requests are limited to 16 MiB.
Idle servers block on stdin. Cancellation ends the server after decoder cleanup;
consumers should discard a timed-out child and start a new one on the next call.

Pillow plugin IDs outside the binary's six-format set (availability depends on
Pillow's build and optional tools): AVIF, BLP, BUFR, CUR, DCX, DDS, DIB, EPS,
FITS, FLI, FTEX, GBR, GRIB, HDF5, ICNS, ICO, IM, IMT, IPTC, JPEG2000, MCIDAS,
MPEG, MSP, PCD, PCX, PIXAR, PPM, PSD, QOI, SGI, SPIDER, SUN, TGA, WMF, XBM,
XPM and XVTHUMB. MPEG video can still use the documented ffmpeg-supported
extensions. Animated WebP/APNG and multipage TIFF currently use their first
frame; palette TIFF and CMYK/LAB ICC profiles are unsupported at the RGB decoder boundary.

Reproduce decoding/performance measurements after building the release binary:

```sh
cargo build --release --example normalized_frames
python tests/parity/compare_decoders.py target/release/orient \
  --dump-binary target/release/examples/normalized_frames
python tests/parity/benchmark_cli.py target/release/orient /path/to/original/src
```

The CLI benchmark measures fresh-cache wall time including process startup. Its
small native RSS helper avoids counting inherited Python harness pages.

The native cancellation adapter uses `ctrlc` (MIT/Apache-2.0), `nix` (MIT),
`dispatch2` (Zlib/Apache-2.0/MIT) and `objc2` (MIT); the latter two are
target-specific dependencies. `gif` (MIT/Apache-2.0) supplies frame metadata.

JPEG decoding uses `turbojpeg` and `turbojpeg-sys` under their MIT license option,
with `cmake` (MIT/Apache-2.0) building the bundled native codec. Libjpeg-turbo's
BSD-style/IJG/Zlib notices ship with the native command; Orient-owned code remains
MIT. CMake and a C compiler are needed for CLI source builds, not for the abi3 wheel.
The cancellation adapter's additional target dependencies `block2` and
`objc2-encode` are MIT licensed.
