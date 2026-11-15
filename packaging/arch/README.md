# Arch Linux packages

The recipe provides `orient`, the standalone Rust command with no Python runtime.
It is Nacre's native engine dependency. The Python library (`python -m orient`)
is published on PyPI. Its parity with Pillow is pinned to Pillow 11–12 and tested
in Orient's CI, so it is not shipped as an Arch package: Arch now carries Pillow 13.

The package supports x86_64 and aarch64. The build uses Rust and LCMS2, and builds
the wheel with PEP 517/maturin only to check the native engine against its frozen
goldens. FFmpeg is optional at runtime and required for video tests. No pip command
runs in the package build and no desktop session is required.

```sh
extra-x86_64-build
sudo pacman -U ./orient-*.pkg.tar.zst
orient --version
```

Devtools needs sudo to create its stock-Arch clean chroot; CI hosts it in a
disposable privileged container. The development recipe uses a fixed tested Git revision, because the published
dev5 archive predates the Rust engine. Development CI separately verifies its
current sdist artifact digest and substitutes that snapshot into a temporary
recipe. The native Arch workflow can publish this exact Git-pinned package independently
of Python release archives; inspect its source receipt when downloading a build.

PyPI uses `0.1.0.dev7`; Arch uses `0.1.0dev7` so it sorts before final `0.1.0`.
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
The tag goes on the tested recipe commit (`recipeRevision`), not the engine commit
(`sourceRevision`, still in `release.json`): GITHUB_TOKEN cannot create a tag on a
commit whose `.github/workflows` files differ from main's (HTTP 403).

The public signing key is in `packaging/keys`; only an expiring signing subkey is
available to the publication job through `ARCH_REPO_SIGNING_KEY`, a secret of the
`arch-signing` environment (main only; the owner approves each run). Recovery and
certification keys stay outside GitHub and the repository. Signing happens only
on main, after package and runtime checks succeed, in a separate job.

Nacre consumes `orient` as a normal package dependency and pins the engine it has
tested. Publishing a new Orient engine does not silently change Nacre's lock;
Nacre must validate and promote the matching dependency before its repository
updates that engine. This keeps independent releases from breaking existing
Nacre installations.
