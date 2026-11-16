---
Stable-Id: 01M7FVJXYK9D0AGF2RXG76GTS3
Status: InProgress
Phase: '1'
Track: release-process
Blocked-on: null
---
# 217 — One-line install: an install script, a cargo install command and a Homebrew formula

## What

Give a new user one command that installs the latest release: an install script that downloads the archive for the platform and checks its build provenance, a documented `cargo install` command that works, and a Homebrew formula.

## Why

the audit of 2026-11-15 found that day one needs a manual archive download or a Rust toolchain. The install page says Intel Macs build with `cargo install` but gives no command, and the workspace is not published, so no such command works today.

## Done means

- The install page gives one command that installs the latest release on Linux x86_64 and Apple Silicon, and a CI job runs that command and then `urzua --version`.
- The install script refuses an archive whose build provenance does not verify, shown by a test with a tampered archive. — evidence: code:scripts/install.sh@sha256:7df133889e5e2e165440f84829d29f2482e4fcd5fe2440f84b95fed3f11db595, test:scripts/install-test.sh@sha256:c4d07691942ae820a4b75ac9fadefa4a6a2d4bb2dac6d33de7ca18d97448826f
- The `cargo install` command on the install page builds the released version, checked in CI.
- A Homebrew formula installs the release, or a decision record says why it is not offered.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-16 | Publishing to the crates registry is decided: yes, alongside the install script. **Why:** Asked on 2026-11-16 whether to publish there as well as shipping the script, the maintainer answered "Publish to crates.io too". It needs a registry token from the maintainer as a repository secret. | **substantive** |
> | 2026-11-16 | Delivered in part: `scripts/install.sh` and its tests `scripts/install-test.sh` (`make install-script-test`), with the install page leading with the one-line install and the sha256 of the `v0.4.4` archives. The item for refusing an archive whose provenance does not verify now cites that code and test. **Remains:** the first item names a CI job that runs the install command and then `urzua --version`, which needs a job in `.github/workflows/ci.yml` (a protected path) and is not cited; `cargo install` and the Homebrew formula are not started, and the crates-registry publish decided above needs a registry token as a repository secret, crate metadata and a publish step in `publish-release.yml`. `Status` is left to `urzua fix`. **Why:** this change builds the script and its tests but cannot add the CI job or touch the registry. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (1 of 4 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `InProgress`. | **structural** |
