---
Stable-Id: 01M6DEN888K1W3TG4P3CZ0P5KN
Status: InProgress
Phase: '0'
Track: schema-governance
Realized-by: code:rust/crates/urzua-core/src/rules.rs, code:rust/crates/urzua-core/src/rules/catalog.rs, code:rust/crates/urzua-core/src/rules/shared.rs, code:rust/crates/urzua-core/src/rules/header.rs, code:rust/crates/urzua-core/src/rules/type_scope.rs, code:rust/crates/urzua-core/src/rules/declaration.rs, code:rust/crates/urzua-core/src/rules/identity.rs, code:rust/crates/urzua-core/src/rules/pointer.rs, code:rust/crates/urzua-core/src/rules/relation.rs, code:rust/crates/urzua-core/src/rules/section.rs, code:rust/crates/urzua-core/src/rules/embodiment.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
---
# 131 — Split the rule engine's rules module into one module per rule family

## What

Split `rust/crates/urzua-core/src/rules.rs` (6,355 lines) into one module per rule family (identity, pointer, relation, section, value, scope, embodiment), with no change in behavior: every existing test passes unchanged and `urzua check` output on this repository is byte-identical before and after.

## Why

Every lane on the roadmap edits `rules.rs`, so parallel work on sections, values, relations and embodiment collides in one file and serializes. Splitting it first is the precondition for running those lanes at once. It is a pure move, so it is cheapest to do before the lanes start rather than after.

## Done means

- `rules.rs` is a thin module root over one module per rule family under `rules/` (`catalog`, `shared`, `header`, `type_scope`, `declaration`, `identity`, `pointer`, `relation`, `section`, `embodiment` and the others). — evidence: code:rust/crates/urzua-core/src/rules.rs, code:rust/crates/urzua-core/src/rules/catalog.rs, code:rust/crates/urzua-core/src/rules/shared.rs, code:rust/crates/urzua-core/src/rules/header.rs, code:rust/crates/urzua-core/src/rules/type_scope.rs, code:rust/crates/urzua-core/src/rules/declaration.rs, code:rust/crates/urzua-core/src/rules/identity.rs, code:rust/crates/urzua-core/src/rules/pointer.rs, code:rust/crates/urzua-core/src/rules/relation.rs, code:rust/crates/urzua-core/src/rules/section.rs, code:rust/crates/urzua-core/src/rules/embodiment.rs
- There is no change in behaviour: every pre-existing test passes unchanged. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs
- `urzua check` output on this repository is byte-identical before and after the split.

## References

- `MILE-98` -- the section parser lane that would otherwise conflict here first.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A gap the Phase 0 roadmap found with no record. | **substantive** |
> | 2026-11-03 | `rules.rs` is now a thin module root (re-exports and shared test fixtures) over fourteen submodules under `rules/`: `catalog` (rule ids, `ALL_RULES`, `RULE_METADATA`), `shared` (slot builders, per-type projectors, field-kind specs, reference helpers), `header`, `type_scope`, `declaration` (the `config.*` rules), `identity`, `pointer`, `narrative`, `field`, `claim`, `filename`, `revision_log`, `embodiment` and `relation`, each with its own tests alongside. A pure move: every public path under `urzua_core::rules` still resolves through `pub use` re-exports, the only visibility changes are `pub(super)` on helpers the families now share, and the workspace test count and `make records` result are unchanged. The root stays `rules.rs` rather than `rules/mod.rs` so existing `Realized-by` locators keep resolving. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #152: `rules.rs` is a thin root over fourteen submodules under `rules/`, with no change in behaviour. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Status` `Done` -> `InProgress`. Status set from its Done-means items (ADR-96, MILE-215): unresolved item 3. **Why:** the item has no evidence clause, and `DoneWithUnresolvedItems` forbids `Done` while an item is unresolved; `fix` never downgrades, so the status is set by hand to the computed value. | **structural** |
