---
Stable-Id: 01M6CXZX537SRJ6YWNB7EGAM86
Status: Done
Phase: '0'
Track: schema-governance
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/diff_gate.rs, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs
---
# 124 — A rule requiring that a diff changing declared source paths also changes a governing record

## What

A rule, declared in config, requiring that a diff which changes declared source paths also changes a
governing record. The paths and the record set are declared; the rule is evaluated against a diff and
reports a source change with no accompanying record change. It depends on a diff-scoped run
(`MILE-123`).

## Why

Replacing a hand-written linter outright (`MILE-102`) means every convention such a linter enforces
needs a declared home, and source-change-requires-record-change is a common one. It is the mechanical half of the
decision-before-implementation idea `MILE-14` leaves undecided: this rule asks only that a record
changed, never that it is the right one, per `AGENTS.md`'s limit on what `check` may claim. `MILE-14`
remains the place to decide whether such a gate should exist by default.

## References

- `MILE-14` -- decide RFC-12's decision-before-implementation gate.
- `MILE-123` -- the diff-scoped run.
- `MILE-102` -- the adopter replacement this serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** Replacing a linter outright needs source-change-requires-record-change to have a declared home; filed for that. | **substantive** |
> | 2026-11-04 | Built as `diff.source-requires-record` (`rules/diff_gate.rs`): with the declared `source_paths` option, one error per changed source path when no governed record changed in the same `--base` diff. "A governed record" is the narrowest reading: a changed `.md` file directly in a configured record type's `dir`, deleted ones included; whether it is the right record is not judged. On a run with no `--base` it reports `not-applicable`, never clean. `Status` unchanged. **Why:** filed work is the adopter-replacement rule `MILE-102` needs; the diff-scoped run it waited on (`MILE-123`) has merged. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #169: opt-in rule `diff.source-requires-record`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-177`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | `Blocked-on` cleared. **Why:** `MILE-123` (the diff-scoped run) is `Done` and this milestone is `Done`; `narrative-field.stale` flagged the leftover. | **substantive** |
