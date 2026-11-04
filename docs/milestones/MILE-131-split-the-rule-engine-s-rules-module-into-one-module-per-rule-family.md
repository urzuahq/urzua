---
Stable-Id: 01M6DEN888K1W3TG4P3CZ0P5KN
Status: Planned
Phase: '0'
Track: schema-governance
---
# 131 — Split the rule engine's rules module into one module per rule family

## What

Split `rust/crates/urzua-core/src/rules.rs` (6,355 lines) into one module per rule family (identity, pointer, relation, section, value, scope, embodiment), with no change in behavior: every existing test passes unchanged and `urzua check` output on this repository is byte-identical before and after.

## Why

Every lane on the roadmap edits `rules.rs`, so parallel work on sections, values, relations and embodiment collides in one file and serializes. Splitting it first is the precondition for running those lanes at once. It is a pure move, so it is cheapest to do before the lanes start rather than after.

## References

- `MILE-98` -- the section parser lane that would otherwise conflict here first.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A gap the Phase 0 roadmap found with no record. | **substantive** |
> | 2026-11-03 | `rules.rs` is now a thin module root (re-exports and shared test fixtures) over fourteen submodules under `rules/`: `catalog` (rule ids, `ALL_RULES`, `RULE_METADATA`), `shared` (slot builders, per-type projectors, field-kind specs, reference helpers), `header`, `type_scope`, `declaration` (the `config.*` rules), `identity`, `pointer`, `narrative`, `field`, `claim`, `filename`, `revision_log`, `embodiment` and `relation`, each with its own tests alongside. A pure move: every public path under `urzua_core::rules` still resolves through `pub use` re-exports, the only visibility changes are `pub(super)` on helpers the families now share, and the workspace test count and `make records` result are unchanged. The root stays `rules.rs` rather than `rules/mod.rs` so existing `Realized-by` locators keep resolving. | **substantive** |
