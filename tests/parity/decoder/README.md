# Frozen decoder reference

These public, synthetic inputs were generated and normalized by the unchanged
Python implementation at `d7a12ee`, with Pillow 12.3.0, before implementing the
Rust thumbnail path. `manifest.json` records dimensions and SHA-256 of each
expected RGBA pixel buffer. Expected PNGs are byte-exact pixel references;
compressed PNG file bytes are not the comparison boundary.

The corpus covers thumbnail boundaries, non-divisible integer reductions,
aspect-ratio rounding, tall/wide images, RGB, RGBA, L, LA, P, 1, indexed
transparency, GIF disposal/transparency, lossless WebP, BMP, TIFF, EXIF and ICC.
No private images or desktop state are included. Never regenerate expected
outputs to approve a Rust mismatch.

Run `cargo test -p orient-decoder --test pillow`. The gallery comparison also
requires zero non-JPEG palette and normalized-pixel differences, with public
lossy/lossless WebP and JPEG derivatives generated in a disposable directory.
JPEG differences are reported separately without relaxing non-JPEG equality.

Thumbnail source algorithms: Pillow 12.3.0 `Image.py` (`thumbnail`, `resize`),
`libImaging/Reduce.c`, `Resample.c`, and the alpha conversions in `Convert.c`:
<https://github.com/python-pillow/Pillow/tree/12.3.0/src>.
The Rust translation retains the MIT-CMU notice in
`crates/orient-decoder/LICENSE-Pillow`.
