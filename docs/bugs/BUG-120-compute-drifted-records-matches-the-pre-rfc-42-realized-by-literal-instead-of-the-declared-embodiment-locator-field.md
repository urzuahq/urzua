---
Stable-Id: 01M35T9RD87NMRK2Z64QK9C743
Status: Fixed
Found-in: "A /code-review 108 pass, reviewing RFC-42/ADR-61's implementation"
Regression-test: "drift_is_detected_against_a_custom_embodiment_locator_field_name_observed_failing, check_integration.rs"
---
# 120 — compute_drifted_records matches the pre-RFC-42 Realized-by literal instead of the declared embodiment_locator field

## What was wrong

`urzua-cli`'s `compute_drifted_records` matched `header.get("Realized-by")` and
`field.key == "Realized-by"` literally instead of resolving `RelationRole::EmbodimentLocator`, so drift
detection never recognized an `embodiment_locator` override.

A type overriding `embodiment_locator` to a custom name, with that field's evidence file changed since
the header line was last touched, never had its record added to the `drifted` set `check.rs` passes to
`embodiment_consistency` — so the expected `"Drift detected"` state could never fire for that type, even
though `embodiment_consistency` itself (fixed by `RFC-42`/`ADR-61`) correctly reads the overridden name.

## Why nothing caught it

`RFC-42`'s review scope was the diff itself; `discovery.rs`'s drift computation is `urzua-cli`-side
plumbing that feeds `embodiment_consistency` as precomputed data (the same shape `full_text` uses) and
wasn't touched by that PR. Nothing exercised drift detection against a non-default locator field name --
there was no integration test for drift detection at all before this fix.

## What changed

`compute_drifted_records` now takes `&Config` and resolves `RelationRole::EmbodimentLocator` through
`relation_field_name` (promoted to `pub` in `rules.rs` to cross the crate boundary), matching both the
header read and the field-line lookup used to find the reference commit.

## References

- `RFC-42`, `ADR-61` — the mechanism this bug's fix threads into `discovery.rs`.
- `BUG-118`, `BUG-119` — the same gap found the same day in `fix` and `explain`/`graph`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a full-PR code review of `RFC-42`'s implementation; a new git-backed integration test (drift detection had none before) observed failing against the hardcoded literal before the fix. | **substantive** |
