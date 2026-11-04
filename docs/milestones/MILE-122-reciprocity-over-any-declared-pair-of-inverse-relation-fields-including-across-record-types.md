---
Stable-Id: 01M6CXZWGF7KPM5ZBYF8X8J8PQ
Status: Done
Phase: '0'
Track: schema-governance
Realized-by: code:rust/crates/urzua-core/src/rules/relation.rs, test:rust/crates/urzua-cli/tests/check_integration.rs, test:rust/crates/urzua-cli/tests/scoped_pair_integration.rs
---
# 122 — Reciprocity over any declared pair of inverse relation fields, including across record types

## What

Generalize reciprocity checking to any declared pair of inverse relation fields, including pairs that
span two record types (for example a field on one type and its inverse on another), instead of the one
`Supersedes / Superseded-by` pair the engine checks. The pair is declared in config; the rule reports a
claim made on one side that the other side does not echo.

## Why

`BUG-41` records that companion-field pairs are checked for one pair and hardcoded to it, and `BUG-17`
records that `Amends` has no reciprocal field or check at all. Both are the same gap seen from two
records; neither is a milestone, and each has stayed `Open`. A corpus declaring inverse
pairs of its own, some across types, cannot be checked until this exists, so it is a requirement for
`MILE-102`, not a nicety. It is also
what the config-driven thesis asks for: a hardcoded pair is a type-specific assumption in Rust source.

## References

- `BUG-41` -- companion-field pairs checked for one pair, hardcoded.
- `BUG-17` -- `Amends` has no reciprocal field or check.
- `MILE-102` -- the adopter replacement this serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `BUG-17` and `BUG-41` describe halves of one gap and neither plans the general fix; filed as the milestone that does. | **substantive** |
> | 2026-11-04 | Built: `relation_pairs` config declaration and the `relation.pair-reciprocity` and `relation.cross-type-disallowed` rules, beside the unchanged `relation.supersession-reciprocity`. The narrowest reading of each question `RFC-47` leaves open is recorded in `SPEC-3`. **Why:** `RFC-47` was accepted on 2026-11-04. `Status` is left for the maintainer. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #163: declared `relation_pairs` with `relation.pair-reciprocity` and `relation.cross-type-disallowed`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
