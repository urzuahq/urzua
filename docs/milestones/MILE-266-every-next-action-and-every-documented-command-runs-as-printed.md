---
Stable-Id: 01M7FW5X1S92KDD4FAABC33847
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 266 — Every next action and every documented command runs as printed

## What

Make each `next_action` and each command in the documentation run unchanged: include `--actor` where an apply needs it, suggest staging an untracked record, and suggest a re-baseline for drift.

## Why

A walkthrough of 2026-11-15 followed only the user documentation with the released binary. `fix --apply` as printed by `next_action` and by three documentation pages fails for want of `--actor`; after `urzua new`, `check` is ok having examined no file; and for a changed pinned file the suggested action wrote a drift state and turned the check green instead of pointing at the re-baseline.

## Done means

- A test runs the `next_action` of every finding variant in a fixture and each command exits without a usage error.
- A test extracts the commands in the reference pages and runs them against a fixture.
- `check` on a repository whose only record is untracked says so and suggests staging it.
- The next action for a changed pinned file is the re-baseline listing.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
