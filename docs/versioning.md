# Versioning during 0.x

Orient is a development preview. The Python distribution uses PEP 440 versions
such as `0.1.0.dev5`; source `VERSION`, Git tags and GitHub release titles use
`0.1.0-dev.5` / `v0.1.0-dev.5`. These identify the same release. Development
versions require `pip install --pre orient-colors` or an exact version pin.
Once a stable version is approved on PyPI, use `pip install orient-colors`.

During 0.x, the [documented API](api.md) is the supported integration surface.
Stable 0.x patch releases preserve its signatures and core palette fields;
incompatible API changes require a minor version and migration notes. Development
releases may change between iterations; pin an exact version for reproducible
integrations and read the changelog. Undocumented helpers have no compatibility
guarantee. Engine/color tuning may change generated values; callers must use
`ENGINE_ID` to invalidate caches rather than expect fixed colors across releases.

Palette JSON has a separate integer `formatVersion`, currently 1. Additive roles
and metadata preserve that format; incompatible core schema changes increment it.
Package version bumps do not reset the JSON format. Existing released versions
and tags are immutable: corrected artifacts get a new version.

Every release builds the sdist and wheel, tests Python 3.13/3.14 installations,
and builds the Arch package. TestPyPI publishing and a fresh TestPyPI install
precede production PyPI. The maintainer approves production through the protected
GitHub `pypi` environment. Development releases remain prereleases even after
approval. See the [release runbook](development/releasing.md).
