# Arch Linux: python-orient-colors

Orient provides both a Python library and a CLI; its Arch package is
`python-orient-colors`, following
[Arch's Python naming/build guidelines](https://wiki.archlinux.org/title/Python_package_guidelines).
PyPI distribution: `orient-colors`; import and command: `orient`.
The recipe is provided here; it is not yet submitted to the AUR or an official
Arch repository. Nacre packagers should depend on `python-orient-colors` and use
an exact version when their tested integration requires one.

The package builds with PEP 517 (`python-build`/Hatchling) and installs with
`python-installer`. Pillow is required; FFmpeg is optional for users sampling
video, and a check dependency for the video tests. No pip command runs in the
package build, and no desktop session is required. `pkgver=0.1.0.dev5` preserves
PEP 440 prerelease ordering. Rebuild on Arch Python minor upgrades.

Install `devtools`, then run from this directory:

```sh
extra-x86_64-build
sudo pacman -U ./python-orient-colors-*.pkg.tar.zst
orient --version
```

Devtools creates/updates a clean stock-Arch chroot and needs sudo for that
isolation. Build users remain unprivileged. CI uses a disposable privileged
container to host the clean chroot; it changes no developer host packages.
See [Arch's chroot instructions](https://wiki.archlinux.org/title/DeveloperWiki:Building_in_a_clean_chroot).

The release sdist is fetched from GitHub and verified by SHA-256. Its exact
contents are used for tests and packaging. On PRs, CI uses the candidate's built
sdist with the same checksum, before the release asset exists. Regenerate
`.SRCINFO` with `makepkg --printsrcinfo > .SRCINFO` after recipe changes.
