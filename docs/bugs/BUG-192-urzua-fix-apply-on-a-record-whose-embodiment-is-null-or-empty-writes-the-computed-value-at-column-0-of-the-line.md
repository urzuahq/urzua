---
Stable-Id: 01M6MKBDK1D9VWKK0KTYEMF7ZD
Status: Fixed
Found-in: 'running urzua fix --apply on a record with `Embodiment: null` and a Realized-by that computes Implemented (2026-11-05)'
Realized-by: test:rust/crates/urzua-core/src/fix.rs#apply_repair_fills_a_null_tilde_or_empty_value_by_position_observed_failing@sha256:e6b90c9196c14e82a6ea8a233069aae852d602f9b09b97b6b1066040b6561d41, test:rust/crates/urzua-core/src/fix.rs#apply_repair_does_not_touch_a_substring_of_the_field_name_or_prose_observed_failing@sha256:c0d4d468b6d4d5fc29895674fa850b80dd4879cca19be84f850f249a6d352e96
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
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs#apply_repair_fills_a_null_tilde_or_empty_value_by_position_observed_failing` (lines 1191-1215) 136de640dfd7 -> e6b90c9196c1, `rust/crates/urzua-core/src/fix.rs#apply_repair_does_not_touch_a_substring_of_the_field_name_or_prose_observed_failing` (lines 1217-1242) 025008389eb4 -> c0d4d468b6d4. Reason: Read the diff of each pinned span since the old pin and the claim still holds. fix.rs#apply_repair_fills_a_null_tilde_or_empty_value_by_position_observed_failing: the call to apply_repair now passes the Attestation and revision-log shape arguments the function gained, the assertions on the written field and row are unchanged; fix.rs#apply_repair_does_not_touch_a_substring_of_the_field_name_or_prose_observed_failing: the call to apply_repair now passes the Attestation and revision-log shape arguments the function gained, the assertions on the written field and row are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
