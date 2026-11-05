---
Stable-Id: 01M6MF5W5K7R6PVMV1B71HRG3C
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 186 — Report declared non-reference entries in their own field-entry population

## What

`citation.resolution`'s check of `non_reference_field` entries (a declared id the record never cites, and a malformed entry) is reported under its own population, with a new `PopulationUnit` for a field entry, so a run over no declarations is distinguishable from a clean one.

## Why

`ADR-75` clause 3 says that check has its own population (unit: field entry). `BUG-190` built the check inside `citation.resolution`, whose one population counts citations, because a rule has one population and a second one changes the output contract (a new `PopulationUnit`, and either a second execution or a separate rule). That is a maintainer decision, so it was deferred.

## Done means

- A `PopulationUnit` for a field entry exists and is documented in `SPEC-2`, with the output-contract change recorded.
- A run with `non_reference_field` set reports a field-entry population counting each declared entry; a run over no declarations reports zero eligible entries.
- The uncited-declaration and malformed-entry findings are counted in it, and `config.scope-matches-nothing` does not read it as a misconfiguration.
- It is exercised on `ADR-18` and `RFC-5` in this repository's own corpus.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** the field-entry population `ADR-75` clause 3 asks for was deferred by the `BUG-190` build. | **substantive** |
