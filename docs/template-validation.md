# Template validation

`tests/test_adoption_templates.py` renders every new template against the frozen
public gallery's six Natural palettes and the default palette, each in dark and
light mode: **14 palettes per template**. These are sample inputs, not regenerated
engine golden data. The separate installed-package and native golden suites
continue to compare the original engine outputs exactly.

The Rust CLI embeds templates; rebuild it after changing a template. The native
export test compares every exported file byte-for-byte with the Python export
for all 14 samples, including the existing templates. Export regression tests
check exact file sets, and the installed-wheel smoke test checks that all 33
bundled files reached the distribution.

## Run locally

Use the project's isolated Python development environment (Python 3.13+):

```sh
python -m pip install . -r tests/template-parsers.txt
nice -n 10 cargo build --release --locked -j2 -p orient-cli
nice -n 10 python -m unittest discover -s tests -p test_adoption_templates.py -q
```

The full suite requires fish and tmux for their application parser tests. Install
them through your normal package manager before running the command above. CI installs both and the pinned Python test parsers.
The parser packages are test-only; the binary has no runtime dependency on them.
A missing required parser fails the tests instead of silently skipping coverage.

## Coverage

| Format / targets | Real parser | Additional checks |
| --- | --- | --- |
| TOML: Helix, Starship, Yazi | Python `tomllib` | Theme sections and paired foreground/background roles. |
| INI: fuzzel, Dunst, Qt, Cava | Python `configparser` | fuzzel RGBA notation, urgency sections, Qt's 21 ordered ARGB roles, gradient count. |
| mako | `configparser` with a synthetic `[global]` header | Native headerless key/value preamble plus criteria sections; no interpolation or inline hash comments. |
| JSON: Zed | Python `json` | Required theme-family fields, appearance, player colors, 16 ANSI keys. |
| CSS: Waybar, SwayNC | `tinycss2` stylesheet and declaration parsers | Reject parse errors, malformed declarations, non-color properties and invalid hex values. GTK widget selectors are documented in template headers. |
| KDL: niri | `kdl-py` | Layout/ring/border nodes and active/inactive/urgent color arguments. Uses syntax common to KDL 1 and 2. |
| YAML: Lazygit | PyYAML safe loader | `gui.theme` and default foreground. |
| Lua: WezTerm | `luaparser` | Complete returned color table syntax. |
| Hyprlang: Hyprlock | Tree-sitter Hyprlang grammar in `tree-sitter-language-pack` | Reject error and missing syntax nodes. |
| fish | `fish --no-config --no-execute` | Parse without executing or reading user configuration. |
| tmux | `tmux source-file` on an isolated temporary socket | Real option validation and resulting status colors; start from `/dev/null`, remove the temporary server afterward. |
| Ghostty | **Skipped: no standalone Ghostty format parser available in this harness** | Rendering, all 16 indexed ANSI values, and documented color options still checked. |
| zathurarc | **Skipped: no headless zathurarc application parser available in this harness** | Rendering and 18 documented `set` color assignments still checked. |

The two format skips are named tests with visible reasons. Rendering them is
never skipped. Their app-specific formats are not mislabeled as TOML/INI merely
because their color lines look similar. A full Ghostty or Zathura session is not
started on the developer's desktop.

Syntax parsing does not replace application schema or visual/session testing.
The Qt list order follows QPalette's serialized role order; qt6ct's loader fills
Accent from Highlight for a 21-entry scheme on Qt 6.6+. Zed keeps strict JSON
because Orient's export preflight parses `.json` output: its leading `_comment`
field carries the header documentation link instead of an invalid `//` comment.
No application is configured or reloaded by the tests.
