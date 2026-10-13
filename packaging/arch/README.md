# Arch Linux packages

The split recipe provides `orient` (standalone Rust command, no Python runtime)
and `python-orient-colors` (Python library plus `python -m orient`, with Pillow).
Install both for callers needing both interfaces. Nacre's native engine
dependency is `orient`; it no longer needs the Python Orient library. Wheels no longer install a competing `orient` console script.

Both packages support x86_64 and aarch64. Builds use Rust, LCMS2 and
PEP 517/maturin; Python installation uses `python-installer`. FFmpeg is optional
at runtime and required for video tests. No pip command runs in the package
build and no desktop session is required.

```sh
extra-x86_64-build
sudo pacman -U ./orient-*.pkg.tar.zst ./python-orient-colors-*.pkg.tar.zst
orient --version
python -m orient --version
```

Devtools needs sudo to create its stock-Arch clean chroot; CI hosts it in a
disposable privileged container. The development recipe uses a fixed tested Git revision, because the published
dev5 archive predates the Rust engine. Development CI separately verifies its
current sdist artifact digest and substitutes that snapshot into a temporary
recipe. A future maintainer release must prepare a release-archive source pin;
this migration does not upload packages, change versions or create tags.

PyPI uses `0.1.0.dev5`; Arch uses `0.1.0dev5` so it sorts before final `0.1.0`.
The recipe has not been submitted to the AUR or an official repository. Regenerate
`.SRCINFO` with `makepkg --printsrcinfo > .SRCINFO` after recipe changes.
