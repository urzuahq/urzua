---
default: minor
---

Adds a new optional per-type config key, `relation_fields`, mapping a fixed role set (`status`,
`embodiment_state`, `embodiment_locator`, `supersession`) to the field name that plays that role for a
record type — e.g. `relation_fields: { status: State }` for a type that calls its lifecycle field
`State` instead of `Status`. Every role defaults to its pre-existing literal
(`Status`/`Embodiment`/`Realized-by`/`Supersedes / Superseded-by`) when undeclared, so no existing
config needs to change.

Replaces seven rules' hardcoded field-name literals (`claim_status_agreement`, `pointer_target_status`,
`narrative_field_stale`, `embodiment_consistency`, `embodiment_locator_exists`,
`embodiment_locator_promotion_candidate`, `supersession_reciprocity`) with a lookup through the
declared-or-defaulted name, in both the gating logic and every finding message, closing `BUG-110`
(`Status` hardcoded) and the same defect shape found recurring in
`Embodiment`/`Realized-by`/`Supersedes / Superseded-by` during review. Adds
`config.relation-field-not-known`, reporting a declared `relation_fields` override not also present in
that type's `required_fields`/`known_fields`, mirroring `config.pointer-field-not-known`.

Design decided in `RFC-42`/`ADR-61`. No adopter-facing behavior changes for a config that doesn't
declare `relation_fields`: verified with the full test suite, clippy, `make ci`, and a real-corpus
`check` run reporting the same 70 findings before and after.
