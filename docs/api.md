# Python API (0.x)

Install `orient-colors`; import `orient`. Python 3.13+ and Pillow 11 or 12 are
required. These entry points are the supported integration surface. Their
compatibility policy is in [versioning](versioning.md). Helpers in other modules
are implementation details.

```python
from orient.engine import from_image
from orient.export import export, render

palette = from_image("wallpaper.jpg", mode="dark", personality="natural")
css = render(":root { --accent: #{{primary}}; }", palette)
files = export(palette, "./themes")  # choose a new output directory
```

| Entry point | Result and contract |
| --- | --- |
| `orient.__version__` | PEP 440 package version, e.g. `0.1.0.dev6`. |
| `orient.FORMAT_VERSION` | Palette JSON format integer, currently `1`. |
| `orient.ENGINE_ID` | Engine/cache fingerprint including package version and implementation hash; use equality, do not parse it. |
| `orient.engine.from_image(path, ...)` | JSON-serializable [palette](palette-json.md) dictionary from a readable local image/animation/video. |
| `orient.engine.default_palette(mode="dark", variant="tonalspot", flavour="default")` | Palette without an image; omits image-source metadata. |
| `orient.extract.analyze(path)` | Analysis dictionary with source identity, seed, candidates, pigment families, source regions, body colors and mean lightness. |
| `orient.extract.sample_color(path, x, y)` | Six hexadecimal digits, without `#`; `x`/`y` are normalized coordinates in 0–1; out-of-range values raise `ValueError`. Samples the first frame. |
| `orient.export.render(template, palette, schemes=None, custom=None)` | Rendered text; no output-file writes. |
| `orient.export.export(palette, directory, templates=None, schemes=None, custom=None)` | List of absolute output path strings. Creates files exclusively and refuses existing target files. |

## Image palettes

`from_image` accepts `mode="dark"` or `"light"`, `personality="natural"`,
`"pigment"`, `"harmony"`, `"pop"`, `"mist"`, `"vivid"`, `"pearl"` or `"tide"`,
`accent` (hex color), and `overrides` (mapping of primary/secondary/tertiary to
hex colors). `background_from_wallpaper=None` uses the personality's policy;
`True`/`False` selects wallpaper-derived/accent-tinted backgrounds explicitly.

Other arguments are `variant="tonalspot"`, `flavour="default"`, `smart=False`
(select mode from source lightness), `harmony=False` (legacy personality choice),
`brightness=0.0`, `saturation=1.0`, `hour=12`, `time_tint=False`, and
`cache_dir=None`. Prefer `personality` over the legacy `harmony` flag. For exact
reproduction, preserve the returned `input` settings and engine fingerprint.

Images/GIF frames use Pillow; videos require the optional `ffmpeg` executable.
Palette generation reads the source and writes cache files beneath the XDG cache
root, or `cache_dir` when specified. It never publishes desktop settings. Source
metadata includes the absolute image path; remove private paths before sharing
JSON. Local I/O failures and invalid inputs can raise `OSError`/`ValueError`.

## Templates and exports

With `templates=None`, `export` uses the wheel's bundled `.in` templates. A custom
folder is searched recursively; relative paths are preserved and the final `.in`
is removed. Templates must remain regular files inside their selected folder.
An interrupted export may leave already-created files; use a fresh staging folder
when an all-or-nothing publication step is required.

`render` supports Orient tokens such as `#{{primary}}` and the documented subset
of Matugen color expressions. `schemes` maps mode names (`dark`/`light`) to
`colours` dictionaries, and `custom` maps custom variable names to values.
Templates requesting an opposite mode need its scheme. Orient can generate it
from an unchanged dynamic image source with a matching engine; otherwise supply
it explicitly. Rendering may therefore read/cache that companion palette.
Missing roles or invalid templates/palettes can raise `KeyError`/`ValueError`.
Arbitrary scripting and includes are unsupported.

## Rust image previews

`orient_decoder::rgb_preview_png(path, (maximum_width, maximum_height), blur_radius)`
returns RGB PNG bytes without publishing files or desktop settings. It applies
EXIF orientation, converts to RGB, makes a non-upscaling Pillow-compatible
bicubic thumbnail and optionally applies Pillow's three-pass Gaussian blur.
PNG ICC and transparency metadata are retained. Bounds must be nonzero and the
blur radius finite within 0–1000; the decoder's 50-megapixel limit applies.

This complements `artwork_png`, whose existing raw-orientation 512px contract
stays unchanged. The preview's sampling, blur rounding and PNG encoding match
frozen Pillow 12.3.0 vectors, including exact encoded bytes. The Pillow-derived
arithmetic retains the MIT-CMU notice in `crates/orient-decoder/LICENSE-Pillow`.

Generated RGBA artwork can use `encode_rgba_png(&image, compression_level)`,
`resize_rgba(&image, (width, height), Filter::Lanczos)` (or `Filter::Bicubic`),
and `blur_rgba(&image, radius)`. These operations preserve Pillow's encoded PNG
bytes, premultiplied-alpha resize rounding and four-channel blur arithmetic.
They accept `image::RgbaImage` buffers, write no files and have no desktop
dependencies. Bounds are nonzero and at most 50 megapixels, compression is 0–9,
and blur radius is finite within 0–1000. Resizing does not perform thumbnail
pre-reduction. Existing image decoding and RGB preview contracts are unchanged.

`load_rgba(path)` reads the full first frame without EXIF transposition or a
profile transform, matching Pillow's raw RGBA conversion, including GIF palette
transparency and 16-bit grayscale clamping. `thumbnail_rgba` keeps a generated
image within rectangular bounds without upscaling. `resize_rgba_region` resamples
a fractional source rectangle directly, preserving the crop coordinates used by
`ImageOps.fit`; rounding that rectangle to integer pixels changes the result.
These operations share the same pixel bound and write no files.

For opaque previews, `load_rgba_oriented` applies EXIF orientation before the
caller converts to RGB; `thumbnail_rgb` retains the integer pre-reduce used by
Pillow's RGB thumbnail path. `encode_rgb_png` serializes generated RGB pixels
without metadata. They share the existing codec, resampler and PNG implementation.
