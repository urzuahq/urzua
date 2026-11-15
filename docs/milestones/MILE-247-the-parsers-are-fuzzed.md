---
Stable-Id: 01M7FVZRQVDJTPKNY9NZND93ZB
Status: Planned
Phase: '1'
Track: code-health
Blocked-on: null
---
# 247 — The parsers are fuzzed

## What

Add fuzz targets for the header parser, the config loader, the locator parser and the harness JSON parser, and run them on a schedule.

## Why

the audit of 2026-11-15 found deep tests but no fuzzing; these four parsers read input an agent or contributor controls.

## Done means

- Four fuzz targets exist and a scheduled CI job runs each for a stated time.
- Every crash found is fixed with a regression test, and the corpus is kept in the repository.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
