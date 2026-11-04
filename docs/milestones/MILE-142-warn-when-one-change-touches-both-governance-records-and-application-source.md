---
Stable-Id: 01M6HZ0PHX2W3QNCF6PKCWYNSA
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
---
# 142 — Warn when one change touches both governance records and application source

## What

A warning when a single change touches both governance records and application source. The changed paths are classified by two declared path sets (record directories, application source) and a diff that hits both warns. It runs only where a base ref is available and fails loudly, rather than passing silently, when the requested diff cannot be computed.

## Why

The adopter keeps a warn-only check that flags a pull request mixing a record change with source change, as a prompt to split it. It is the softest of its diff checks. It depends on a diff scope (`MILE-123`) and is the inverse of `MILE-124`, which requires a record change alongside a source change.

## Done means

A diff touching both path sets produces a warning naming one path from each; a diff touching only one produces none; with no base ref the rule reports that it did not run rather than reporting clean; test files are excluded from the application set as the config declares.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter warns when one diff mixes governance records with application source, and no urzua rule covers the mixed case. | **substantive** |
> | 2026-11-04 | Built as `diff.source-and-records-mixed` (`rules/diff_gate.rs`): with the declared `application_paths` and optional `exclude_paths` options, one finding naming the first changed record and the first changed application source path when a `--base` diff touches both; the record set is the configured record type directories, and a governed record is never application source. Reports `not-applicable` on a run with no `--base`. `Status` unchanged. **Why:** the diff-scoped run it depends on (`MILE-123`) has merged, so the check could be built. | **substantive** |
