# Palette JSON: formatVersion 1

Both `orient palette` and `orient.engine.from_image` return this format.
`default_palette` returns the same core envelope with `name="default"` and no
image-source fields. `orient analyze` is an analysis result, not a palette.

| Field | Type and meaning |
| --- | --- |
| `formatVersion` | Integer `1`. Consumers must reject unsupported major formats. |
| `name` | `dynamic` for image-derived palettes; `default` for the built-in fallback. |
| `mode` | `dark` or `light`. |
| `variant`, `flavour` | Strings recording requested palette options. |
| `colours` | Object mapping semantic roles to six lowercase RGB hex digits **without `#`**, e.g. `"primary": "47a99a"`. |
| `engine` | Opaque engine fingerprint. Changes invalidate palette caches. |
| `input` | Object recording generation settings. Default palettes contain only a personality. |
| `accessibility` | Diagnostics object with contrast checks and color-vision differences; see below. |
| `source` | Dynamic palettes only: analysis/source identity, selected accent, suggested options and image statistics. |
| `roleSources` | Dynamic palettes only: primary/secondary/tertiary source records with source color, selection type, coverage and regions. |
| `provenance` | Dynamic palettes only: per-role derivation records (`parent`, `type`, `original`, `adjusted`). |

Colors are opaque sRGB values. Common roles include `background`, `surface`,
`surfaceContainer`, `surfaceContainerLow`, `surfaceContainerHigh`, `frame`,
`onSurface`, `onSurfaceVariant`, `primary`, `secondary`, `tertiary`, their
`onPrimary`/`onSecondary`/`onTertiary` counterparts, `error`, `success`, and
`term0` through `term15`. Additional roles and legacy aliases are emitted for
portable templates. Use role names, not object key order.

The `source` object includes the absolute `path`, source-file SHA-256 `digest`,
`seed`, `selected`, `candidates`, `families`, and `options`. Candidate/family
records carry hex colors, fractional `coverage`, perceptual statistics, normalized
`location` (`x`, `y`), and grid `regions` (`columns`, `rows`, `cells` with
`index`/fractional `coverage`). Treat source information and diagnostics as
metadata: it is not required to render ordinary single-mode templates.

`accessibility` reports diagnostics rather than a universal accessibility
certification. Contrast values are ratios, not percentages. `text` contains foreground/background role pairs, `ratio` and `passes` (4.5:1
threshold). `textPasses` and `minimumTextContrast` summarize them. `distinctions`
compares accent pairs by simulated vision, `warnings` gives advisory strings,
`simulation` identifies the approximation, and `simulated` contains transformed
role colors. Applications should tolerate additional diagnostic keys.

A minimal consumer reads colors this way:

```python
import json

palette = json.loads(open("palette.json").read())
if palette["formatVersion"] != 1:
    raise ValueError("Unsupported Orient palette format")
accent = "#" + palette["colours"]["primary"]
```

Retain the full generated object for `orient export`/`render`; the table above is
not a recipe for constructing incomplete palettes. New metadata keys and color
roles may be added within format 1; consumers should ignore unknown keys. Removal,
renaming or incompatible interpretation of existing core fields requires a new
`formatVersion`, independently of the Python package version. Color output can
change between engine versions without changing the serialization format.
