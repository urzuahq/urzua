---
Stable-Id: 01M6D5VKCXX89AM4E8HTJVFAKA
Status: Fixed
Found-in: "A /code-review pass, round 30"
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_errors_when_no_rule_is_enabled@sha256:7988a4173c1393a32dabfaf944f28252704bbd500a06faacbdb33f7497a05cb7
---
# 174 — doctor reports healthy for a config that enables no rule where check exits 2 not-run

## What was wrong

`urzua doctor` reported `warn` and exited 0 for a config declaring record types but no enabled rule
(no `rules:` key, `rules: {}`, or every rule `off`), while `urzua check` on the same config exits 2
not-run with every rule `not-enabled`. `record_types: {}` was flagged; the analogous empty-rules state was not.

## Why nothing caught it

Since `ADR-53` made every rule opt-in, "no rules" is the exact "check that exists but reports nothing"
state doctor exists to catch, but only the empty-`record_types` form was ever checked. `SPEC-15` still
said there was "no per-rule enable/disable ... config yet", true before `ADR-53`.

## Fix

New `rules-enabled` doctor check, `error` when no rule has a level other than `off`. `SPEC-15` bumped to
0.5: the check is documented and the stale "which rules are off" paragraph is corrected.

## References

`ADR-53`, `SPEC-15`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed and fixed. **Why:** found by a round 30 review and reproduced against a disposable fixture. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
