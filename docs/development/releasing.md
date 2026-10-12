# Releasing Orient

## One-time setup

Create accounts on [PyPI](https://pypi.org/account/register/) and
[TestPyPI](https://test.pypi.org/account/register/), verify email, and enable 2FA.
On each account's Publishing page, add a pending GitHub Trusted Publisher:
project `orient-colors`, owner `nacre-desktop`, repository `orient`, workflow
`publish.yml`; environment `testpypi` on TestPyPI and `pypi` on PyPI.
[PyPI's pending publisher guide](https://docs.pypi.org/trusted-publishers/creating-a-project-through-oidc/)
explains first-upload project creation; pending publishers do not reserve names.
No long-lived API token is configured.

In GitHub Settings → Environments, restrict both environments to `v*` **tags**.
Require the maintainer as a reviewer for `pypi` and disable administrator bypass.
A sole maintainer may approve their own run; required review still pauses every
production upload. These repository settings are essential: YAML environment
names alone do not enforce approval. Never approve production through automation.

## Prepare and validate

1. Rebase the task branch on `main`. Update `pyproject.toml`,
   `src/orient/__init__.py`, `VERSION`, the README preview link and CHANGELOG.
2. Run the CONTRIBUTING lint/tests and `python -m build` (wheel built from sdist),
   then `python -m twine check --strict dist/*`.
3. Update the Arch `pkgver` (e.g. `0.1.0dev5`), `_pyver` (`0.1.0.dev5`), source SHA-256 and `.SRCINFO` from that sdist. The
   sdist excludes Arch packaging and the progress note to avoid a self-referencing
   checksum. Any included source/doc change requires rebuilding and a new checksum.
4. CI installs the built wheel in fresh Python 3.13/3.14 venvs, runs
   `scripts/smoke_wheel.py` with the public coast-day fixture, and builds Arch in a
   clean devtools chroot. Arch PR validation substitutes this run's local sdist
   for its eventual GitHub release URL; it still enforces the committed checksum.
5. Open one PR with validation evidence and the Arch name `python-orient-colors`
   in its packaging handoff. Merge only once required checks pass.

## Publish

Create the matching immutable version tag, e.g. `v0.1.0-dev.5`, on the tested
release revision and push it. `publish.yml` validates tag/version equality, builds
and checks artifacts, publishes to TestPyPI with short-lived OIDC credentials,
and installs the exact version from TestPyPI on Python 3.13/3.14. Dependencies
come separately from real PyPI; no mixed-index resolution is used.

The production upload waits for `pypi` environment approval and uses the **same**
built artifacts. Leave this job pending until the maintainer chooses to approve.
If a publish fails after an index upload, retry the failed jobs; `skip-existing`
allows immutable existing files to be skipped. Never overwrite an existing tag or
version. A TestPyPI setup failure can be retried after the pending publisher is
configured. A manual dispatch is permitted only with an existing version tag.

`publish.yml` also calls `release.yml` to attach the **same tested artifacts**
as a GitHub release (prerelease for development tags). The Arch source URL points
to that sdist and its digest is verified against the recipe. Do not rebuild
release archives on another toolchain: compressed bytes can differ even when
contents match. For manual recovery, dispatch `release.yml` with the tag and
original publishing run ID; it downloads and verifies existing artifacts.

## Fresh TestPyPI installation

```sh
python3 -m venv /tmp/orient-test
/tmp/orient-test/bin/python -m pip install 'pillow>=11,<13'
/tmp/orient-test/bin/python -m pip install --no-cache-dir --no-deps \
  --index-url https://test.pypi.org/simple/ orient-colors==0.1.0.dev5
/tmp/orient-test/bin/orient --version
```

Finally open a separate minimal Nacre PR updating only its Orient submodule and
matching explicit pin to the release tag. Share the Arch artifact/name with its
packager. Update [release progress](progress/release.md) with run/PR links,
results and any remaining maintainer actions.
