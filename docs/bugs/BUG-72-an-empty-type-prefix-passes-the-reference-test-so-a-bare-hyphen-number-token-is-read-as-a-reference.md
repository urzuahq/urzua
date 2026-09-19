---
Stable-Id: 01M2WG83F1V7Q6DP9622KQTFE5
Status: Fixed
Found-in: "Round 7 of the 0.4.0 review"
Regression-test: "rust/crates/urzua-core/src/rules.rs::the existing reference tests, with !prefix.is_empty() added at both call sites"
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
