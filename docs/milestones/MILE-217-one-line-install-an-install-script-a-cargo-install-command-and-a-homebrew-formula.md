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
- The install script refuses an archive whose build provenance does not verify, shown by a test with a tampered archive. — evidence: code:scripts/install.sh@sha256:beaaf0ac9f236ab8b3c586fa602d7f0780339e3ef1772f9b307a22ed622b12f2, test:scripts/install-test.sh@sha256:deea59418d7d6e3c4ba773dd2b3306221dfdf58c3a54d39c443951bc14073891
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
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `scripts/install.sh` 7df133889e5e -> beaaf0ac9f23, `scripts/install-test.sh` c4d07691942a -> 7775c1f4517b. Reason: install.sh wraps its body in a main function called on the last line, adds download timeouts, refuses a musl system and an empty --sha256, and stages with mktemp; install-test.sh drops a stray -- that made three checks vacuous; the tampered-archive refusal the claim rests on is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-23 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `scripts/install-test.sh` 7775c1f4517b -> deea59418d7d. Reason: scripts/install-test.sh gained a check that fails when a documented install command pipes a download into a shell; no installer behaviour or existing scenario changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
