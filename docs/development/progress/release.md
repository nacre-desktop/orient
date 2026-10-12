# Orient 0.1.0.dev5 release progress

## Done

- Isolated release branch from current main; palette/engine behavior remains unchanged.
- PyPI and TestPyPI JSON APIs returned HTTP 404 for orient-colors (2026-10-12).
  No published project exists; pending registration/upload must confirm acceptance.
- Arch package name: python-orient-colors (Arch 0.1.0dev5; Python 0.1.0.dev5); distribution/import/CLI names:
  orient-colors / orient / orient.
- GitHub environments created: testpypi accepts version tags; pypi accepts version
  tags and requires Siverteh approval, with administrator bypass disabled.

- Added tokenless tag-driven publishing and reusable distribution checks, including
  clean wheel installs on Python 3.13/3.14 and Arch devtools clean-chroot CI.
- Added CLI/library/API/JSON/versioning/release docs and Arch recipe.
- Local Python 3.14.7: 57 tests, Ruff lint/format, sdist-to-wheel build, strict
  Twine metadata checks and installed-wheel CLI/library smoke passed (13 templates).

## Next

- Build/check distributions, installed wheel CLI smoke on Python 3.13 and 3.14.
- Add user/API/JSON/versioning/release documentation and Arch recipe.
- Validate Arch recipe in a clean chroot and coordinate its name/artifact with Nacre packaging.
- Open one Orient PR, release tag v0.1.0-dev.5, publish and install from TestPyPI.
- Leave production PyPI job waiting for maintainer approval.
- Open one minimal Nacre PR updating the Orient submodule and its matching pin.

## Open questions

- Maintainer must create PyPI/TestPyPI accounts and configure pending Trusted
  Publishers: nacre-desktop/orient, publish.yml, respective pypi/testpypi environments.
- Local sudo/devtools unavailable; use disposable CI clean-chroot builder.

- Checked Arch ordering with vercmp: 0.1.0dev5 < 0.1.0, unlike dotted dev spelling.
