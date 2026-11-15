---
Stable-Id: 01M7FVSWASJMN906V7ZDWEN9S4
Status: Planned
Phase: '2'
Track: integrations
Blocked-on: null
---
# 222 — A CI recipe and an actors source that do not depend on GitHub

## What

Document and test running the gate on a CI system other than GitHub Actions, and let the actors file and `refs refresh` be produced from another forge.

## Why

the audit of 2026-11-15 found that the gate action, `refs refresh`, the actors file producer and the identity lookup all assume GitHub. The engine reads a caller-supplied actors file, so the gap is the tooling around it.

## Done means

- The CI reference page has a worked recipe for one non-GitHub CI system, and a test runs the same commands.
- A documented script or command produces the actors file from one non-GitHub forge's merge request data.
- `refs refresh` reads issue and pull request references from a configured forge, or reports that the configured forge is unsupported and how to supply the file by hand.
- `urzua new`, `waive` and the apply modes work with no `gh` on `PATH`, shown by tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
