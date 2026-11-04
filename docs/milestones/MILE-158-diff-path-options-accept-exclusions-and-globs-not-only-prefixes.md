---
Stable-Id: 01M6JKNW1YB9CAQ7EMKFQE6HMW
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/diff_gate.rs, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs
---
# 158 — Diff path options accept exclusions and globs, not only prefixes

## What

Let the `diff.*` path options take globs and exclusions so a set such as every source file except test files can be stated.

## Why

The options are prefixes only, so when tests sit beside the code in `*.test.*` files there is no way to say everything except tests. The rule `diff` is meant to answer "did code change without a governing record"; a test-only change must not trip it, and the configuration cannot say so. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A path option may declare an exclusion or glob; a diff touching only an excluded file is not a trigger; prefix-only configurations behave unchanged; fixtures cover tests beside code; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Implemented: the diff rules' path options accept globs and `exclude_paths` also narrows `diff.source-requires-record`, leaving excluded paths eligible and unexamined. This repository excludes `rust/crates/*/tests` from `source_paths`, and `diff_gate_integration.rs` runs the shipped config on a test-only and a source change. `Status` stays `Planned`. **Why:** the Done-when contract; status is a human decision. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #200: the diff rules' path options accept globs and `exclude_paths`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-177`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
