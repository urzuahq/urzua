---
Stable-Id: 01M7FVYT64D6GPPZYV3JD6PF0W
Status: Planned
Phase: '1'
Track: code-health
Blocked-on: null
---
# 243 — Workspace lints and dependency hygiene

## What

Declare lints once for the workspace with a chosen set of pedantic lints, move shared dependencies to the workspace table, remove the unused error-handling dependency, and decide the fate of the stub interchange crate.

## Why

the audit of 2026-11-15 found no workspace lint table, 1,215 pedantic warnings (374 of them missing `must_use`), `sha2` declared twice, a declared dependency nothing uses, and a 42-line crate nothing depends on.

## Done means

- The workspace manifest carries a lints table every crate inherits, and CI fails on a violation.
- No dependency is declared that no crate uses, checked in CI.
- The stub interchange crate is removed, or a milestone that needs it is named in its manifest.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
