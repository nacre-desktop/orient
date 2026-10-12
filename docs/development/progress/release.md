# Orient 0.1.0.dev5 release progress

## Done

- Released immutable tag `v0.1.0-dev.5` at
  `6ee17843647250b45f75dc229ab02688a6450bc2`.
- PyPI/TestPyPI initially returned 404 for `orient-colors` (2026-10-12).
  The maintainer configured both pending publishers; TestPyPI accepted the name
  and created the project through Trusted Publishing. No stored API tokens.
- [TestPyPI 0.1.0.dev5](https://test.pypi.org/project/orient-colors/0.1.0.dev5/)
  published. Fresh index installs and all CLI/library/template smoke checks passed
  on Python 3.13/3.14 in CI, and Python 3.14.7 locally.
- [Release workflow 38248389874](https://github.com/nacre-desktop/orient/actions/runs/38248389874)
  completed build, wheel smoke, Arch clean-chroot build, TestPyPI publication and
  TestPyPI installation jobs. Production `pypi` job is waiting for Siverteh review;
  tag-only environment policy and administrator bypass disabled were verified.
- [GitHub release](https://github.com/nacre-desktop/orient/releases/tag/v0.1.0-dev.5)
  includes the tested sdist, wheel and clean-chroot Arch package.
- Arch package: **python-orient-colors 0.1.0dev5-1**. Python package:
  **orient-colors 0.1.0.dev5**; import/CLI: **orient**. Arch's undotted dev suffix
  was verified with vercmp to sort before final 0.1.0. Recipe/.SRCINFO in
  `packaging/arch/`; final name and versions shared with Nacre packager via PR.
- Native final stock-Arch clean-chroot build also passed all 57 tests and the
  installed-wheel smoke. Namcap had no errors; its entry-point module lookup
  warning remains documented in validation evidence.
- [Candidate CI 38247953341](https://github.com/nacre-desktop/orient/actions/runs/38247953341)
  passed all seven checks: three Python/Pillow engine combinations, distribution
  build, two clean wheel environments and Arch. Ruff lint/format, strict Twine,
  actionlint and local 57-test suite passed. All 13 bundled templates verified.
- Added README CLI/library usage, supported API, JSON formatVersion 1, 0.x
  versioning and release documentation. Engine/palette implementation unchanged;
  runtime source diff is only the package-version declaration.
- [Nacre bump PR #6](https://github.com/nacre-desktop/nacre/pull/6) merged as
  `6b35c4e45b11aa50ace5d5784a2af1edcc72cb3f`. It updates gitlink,
  lock, exact Python dependency and one startup fingerprint line. All colors and
  presets unchanged. Rebased local 616 tests and native/source checks passed;
  [CI 38248577511](https://github.com/nacre-desktop/nacre/actions/runs/38248577511)
  passed including JavaScript formatting.

## Artifact identity

GitHub and both indexes reuse the same tested Python artifacts. Local and Ubuntu
compression differed despite identical archive contents, so Arch pins the
canonical CI sdist rather than a locally rebuilt archive.

- sdist SHA-256: `58ec261d8c0d5888796ff46775f243dbfa6a11253ea9fa4f97bc65c57aba5b8b`
- wheel SHA-256: `f45f7fa03e554e0514f967d394497cfcc72d0f6cfbd9f6af64c479c3c0991553`
- TestPyPI JSON API digests matched both artifacts independently.
- Progress notes and Arch packaging are excluded from the sdist, avoiding a
  self-referencing checksum. Changes to included source/docs require a new build.

## Next

Maintainer may review the release artifacts and approve production PyPI when
ready. Release implementation/checkpoint review is tracked through
[Orient PR #2](https://github.com/nacre-desktop/orient/pull/2); the release tag
remains at the tested immutable revision above. No desktop deployment is part of
this release task.

## Open questions / maintainer decisions

- Production PyPI publication intentionally remains pending approval at the
  linked release workflow. This task does not approve it.
- The Arch recipe/package is available from this repository/release; AUR or
  official Arch repository submission is a separate distribution decision.
