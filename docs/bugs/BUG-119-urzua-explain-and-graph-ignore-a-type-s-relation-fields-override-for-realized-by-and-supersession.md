---
Stable-Id: 01M35T9QZDY6MVXTREFE64H83Z
Status: Fixed
Found-in: "A /code-review 108 pass, reviewing RFC-42/ADR-61's implementation"
Realized-by: test:rust/crates/urzua-core/src/graph.rs#explain_reads_a_custom_embodiment_locator_field_name_observed_failing@sha256:30d4e5d33e2862d94e6ac0f39b50e3d5e1ed6155e614f0c17af87c48b8c4e1c9, test:rust/crates/urzua-core/src/graph.rs#graph_reads_a_custom_supersession_field_name_observed_failing@sha256:8be2dac24c0f85a9242255202d9c58beeacf8d5858f1f83575841ac400d83e67
---
# 119 — urzua explain and graph ignore a type's relation_fields override for Realized-by and supersession

## What was wrong

`urzua_core::graph::explain` read `record.header.get("Realized-by")` directly with no `Config`
parameter, and `graph`'s `const SUPERSESSION_FIELD: &str = "Supersedes / Superseded-by"` was a second,
independent hardcoded literal for the same concept `RFC-42`/`ADR-61` made adopter-declared. Neither
honored a type's `relation_fields.embodiment_locator`/`supersession` override.

A type overriding either field — the same shapes this PR's own new tests exercise for
`embodiment_locator_exists`/`supersession_reciprocity` — got `urzua explain <path>` silently omitting
the governing-record match, and `urzua graph` silently omitting the supersession edge, for that type,
while `check`'s equivalent rules worked correctly on the same corpus.

## Why nothing caught it

`RFC-42`'s review scope was the diff itself; `graph.rs` implements a read-only, informational view over
the same fields and wasn't touched by that PR, so nothing in the diff pointed at it. `graph.rs`'s own
tests all use the default field names.

## What changed

`explain` and `graph` both now take `&Config` and resolve `RelationRole::EmbodimentLocator`/
`Supersession` through `relation_field_name`, per record's own type, replacing the hardcoded
`"Realized-by"` read and the `SUPERSESSION_FIELD` constant. The exclusion filter that keeps a
config-declared `pointer_fields`/`narrative_fields` entry from duplicating the unconditional supersession
edge now compares against the resolved name too.

## References

- `RFC-42`, `ADR-61` — the mechanism this bug's fix threads into `graph.rs`.
- `BUG-118`, `BUG-120` — the same gap found the same day in `fix` and drift detection.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a full-PR code review of `RFC-42`'s implementation; two planted-violation tests observed failing against the hardcoded literals before the fix. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
