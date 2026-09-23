---
Stable-Id: 01M35T9QHFVNYZEDK6X18YTPWJ
Status: Fixed
Found-in: "A /code-review 108 pass, reviewing RFC-42/ADR-61's implementation"
Regression-test: "a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing, fix.rs"
---
# 118 — urzua fix ignores a type's relation_fields override for Embodiment and Realized-by

## What was wrong

`RFC-42`/`ADR-61` made `Status`/`Embodiment`/`Realized-by`/`Supersedes / Superseded-by` adopter-declared
per type, and rewired all seven rules `check` runs through the new `relation_field(role)` accessor. It
missed a caller outside `check`: `urzua_core::fix::detect_repairs` still read the literals `"Embodiment"`
and `"Realized-by"` directly, with no `Config` parameter to resolve an override from at all.

A type declaring `relation_fields.embodiment_state`/`embodiment_locator` — exactly the shape `RFC-42`'s
own new tests exercise for `embodiment_consistency` — got `check` correctly flagging a mismatch while
`urzua fix` silently reported zero repairs for the same record: two commands reading one concept,
disagreeing the moment an adopter used the mechanism the review's own PR had just built.

## Why nothing caught it

`RFC-42`'s review scope was the diff itself (`config.rs`, `rules.rs`, `property.rs`, `check.rs`) — `fix.rs`
wasn't touched by that PR, so nothing in the diff pointed at it, and `fix`'s own tests all used the
default field names, which never exercises the divergence.

## What changed

`detect_repairs` now takes `&Config` and resolves `RelationRole::EmbodimentState`/`EmbodimentLocator`
through the same `relation_field_name` helper `check`'s rules use (promoted to `pub` to cross the crate
boundary into `urzua-cli`). The `Repair.field`/`evidence` values now name the resolved field, not a
hardcoded literal.

## References

- `RFC-42`, `ADR-61` — the mechanism this bug's fix threads into `fix.rs`.
- `BUG-119`, `BUG-120` — the same gap found the same day in `explain`/`graph` and drift detection.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a full-PR code review of `RFC-42`'s implementation; a planted-violation test observed failing against the hardcoded literals before the fix. | **substantive** |
