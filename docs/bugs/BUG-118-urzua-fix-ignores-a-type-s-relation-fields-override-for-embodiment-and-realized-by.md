---
Stable-Id: 01M35T9QHFVNYZEDK6X18YTPWJ
Status: Fixed
Found-in: "A /code-review 108 pass, reviewing RFC-42/ADR-61's implementation"
Realized-by: test:rust/crates/urzua-core/src/fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing@sha256:46f01692c3ffdc5691b8d4f2d250a36ce0cdd3e22088d1c01531df862d76e995
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
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing` (lines 1063-1129) 5aa5095668de -> 46f01692c3ff. Reason: Read the diff of each pinned span since the old pin and the claim still holds. fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing: the fixture literals gained the new default fields (roadmap, attestation, new, discovery, harness, authority, permanent) and the test helper detect replaced the detect_repairs call, assertions unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
