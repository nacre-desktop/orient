# Frozen dashboard artwork staging

These reference bytes were captured from Pillow 12.3.0 before porting each path.
They use Pillow's zlib 1.3.1.zlib-ng encoder. They reproduce the current
dashboard-media path: `Image.open` without explicit EXIF/ICC processing,
`thumbnail((512, 512))` with default BICUBIC/reducing_gap=2, `convert("RGB")`
and default PNG save. JPEG's implicit draft request is 1024x1024.

The 64-case manifest references 49 existing decoder inputs, seven synthetic RGB
JPEG sizes (including fractional draft boxes), two CMYK JPEGs, one tiny PPM, and
five 16-bit PNGs. Successful cases record dimensions, decoded RGB hash and exact
PNG bytes/hash. One large grayscale 16-bit case freezes Pillow's pre-reduction
failure (`ValueError: image has wrong mode`) instead of silently accepting it.
Existing 128px decoder behavior remains independently gated.
Do not regenerate these outputs to bless a Rust mismatch.

Observed metadata: ICC is retained; EXIF is not automatically copied into the
saved PNG. Palette/GIF transparency becomes RGB tRNS. Compare encoded bytes too;
pixel-only equivalence is insufficient for this helper's frozen file contract.

The Rust integration test compares all encoded bytes and that error, without regenerating any
references. PNG filtering and the row-by-row deflate sequence follow
[Pillow ZipEncode.c](https://github.com/python-pillow/Pillow/blob/12.3.0/src/libImaging/ZipEncode.c).
The bicubic and CMYK arithmetic retain the MIT-CMU notice in `LICENSE-Pillow`.
