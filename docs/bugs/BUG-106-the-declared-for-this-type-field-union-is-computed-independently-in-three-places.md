---
Stable-Id: 01M33QPNXNC0ARDJAP27B5HGDH
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "declared_fields_unions_required_and_known, config.rs"
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
