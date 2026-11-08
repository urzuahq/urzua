---
Stable-Id: 01M33QPNXNC0ARDJAP27B5HGDH
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Realized-by: test:rust/crates/urzua-core/src/config.rs#declared_fields_unions_required_and_known@sha256:f7b2584fedb2431840f75abbe6d4d4770418ba8529783abdd3e18bb4d273b771
---
# 106 — the declared for this type field union is computed independently in three places

## What was wrong

"A type's declared fields" (`required_fields` **∪** `known_fields`) was computed independently in
three places: `check.rs`'s `declared_fields_by_type`/`known_fields_by_type` builder, and
`rules::config_pointer_field_not_known`'s own inline union. `rules.rs`'s own `declared_slots` doc
comment already noted "`config.pointer-field-not-known` already unions the two" — documenting the
duplication without removing it. If the union rule ever changed (a new field kind, a casing policy),
only some call sites would get updated, and `field.untrimmed-value` and
`config.pointer-field-not-known` could start disagreeing about which fields a type declares — the
exact shape of the earlier, already-fixed defect where `field.untrimmed-value` was wired to the wrong
projection of this same union.

## Why nothing caught it

Each computation was three lines and looked cheap enough to write inline rather than extract, and each
individually was correct. Nothing compared the two computations to each other, so a divergence would
only surface as two rules disagreeing about one field on one record — which is exactly how the earlier
projection-wiring bug was found, by inspection rather than a test.

## What changed

`RecordTypeConfig::declared_fields()` (`config.rs`) is the one definition. `check.rs`'s
`declared_fields_by_type` builder and `rules::config_pointer_field_not_known` both call it.

## References

- `RFC-39` — names this exact class of drift for value comparisons; this is the same class one level
  up, for a config projection.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed and fixed in one pass. **Why:** found by a full-release code review; a mechanical extraction with no design decision to defer. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/config.rs#declared_fields_unions_required_and_known` (lines 4151-4181) 79057f66868f -> f7b2584fedb2. Reason: Read the diff of each pinned span since the old pin and the claim still holds. config.rs#declared_fields_unions_required_and_known: the test's Config or report fixture literal gained newly added default fields (for example roadmap, attestation, new, discovery, harness, authority, permanent, rules_selected, include_untracked), assertions unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
