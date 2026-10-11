# Orient

Wallpaper colors, readable palettes and portable theme templates.

**Development preview · 0.1.0-dev.2.** The engine works and has regression tests;
its public packaging/API is being stabilized before a 1.0 release.

Orient is a standalone Python library and CLI. It analyzes images or sampled
animation frames, preserves source color families, and creates readable dark/light
palettes. It does not require Hyprland, Quickshell, Nacre, a display server or a
running desktop. [Nacre](https://github.com/nacre-desktop/nacre) is one consumer.

## Try it

Python 3.13+ is required. Install in an isolated environment:

```sh
python3 -m venv .venv
.venv/bin/pip install .
.venv/bin/orient palette wallpaper.jpg --personality natural --mode dark > palette.json
.venv/bin/orient analyze wallpaper.jpg
.venv/bin/orient export palette.json ./themes
.venv/bin/orient render palette.json ./my-template.css > ./colors.css
```

The distribution name is `orient-colors`; the command and import module are
`orient`. No PyPI publication is assumed by these source-install instructions.
FFmpeg enables video frame sampling; still images and animated GIFs use Pillow.

## What it does

- Natural, Pigment, Harmony, Pop, Mist, Vivid, Pearl and Tide personalities.
- Independent accent-tinted or wallpaper-derived backgrounds.
- Perceptual grouping, source coverage/regions, salience and stable frame sampling.
- Text contrast diagnostics and color-vision simulations.
- Semantic terminal colors and portable export templates.
- Orient tokens and supported Matugen color expressions, loops, filters and
  conditions. Arbitrary scripting and includes are intentionally unsupported.

Palette JSON has an independent `formatVersion` (currently **1**). The package's
0.x version does not reset this schema. Exact engine fingerprints invalidate
stale caches; previews never change your desktop. Export writes only the directory
or stdout explicitly requested. Nacre owns live palette publication.

## Using palettes and templates

The CLI prints JSON for palette/analyze/sample queries. Use `orient export` to
write themes into the directory you select, or `orient render` for a template on
stdout. JSON `formatVersion` is1; the engine version is independent. The built-in
templates target terminals/editors and common desktop applications. Matugen color
expressions, color-map loops and the supported filters/conditions work alongside
Orient tokens; arbitrary scripts/includes are not executed.

The engine uses a private XDG cache and never applies desktop settings. Use Nacre
or another caller to publish a palette. Point samples, source regions/coverage and
contrast/vision diagnostics are included for applications that build previews.

Orient-owned code is **GPL-3.0-only**: [LICENSE](LICENSE). Pillow is an independently
installed dependency with its own license. Original synthetic regression images
are public-domain fixtures. See [CHANGELOG](CHANGELOG.md) for user-facing changes.
