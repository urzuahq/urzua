---
Stable-Id: 01M7FVJXYK9D0AGF2RXG76GTS3
Status: Planned
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
- The install script refuses an archive whose build provenance does not verify, shown by a test with a tampered archive.
- The `cargo install` command on the install page builds the released version, checked in CI.
- A Homebrew formula installs the release, or a decision record says why it is not offered.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
