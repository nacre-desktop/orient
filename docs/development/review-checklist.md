# Pull request review checklist

Apply this checklist to every Orient PR, whether authored or reviewed by a human
or an AI. Read the entire diff, including generated assets and workflow changes.
Mark non-applicable checks with a reason in the review. An AI review assists the
maintainer; it does not supply permission or prove that a real-session test ran.

## Every PR

- [ ] The problem, resulting behavior and scope are clear. Inspect all changed
      files for broad unrelated edits, hidden behavior changes or unexplained
      generated output. Split unrelated changes before merging.
- [ ] The branch is current with `main` and applicable local checks pass at the
      revision being merged. Read lint/format/test results and relevant logs, including
      skips; a green summary does not replace tests of the affected behavior.
- [ ] New code has meaningful tests; reproduction and regression evidence cover
      the changed path. Confirm claimed real-session/environment checks and their
      versions. Docs-only work checks links, forms and policy accuracy instead.
- [ ] Affected docs and public contracts are updated. UI changes include readable
      before/after screenshots using public demo data and respect translation
      conventions; flag unexplained hard-coded visible strings.
- [ ] The author can explain and has tested the work, including AI-assisted output.
      Separate measured results, reported observations and assumptions.
- [ ] Licenses and provenance permit contribution/redistribution; check third-party
      notices, image licenses and optional DCO trailers without requiring sign-off.
- [ ] No personal profiles, private settings, account state, memories, credentials
      or private images are committed or shown in logs/screenshots. Redact secrets
      and report suspected vulnerabilities through [SECURITY.md](../../SECURITY.md).

## Changes needing extra scrutiny

- [ ] Image/animation parsing handles malformed or oversized inputs, process
      timeouts and resource limits; FFmpeg arguments do not invoke a shell or
      accept unintended command options from filenames.
- [ ] Template rendering does not introduce arbitrary scripts/includes. Export
      paths, cache permissions, atomic writes and cleanup remain scoped to the
      caller's intended directories; query/preview does not apply desktop state.
- [ ] Palette schema/formatVersion, cache fingerprints and public API compatibility
      are considered; color/contrast results have comparable input and options.
- [ ] Packaging and build hooks are reviewed for execution, downloads and wheel/
      sdist contents. Orient remains independent of Nacre and a running compositor.
- [ ] Any proposed installer/sudo path or AI/credential feature receives explicit
      maintainer scrutiny; these are not required by the current standalone engine.
- [ ] CI/workflow changes use minimum token permissions and trusted pinned actions.
      Review triggers, shell quoting, caches/artifacts and secret access. Never run
      untrusted fork code with write tokens/secrets via `pull_request_target` or an
      equivalent privileged path. Inspect code before approving a first-time
      contributor's workflow; that approval is separate from PR approval.
- [ ] New dependencies and downloads explain necessity, alternatives, source,
      version pin/integrity, license and execution/install hooks. Review lockfile
      and submodule changes; no unexplained network/sudo/dependency additions.
- [ ] New network calls explain destination, data sent, user consent, timeouts,
      retries and offline behavior. No hidden telemetry or transmission of private
      data; avoid secrets in URLs, process arguments or logs.
- [ ] Credential/auth changes use the existing OS secret store and permission
      boundaries; no plaintext credentials, copied authentication or weakened
      validation. Check exposure in errors, environment variables and subprocesses.
- [ ] Binaries/minified/encoded code have a stated source, license, build procedure
      and verification evidence. Inspect the source diff; reject unexplained opaque
      payloads rather than trusting generated output or an AI summary.

## Before merge

- [ ] Resolve outstanding review findings and validate the current revision;
      check CI after a direct push during experimental development. Sensitive paths
      have maintainer scrutiny, including anything newly run with sudo; add explicit CODEOWNERS patterns when introducing such paths.
- [ ] Check deployment/recovery impact and rollback where applicable. Do not
      merge based on an unverified claim of live testing or bypass a failing check.
- [ ] Verify repository settings against the
      [community-settings guide](community-settings.md) when changing CI job names,
      workflows or community policy; the experimental direct-push policy and enabled
      CI must match the documented settings.
