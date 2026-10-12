# Orient 0.1.0.dev5 release progress

## Done

- Isolated release branch from current main; palette/engine behavior remains unchanged.
- PyPI and TestPyPI JSON APIs returned HTTP 404 for orient-colors (2026-10-12).
  No published project exists; pending registration/upload must confirm acceptance.
- Arch package name: python-orient-colors; distribution/import/CLI names:
  orient-colors / orient / orient.
- GitHub environments created: testpypi accepts version tags; pypi accepts version
  tags and requires Siverteh approval, with administrator bypass disabled.

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
