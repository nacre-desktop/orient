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
recipe. The native Arch workflow can publish this exact Git-pinned package independently
of Python release archives; inspect its source receipt when downloading a build.

PyPI uses `0.1.0.dev6`; Arch uses `0.1.0dev6` so it sorts before final `0.1.0`.
The recipe has not been submitted to the AUR or an official repository. Regenerate
`.SRCINFO` with `makepkg --printsrcinfo > .SRCINFO` after recipe changes.

## Independent native releases

`Native Arch release` (`native-arch.yml`) builds the committed, fixed Git recipe
in a clean stock-Arch chroot, then installs **only** the native package into a
separate runtime image. It verifies palette generation without Python or a
compiler. The build retains archives, recipe, logs, checksums and a source receipt.
Dispatch with `publish=true` to sign these same passing artifacts and create an
immutable `arch-PKGVER-PKGREL` GitHub prerelease. A rerun cannot overwrite an
existing release. The native Arch release is independent of Python/PyPI publishing;
no PyPI environment approval is bypassed.

The public signing key is in `packaging/keys`; only an expiring signing subkey is
available to the publication job through `ARCH_REPO_SIGNING_KEY`. Recovery and
certification keys stay outside GitHub and the repository. Signing happens only
on main, after package and runtime checks succeed, in a separate job.

Nacre consumes `orient` as a normal package dependency and pins the engine it has
tested. Publishing a new Orient engine does not silently change Nacre's lock;
Nacre must validate and promote the matching dependency before its repository
updates that engine. This keeps independent releases from breaking existing
Nacre installations.
