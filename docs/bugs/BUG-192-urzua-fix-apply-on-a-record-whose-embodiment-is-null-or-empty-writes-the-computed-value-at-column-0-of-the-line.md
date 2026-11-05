---
Stable-Id: 01M6MKBDK1D9VWKK0KTYEMF7ZD
Status: Fixed
Found-in: 'running urzua fix --apply on a record with `Embodiment: null` and a Realized-by that computes Implemented (2026-11-05)'
Realized-by: test:rust/crates/urzua-core/src/fix.rs#apply_repair_fills_a_null_tilde_or_empty_value_by_position_observed_failing@sha256:136de640dfd781b220e334af52033670a370b33bd456664b16ae592b2ac786b7, test:rust/crates/urzua-core/src/fix.rs#apply_repair_does_not_touch_a_substring_of_the_field_name_or_prose_observed_failing@sha256:025008389eb47e78d3da26aa0481c17ad702568664912467076644c7a3d1ecf7
---
# 192 — urzua fix --apply on a record whose Embodiment is null, ~ or empty writes the computed value at column 0 of the line

## What was wrong

For a record with `Embodiment: null`, `~` or an empty value and a `Realized-by` that computes `Implemented`, `urzua fix --apply` rewrote the line as `ImplementedEmbodiment: null`, exited 0 and added a revision row saying "stated `` disagreed". `apply_repair` edited the line with `replacen(&current_value, &computed_value, 1)`; with an empty stated value `contains("")` always matches, so the computed value was inserted at column 0 and the real value was left in place. The record was corrupted silently, and ADR-74 had to be repaired by hand once.

## Why nothing caught it

Every `apply_repair` test used a non-empty stated value (`Not started`, `Implemented`), where a substring match and a positional edit give the same text. No test covered a stated value the YAML reader returns as empty, and nothing asserted that the edit changed only the value after the key.

## References

Fixed in `apply_repair`: the value after `Field:` on the field's own line is replaced by position, so no substring is ever searched. An empty, `~` or `null` stated value matches an empty, `~` or `null` value and is filled in place; a non-empty stated value must equal the value on the line or the repair is refused as stale. The Status repair (`MILE-183`) goes through the same function and is covered by it. The revision row reads "stated no value disagreed" for an empty stated value. `SPEC-8` is updated. A search of this repository's records found no `*Embodiment` or `*Status` line and no "stated `` disagreed" row, so no record needed repair; the ADR-74 hand repair is the only known instance.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed and fixed. **Why:** `urzua fix --apply` corrupted a record with an empty `Embodiment`. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
