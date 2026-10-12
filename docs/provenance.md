# Implementation origin

The desktop integration that preceded Orient used the GPL-licensed
[Caelestia CLI](https://github.com/caelestia-dots/cli). That historical implementation
was replaced in October 2026 before this standalone repository was created.

The replacement record covers the image analysis, color math, palette role policy,
cache/storage and command-line modules. The inherited engine implementation and
seed palette data were removed. Replacement modules were written against a behavior
specification, observed CLI contracts and public API/math documentation. Upstream
implementation bodies were not consulted during that replacement. Earlier exposure
to the integration was recorded; this is an implementation-origin record, not a
legal clean-room certification.

The source audit compared the maintained implementation against pinned historical
upstream snapshots, using exact bytes and bounded normalized-block comparisons.
Known historical imports served as positive controls. The recorded CLI inputs were
[`b698f7a`](https://github.com/caelestia-dots/cli/tree/b698f7a38543df7139c6f8ae3c748cb07c69769e)
and [`f16ec8f`](https://github.com/caelestia-dots/cli/tree/f16ec8f54e3dd60a47a73aaab75d0ad64e5025f6).
The historical CLI control included 17 modules: seven exact matches and 15 with
qualifying normalized blocks. The maintained engine review found no inherited
implementation/import. Matches were reviewed rather
than treating a similarity score as proof of authorship. Behavioral, palette,
contrast, image/frame and CLI tests checked the replacement independently.

Current Orient imports no Caelestia or Material You implementation. Its source is
MIT-licensed. Historical GPL versions remain governed by their historical licenses;
the MIT license does not relicense code owned by other projects.

OKLab math follows [Björn Ottosson's public-domain definition](https://bottosson.github.io/posts/oklab/).
Pillow is an independently installed dependency and retains its own license.
Palette JSON, template compatibility and application-role names are interfaces,
not imports of another engine's implementation.
