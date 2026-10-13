<p align="center"><img src="assets/branding/nacre-avatar-transparent.png" width="140" alt="Mother-of-pearl shell logo"></p>
<h1 align="center">Orient</h1>
<p align="center">Wallpaper colors. Readable palettes. Themes anywhere.</p>
<p align="center"><a href="#use-orient-in-your-setup">Get started</a> · <a href="#what-it-does">Features</a> · <a href="#using-palettes-and-templates">Templates</a> · <a href="https://github.com/nacre-desktop/nacre">Nacre</a></p>

**Development preview · 0.1.0-dev.6 (source only).** The engine works and has regression tests;
its public packaging/API is being stabilized before a 1.0 release.

Orient is a Rust color engine with a Python library and a standalone CLI. It analyzes images or sampled
animation frames, preserves source color families, and creates readable dark/light
palettes. It does not require Hyprland, Quickshell, Nacre, a display server or a
running desktop. [Nacre](https://github.com/nacre-desktop/nacre) is one consumer.

## Use Orient in your setup

Orient can replace the palette-generation and template-rendering part of a
Matugen workflow. It runs without Nacre, a compositor, or Python. You choose the
wallpaper, render a theme, and decide which applications should load it.

### Build and install the binary

Install Rust **1.85 or newer**, a C/C++ compiler, Make, CMake, pkg-config, and the
LCMS2 development headers using your operating system's package manager. For
example, Debian/Ubuntu package names are `build-essential cmake pkg-config
liblcms2-dev`; Arch names are `base-devel cmake pkgconf lcms2`. FFmpeg is optional
for video wallpapers. These are build prerequisites; Python is not required.

```sh
git clone https://github.com/nacre-desktop/orient.git
cd orient
nice -n 10 cargo build --release --locked -j2 -p orient-cli
./target/release/orient --version

# Optional user-local installation; add ~/.local/bin to PATH if needed.
install -Dm755 target/release/orient "$HOME/.local/bin/orient"
orient --help
```

The source checkout contains all bundled templates, and the binary embeds them.
Keep the checkout if you want to edit a template or render an individual bundled
file. The binary alone can export all built-ins. Source-only dev6 has no new
release artifacts; the [Arch recipes](packaging/arch/) also build from source.
Image decoding supports PNG, JPEG, WebP, GIF, BMP and TIFF. See the
[decoder contract](tests/parity/decoder/README.md) for exact parity coverage.

### Generate, export, and render

Run these from the checkout with a wallpaper you own:

```sh
orient palette wallpaper.jpg --personality natural --mode dark > palette.json
orient palette wallpaper.jpg --personality natural --mode light > palette-light.json

# Export all built-ins into a fresh directory; existing files are refused.
orient export palette.json ./themes-dark

# Render one file to stdout. Redirect only to a file you intend to replace.
orient render palette.json src/orient/templates/waybar.css.in > colors.css

# Render your own template tree, preserving subdirectories and removing .in.
orient export palette.json ./custom-themes --templates ./my-templates
```

Natural preserves distinct wallpaper pigments. Other choices are Pigment,
Harmony, Pop, Mist, Vivid, Pearl and Tide (use lowercase CLI names). `--mode` is
`dark` or `light`; use the same mode across apps for coherent foregrounds and
backgrounds. Generated palette JSON uses `formatVersion: 1` and carries its input
settings. It also contains the source image path, so review metadata before
sharing it.

Export writes files; it does not install themes, run hooks, or reload applications.
Use a fresh staging directory for each export, inspect the result, and copy or
include only the files you want. Keep backups of configuration you replace.

### Bundled targets

All filenames below are relative to the export directory. The original Alacritty,
btop, Discord, Firefox, foot, GTK, Hyprland, kitty, Neovim, Obsidian, rofi, Spotify
and VS Code templates remain available.

| Target | Exported file | How to use it |
| --- | --- | --- |
| Ghostty | `ghostty.conf` | Load with `config-file`, or install as a Ghostty theme. |
| WezTerm | `wezterm.lua` | Set `config.colors = dofile("path/to/wezterm.lua")`. |
| niri | `niri.kdl` | Include after existing layout settings; color properties merge. |
| Waybar | `waybar.css` | Import after layout CSS. |
| mako | `mako.conf` | Merge global colors and urgency criteria into your config. |
| SwayNC | `swaync.css` | Import after layout CSS; reload styles with `swaync-client -rs`. |
| fuzzel | `fuzzel.ini` | Merge the `[colors]` section into your config. |
| Helix | `helix.toml` | Save as `themes/orient.toml`; select `theme = "orient"`. |
| Zed | `zed-theme.json` | Save in Zed's `themes` directory; select `Orient dark` or `Orient light`. |
| Starship | `starship.toml` | Merge the module tables into your existing config. |
| tmux | `tmux.conf` | Source from your config or with `tmux source-file`. |
| fish | `fish.fish` | Source from `config.fish`; uses session globals, not universal state. |
| Zathura | `zathurarc` | Include from your config; document recoloring remains opt-in. |
| qt5ct / qt6ct | `qt5ct/colors/orient.conf`, `qt6ct/colors/orient.conf` | Copy into the respective `colors` directory and select in the Colors tab. |
| Hyprlock | `hyprlock.conf` | Merge color keys into your existing `input-field` widget. |
| Dunst | `dunst.conf` | Merge urgency sections into `dunstrc`. |
| Yazi | `yazi-theme.toml` | Merge into `theme.toml` (current `[mgr]` syntax). |
| Lazygit | `lazygit.yml` | Merge `gui.theme` into `config.yml`; use a matching terminal palette. |
| Cava | `cava.conf` | Merge `[color]`; use an output backend that supports hex colors. |

Every new template links its application's documented keys. Strict JSON cannot
contain comments, so Zed stores its documentation link in the leading `_comment`
metadata field and includes its theme schema URL. Qt schemes use 21 ordered
QPalette roles with **AARRGGBB** colors; qt6ct supplies Accent from Highlight when
needed. fuzzel uses **RRGGBBAA** without `#`.

The [template tests](tests/test_adoption_templates.py) render the public sample
palettes in both modes, parse output, and compare native and Python exports.
See [parser coverage and limitations](docs/template-validation.md). Syntax checks
cannot prove how every installed app/version will draw the colors.

### Template syntax

An Orient role token is six hex digits **without** `#`. Match the application's
color notation:

```css
/* my-templates/colors.css.in */
window {
    background-color: #{{surface}};
    color: #{{onSurface}};
}
button:selected {
    background-color: #{{primaryContainer}};
    color: #{{onPrimaryContainer}};
}
```

Pair `surface`/`onSurface`, `primary`/`onPrimary`,
`primaryContainer`/`onPrimaryContainer`, and the corresponding secondary, tertiary,
and error roles. `outline` and `outlineVariant` are borders. `term0` through
`term15` are the 16 ANSI terminal slots. [Palette JSON](docs/palette-json.md)
describes the remaining roles. `{{mode}}` expands to `dark` or `light`.

The supported Matugen expression subset includes:

| Expression | Meaning |
| --- | --- |
| `{{ colors.primary.default.hex }}` | Current palette's primary with `#`. |
| `{{ colors.on_surface.default.hex_stripped }}` | Snake-case role, no `#`. |
| `{{ colors.primary.hex }}` | Shorthand for the current scheme. |
| `{{ colors.primary.dark.hex }}` / `.light.hex` | Explicit scheme; never substitutes the other mode. |
| `.hex_alpha`, `.rgb`, `.rgba`, `.hsl`, `.hsla` | Formatted color representations. |
| `.red`, `.green`, `.blue`, `.alpha` | Integer channels (alpha is 255 before filtering). |
| `{{ colors.source_color.default.hex }}` | Orient's source/overtone color. |
| `{{ palettes.primary._50.hex }}` | Derived primary tone; families have tones `_0` through `_100`. |
| `{{ base16.base05.default.hex }}` | Derived Base16 role (`base00` through `base0F`). |
| `{{ custom.font }}` / `{{ image }}` | Custom variable / source path, when provided. |

Filters can be chained: `replace: "_", "-"`, `lighten: 20`,
`auto_lightness: 20`, `set_lightness: 60`, `saturate: 20, "hsl"` (or `"hsv"`),
and `set_alpha: 0.5`. Lightness/saturation parameters are percentages; alpha is
0–1. For example:

```css
button { background-color: {{ colors.primary.default.rgba | set_alpha: 0.5 }}; }
```

Map loops use `<* for name, value in colors *>…<* endfor *>`; `palettes`, `base16`,
and their nested maps are also supported. Boolean conditions use
`<* if {{is_dark_mode}} *>…<* else *>…<* endif *>`. Inside loops,
`{{loop.first}}` and `{{loop.last}}` can control separators. For example:

```text
<* for name, value in colors *>
--{{name | replace: "_", "-"}}: {{value.default.hex}};
<* endfor *>
```

To use both modes or custom variables explicitly:

```sh
# variables.json contains an object, for example {"font": "Noto Sans"}.
orient render palette.json my-template.css.in \
    --companion palette-light.json --variables variables.json > my-theme.css
```

`--companion` and `--variables` are options for `render` only. For a custom tree
that needs explicit companions or variables, render its files individually;
`export --templates` does not accept those flags.

An explicit opposite-mode token requires a companion. Orient can generate it
from an unchanged dynamic source when the engine fingerprint matches; supplying
`--companion` avoids depending on that source. A template's `image` variable is
plain text: quote/escape it appropriately for your output format. Prefix `\{{`
to emit a literal `{{` rather than expand a token. Missing roles, unsupported
expressions, and unclosed blocks fail instead of producing partial substitutions.

### Coming from matugen

Keep supported `colors.<role>.default.<format>` expressions; do not add a second
`#` before `.hex`. Replace Matugen's generation invocation with `orient palette`,
then use `orient render` for one template or `orient export --templates` for a tree
of `.in` files. Matugen config-file entries and post-hooks are not imported; wire
application includes/reloads into your own script after reviewing the output.
Orient does not implement arbitrary scripting, includes, arithmetic, or filters
outside the list above. Its wallpaper-driven colors and derived tonal/Base16
contexts follow Orient's policies, so matching expression syntax does not promise
Matugen's palette colors. Test imported templates before replacing live files.

### Worked example: niri + Waybar

This example uses niri 25.11 or newer, which supports includes. On older
versions, merge the generated layout color keys into the main config instead.

From the checkout, first stage the files without changing either application's
configuration:

```sh
orient palette wallpaper.jpg --mode dark --personality natural > palette.json
mkdir -p ./orient-stage
orient render palette.json src/orient/templates/niri.kdl.in > ./orient-stage/colors.kdl
orient render palette.json src/orient/templates/waybar.css.in > ./orient-stage/colors.css
niri validate --config ./orient-stage/colors.kdl
```

After reviewing the staged files, copy them to
`~/.config/niri/orient-colors.kdl` and `~/.config/waybar/orient-colors.css`.
In niri's `config.kdl`, add this at top level **after** existing layout settings:

```kdl
include "orient-colors.kdl"
```

niri's [include rules](https://niri-wm.github.io/niri/Configuration:-Include.html)
merge the focus-ring and border color properties, preserving widths and other
layout settings. Enable your preferred ring/border in your existing layout;
the theme only supplies colors. Existing `active-gradient`, `inactive-gradient`,
or `urgent-gradient` settings take precedence over solid colors: remove/comment
those gradient settings if you want this template
([gradient rules](https://niri-wm.github.io/niri/Configuration:-Layout.html#gradients)).
Validate the complete config with
`niri validate --config "$HOME/.config/niri/config.kdl"`; niri reloads a valid
saved configuration automatically.

At the **end** of Waybar's existing `style.css`, add:

```css
@import url("orient-colors.css");
```

Waybar uses GTK CSS and can import at that position. Existing rules with more
specific selectors can still override these colors; adjust those rules if needed.
The selectors cover both
niri's `.active` and Sway's `.focused` workspace states. Restart your Waybar process
using your existing launcher/service so it reads the stylesheet. To switch to
light mode, generate a light palette and render these same templates again.

### Worked example: Hyprland

The existing bundled template targets Hyprland's Lua configuration API. Render a
small border-color fragment:

```sh
orient palette wallpaper.jpg --mode dark --personality natural > palette.json
orient render palette.json src/orient/templates/hyprland.lua.in > ./orient-borders.lua
```

After inspection, save it as `~/.config/hypr/orient-borders.lua`, then load it
**after** your main `hl.config` call in `hyprland.lua`:

```lua
dofile(os.getenv("HOME") .. "/.config/hypr/orient-borders.lua")
```

It changes active/inactive border colors. Set border width in your own config.
See the [upstream Lua example](https://github.com/hyprwm/Hyprland/blob/main/example/hyprland.lua)
for the API used by the bundled template, then reload through your normal
Hyprland workflow. For older releases using Hyprlang, create `borders.conf.in`:

```text
# Docs: https://wiki.hypr.land/Configuring/Variables/
general {
    col.active_border = rgba({{primary}}ff) rgba({{secondary}}ff) 45deg
    col.inactive_border = rgba({{outlineVariant}}ff)
}
```

Render it with `orient render palette.json borders.conf.in > borders.conf`,
inspect it, and use a `source = …/borders.conf` entry in your existing Hyprlang
config. Choose the example that matches your installed compositor's config API.

## Python library

The optional Python 3.13+ library uses Pillow for image decoding. Install it from
source in an isolated environment:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install .
.venv/bin/python -m orient --version
```

The distribution is `orient-colors`; the module CLI is `python -m orient`. It does
not install the standalone `orient` console command.

```python
from orient.engine import from_image
from orient.export import export, render

palette = from_image("wallpaper.jpg", mode="dark", personality="natural")
css = render(":root { --accent: #{{primary}}; }", palette)
files = export(palette, "./themes")
```

Choose a fresh export directory; existing files are never silently replaced.
See the [stable entry points](docs/api.md), [palette JSON format 1](docs/palette-json.md)
and [0.x versioning policy](docs/versioning.md).

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
stdout. JSON `formatVersion` is 1; the engine version is independent. The built-in
templates target terminals/editors and common desktop applications. Matugen color
expressions, color-map loops and the supported filters/conditions work alongside
Orient tokens; arbitrary scripts/includes are not executed.

The engine uses a private XDG cache and never applies desktop settings. Use Nacre
or another caller to publish a palette. Point samples, source regions/coverage and
contrast/vision diagnostics are included for applications that build previews.

Orient-owned code is **MIT**: [LICENSE](LICENSE). Pillow is an independently
installed dependency with its own license. Original synthetic regression images
are public-domain fixtures. See [CHANGELOG](CHANGELOG.md) for user-facing changes.

See the [implementation-origin record](docs/provenance.md) for the independent
engine implementation and historical integration context.

For development checks and PR expectations, see [Contributing](CONTRIBUTING.md).
Report vulnerabilities through the private channel in [Security](SECURITY.md).

Building from source requires a Rust toolchain (Rust 1.85 or newer). Prebuilt
abi3 wheels support Python 3.13 and newer; Pillow still decodes images.
