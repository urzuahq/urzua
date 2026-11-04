---
Status: Planned
Stable-Id: 01M1YN846AP7AYBX4D0XGNSYR3
Phase: '1'
Track: schema-governance
Blocked-on: —
---
# 45 — Deterministic corpus fingerprint as proof the corpus was read

## What

A deterministic fingerprint of the checked corpus (e.g. a hash over discovered file paths + contents), included in `check`'s JSON output as stronger proof the corpus was actually read than a file count alone.

## Why

RFC-8 already names "corpus fingerprints" in passing as a hypothetical future `computed:`-namespaced field, but nothing designs or builds it. `files_examined` proves a count; it doesn't prove the count refers to the corpus a reader thinks it does -- a fingerprint is a stronger, cheap-to-add claim in the same spirit as the no-silent-no-op principle `rules_executed` already serves.

## Done means

`check`'s JSON carries a corpus fingerprint, a hash over discovered paths and contents, identical for identical input and different when any record changes, with its algorithm and input stated in `SPEC-2`; a test asserts determinism across runs and sensitivity to a one-byte change, and urzua's own `check` output includes it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |