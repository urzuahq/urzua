---
Stable-Id: 01M85B805RSD3J4ZG1C5K2RHEE
Status: Planned
Phase: '1'
Track: release-process
Blocked-on: null
---
# 363 — CI runs the test suite on macOS and smoke-tests each built archive before it is published

## What

Add a macOS job to the `ci` gate that builds and runs the tests on `macos-latest`, and add a smoke step to
`publish-release.yml`'s `build` job that unpacks the archive it just packaged, runs the binary on its own
runner and requires `urzua --version` to print exactly `urzua <tag without v>` before the archive is uploaded.

## Why

Found while reviewing the release flow: `macos-latest` appears in no workflow except `publish-release.yml`'s
release build (line 103), and no `checks.yml` job runs on anything but `ubuntu-latest`. The `aarch64-apple-darwin`
binary is therefore built and attested without ever having been executed, and neither platform's archive is run
before it is attached to a public release. `scripts/install.sh` runs the downloaded binary only after the
release is public, and refuses it if the version differs, so a defective archive is found by a user's install.
The decision on which platforms to test is a cost call (macOS runner minutes), which is why this is a
milestone and not a bug.

## Done means

- A `ci` job runs the Rust tests (or a stated subset) on `macos-latest`, and `ci` needs it, so a failure or skip fails the gate.
- `publish-release.yml`'s `build` job runs the packaged binary and fails when `--version` is not exactly the tag's version, before `attest-build-provenance` and the upload.
- A planted case, observed failing before and passing after, shows the smoke step failing on an archive whose binary reports a different version.
- `docs/specs/SPEC-20-ci-cd-pipeline.md` describes the macOS job and the smoke step.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
