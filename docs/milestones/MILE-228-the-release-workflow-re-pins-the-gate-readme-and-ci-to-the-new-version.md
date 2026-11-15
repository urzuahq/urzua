---
Stable-Id: 01M7FVV703VGQP17JR75MAR9AR
Status: Planned
Phase: '1'
Track: release-process
Blocked-on: null
---
# 228 — The release workflow re-pins the gate, README and CI to the new version

## What

Make the version pins that follow a release (the gate action's default, the README and CI examples, the pinned-release CI job) part of the release, so they are not a hand-run follow-up.

## Why

`MILE-212` re-pinned to one release by hand. the audit of 2026-11-15 found that release 0.4.3 has no record for its re-pin, and that the install and CI pages still named a release three versions old.

## Done means

- After a release is published, a pull request that bumps every version pin and checksum is opened automatically, or the release pull request itself carries them.
- A test fails when a documentation page names a release tag older than the latest tag.
- The pins in this repository name the latest release.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
