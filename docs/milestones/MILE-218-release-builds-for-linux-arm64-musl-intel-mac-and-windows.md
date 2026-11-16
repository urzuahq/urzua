---
Stable-Id: 01M7FVR9RA8ZEQPFQS2WD0PNAD
Status: Planned
Phase: '1'
Track: release-process
Blocked-on: null
---
# 218 — Release builds for Linux arm64, musl, Intel Mac and Windows

## What

Publish release archives for `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `x86_64-apple-darwin` and `x86_64-pc-windows-msvc`, each with build provenance, and make the agent harness hooks run on Windows or say plainly that they do not.

## Why

the audit of 2026-11-15 found releases for two targets only. Arm CI runners, Alpine containers, Intel Macs and Windows developers have to compile, and the gate action cannot install a release there.

## Done means

- A release carries an archive and attestation for each of the four added targets.
- The gate action installs the matching archive on an arm64 Linux runner and in an Alpine container, shown by its tests.
- `urzua check` passes its own acceptance corpus on Windows in CI, including a checkout with `core.symlinks=false`.
- The install page lists every published target and what the agent harness needs on each.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-16 | Decided: Linux arm64 and musl first, Windows later; Intel Mac stays dropped. **Why:** the maintainer decided this on 2026-11-16 when asked which of the platforms to build first. | **substantive** |
