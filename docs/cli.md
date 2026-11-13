# Orient CLI reference

The standalone `orient` binary is read-only: it reads images and writes only
the JSON on stdout, or the export directory you name. It never changes desktop
settings. `orient --version` prints the engine id, which doubles as the cache
fingerprint.

```text
orient analyze IMAGE
orient sample IMAGE X Y
orient palette IMAGE [options]
orient export PALETTE DIRECTORY [--templates DIR]
orient render PALETTE TEMPLATE [--companion FILE] [--variables FILE]
```

Paths accept `~` and `~/…`.

## `orient analyze IMAGE`

Prints the engine's analysis of the image as JSON. This is not a palette; use
it to see what the engine found before choosing a personality.

## `orient sample IMAGE X Y`

Prints the color at normalized coordinates `X`, `Y` (0–1 from the top-left,
first frame for videos) as `{"hex": "rrggbb"}`, six hex digits without `#`.

## `orient palette IMAGE`

Prints a `formatVersion: 1` palette JSON.

| Option | Default | Meaning |
| --- | --- | --- |
| `--mode dark|light` | `dark` | Which scheme to build. Build both for templates that want the opposite mode. |
| `--personality NAME` | `natural` | `natural`, `pigment`, `harmony`, `pop`, `mist`, `vivid`, `pearl`, `tide`. |
| `--background-from-wallpaper` / `--no-background-from-wallpaper` | personality's policy | Wallpaper-derived or accent-tinted backgrounds. The last one given wins. |
| `--accent HEX` | from the image | Force the seed accent. |
| `--primary HEX`, `--secondary HEX`, `--tertiary HEX` | from the image | Override one role's source color. |
| `--brightness F` | `0.0` | Lightness shift. |
| `--saturation F` | `1.0` | Saturation multiplier. |
| `--hour H` | `12` | Hour of day used by time-based tinting; an integer, which may be negative or above 23 for testing. |

The JSON includes the absolute image path and a digest under `source`; strip
them before sharing. Results are cached under the XDG cache directory and
reused only while the engine id, format version and input settings match.

## `orient export PALETTE DIRECTORY`

Renders templates into `DIRECTORY`. An existing target file is refused, so use a
fresh directory each time.

| Option | Meaning |
| --- | --- |
| `--templates DIR` | Use your own template tree instead of the bundled one. Subdirectories are kept and the final `.in` is removed. |

Without `--templates`, every bundled target is written (Alacritty, btop, Cava,
Discord, Dunst, Firefox, fish, foot, fuzzel, Ghostty, GTK, Helix, Hyprland,
hyprlock, kitty, Lazygit, mako, Neovim, niri, Obsidian, qt5ct, qt6ct, rofi,
Spotify, Starship, SwayNC, tmux, VS Code, Waybar, WezTerm, Yazi, Zathura, Zed).
The README's "Bundled targets" table says where each file goes.

## `orient render PALETTE TEMPLATE`

Renders one template to stdout. Redirect to a file you mean to replace.

| Option | Meaning |
| --- | --- |
| `--companion FILE` | A second palette for the opposite mode, for templates that use explicit `.dark`/`.light` expressions. |
| `--variables FILE` | JSON object of custom variables for templates that reference `{{custom.*}}`. |

## Template tokens

`#{{primary}}` is the pattern: the token is six hex digits without `#`, so you
write the `#` (or `0x`, or nothing) the application wants. `{{mode}}` expands
to `dark` or `light`. The supported subset of Matugen color expressions, loops,
filters and conditions is listed in the README under "Template syntax";
scripts and includes are not executed.

## Exit status and errors

Unreadable images, bad hex values, an out-of-range `sample` coordinate, an
existing export target and template errors exit non-zero with a one-line
message on stderr.

## Persistent mode

`orient --rpc` and `orient --rpc --serve` keep one process and talk JSON over
stdin and stdout (`crates/orient-cli/src/rpc.rs`) for desktop helpers that
build many palettes. It is not a stable public interface yet.
