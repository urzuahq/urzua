---
Stable-Id: 01M2WG83F1V7Q6DP9622KQTFE5
Status: Fixed
Found-in: "Round 7 of the 0.4.0 review"
Realized-by: test:rust/crates/urzua-core/src/rules/pointer.rs#a_bare_hyphen_number_token_is_not_a_reference_observed_failing@sha256:e1b6dfccf575bf9e0d5afbf44523a9b1f87b4a6f8775d7639dd9fb7d7675b107
---
# 72 — An empty type prefix passes the reference test so a bare hyphen-number token is read as a reference

## What was wrong

A reference is recognised by splitting on the first hyphen and requiring the prefix to be uppercase
and the remainder to be digits. `"".chars().all(|c| c.is_ascii_uppercase())` is `true`, so a token
like `-1` splits to `("", "1")` and passes.

`extract_references` and `header_pointer_field_clean` both accept it. `header.pointer-field-clean`
reports a pointer field holding `-1` as clean, and `pointer.resolution` emits
`Derives-from: -1 does not resolve to any discovered record` about a token that was never a
reference.

`scan_references` already guards with `!prefix.is_empty()`. The other two call sites do not.

## Why nothing caught it

Three call sites grew the same predicate separately, and only one of them considered the empty case.
The rules' tests supply well-formed references, so the vacuous-truth branch is never reached.

## References

- `BUG-7`, an earlier defect in the same reference-recognition code.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Regression test added: `a_bare_hyphen_number_token_is_not_a_reference_observed_failing`, observed failing with the fix undone and passing with it. **Why:** this record named no test, so its `Fixed` had no evidence behind it under ADR-84. | **substantive** |
> | 2026-11-05 | `Regression-test` retired (ADR-84); its value named no test, so a regression test was written and cited in `Realized-by`. **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
