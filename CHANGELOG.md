# Changelog

## 0.1.0-dev.6 (unreleased)

- Expose bounded native decoding as `orient-decoder` for direct Rust consumers.
  Preserve Pillow thumbnail notices and freeze 55 additional normalized frames.

- Match Pillow thumbnail arithmetic and JPEG draft decoding, restoring exact
  native gallery and tested wallpaper palettes.
  Add a fingerprinted JSON interface for native desktop consumers and safe
  decoder subprocess cancellation.

- Move deterministic engine calculations into Rust with abi3 Python bindings.
  Preserve Pillow decoding, Python API, palette colors and template output.
  Source builds now require Rust and maturin.
- Add the standalone Rust `orient` binary with LCMS2 conversion and bounded video
  decoding. The wheel retains `python -m orient` and drops its console script;
  Arch splits the binary and Python library. Pillow remains Python's decoder.

## 0.1.0-dev.5

Add tokenless TestPyPI-first publishing with a protected production approval gate,
installed-wheel checks on Python 3.13/3.14, and the python-orient-colors Arch recipe
with clean-chroot validation. Document CLI/library entry points, palette JSON
format 1, and the 0.x compatibility policy. Palette and engine behavior is unchanged.

## 0.1.0-dev.4

Support Pillow 11 and 12 without forcing library users to one patch version.
Document the independent implementation and its historical integration context.

## 0.1.0-dev.3

Orient-owned source is now MIT-licensed for reuse in other desktops, libraries and
applications. Package metadata and the bundled license agree. Palette generation
and template behavior are unchanged; palette JSON format remains 1.

## 0.1.0-dev.2

Initial standalone Orient preview with image/frame analysis, readable dark/light
palettes, source color families/regions, Matugen-compatible templates and export.
The palette JSON format is1. No desktop or running display server is required.
