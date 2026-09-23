---
Stable-Id: 01M35QF98A6W7VYP7JHR3KTCDZ
Status: Done
Phase: '1'
Track: schema-governance
Implements: RFC-42
Blocked-on: —
---
# 109 — Implement declared relation field names replacing hardcoded status embodiment and supersession literals

## What

`RFC-42`/`ADR-61`'s implementation: a per-type `relation_fields` config map (`status`,
`embodiment_state`, `embodiment_locator`, `supersession`), each defaulting to its pre-`RFC-42` literal
(`Status`, `Embodiment`, `Realized-by`, `Supersedes / Superseded-by`). Seven call sites across
`claim_status_agreement`, `pointer_target_status`, `narrative_field_stale`, `embodiment_consistency`,
`embodiment_locator_exists`, `embodiment_locator_promotion_candidate`, and `supersession_reciprocity`
resolve their field name through `RecordTypeConfig::relation_field(role)` instead of a literal. A new
rule, `config.relation-field-not-known`, reports a declared override not also present in that type's
`required_fields`/`known_fields`.

## Why

`BUG-110` found `Status` hardcoded in three rules; a round-18 code review found the identical shape
recurring in four more rules for `Embodiment`, `Realized-by`, and `Supersedes / Superseded-by`. Seven
rules, one defect shape, so `RFC-42` consolidated them into one config mechanism rather than four
separate `status_field`-shaped patches, matching the precedent `MILE-90` already set for
`pointer_fields`/`narrative_fields`.

## Verification

Full test suite (219 tests, 10 new), `cargo fmt`, `clippy -D warnings`, `make ci`, and a real-corpus
`check` run reporting the same 70 findings before and after — this repository's own config declares no
`relation_fields` override, so the change is a no-op on it by design. Each of the four resolved roles
(`status`, `embodiment_state`+`embodiment_locator`, `supersession`) has its own planted-violation test,
observed failing against the pre-fix literal and passing after, plus a CodeRabbit-review-prompted fix
making every finding message report the resolved field name instead of the pre-`RFC-42` literal.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and implemented in one round. **Why:** `RFC-42`/`ADR-61` decided the design the same day a round-18 review surfaced the recurring defect shape; scheduled and built together so the gap didn't sit open for a later release. | **substantive** |
