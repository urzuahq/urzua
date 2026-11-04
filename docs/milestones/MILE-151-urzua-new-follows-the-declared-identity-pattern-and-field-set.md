---
Stable-Id: 01M6JKNQTB0RBVXEWN2RBZTH86
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 151 — Urzua new follows the declared identity pattern and field set

## What

Make `urzua new` honour the type's declared identity pattern, status vocabulary and closed field set. This gap is the feature request; its defect form is the filed bug in the same batch (BUG-182).

## Why

The adopter's authoring commands cannot adopt `urzua new` until it produces a record that passes the type's own rules, and `MILE-103` (the faster path replaces hand-authoring) cannot be met while the command's output fails `check`. A generator that disagrees with the checker is two sources of truth for one shape. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

`urzua new` run against a type with a padded numeric pattern writes a file and H1 that pass `filename.identity-pattern`, `header.required-fields` and `field.quality` with no manual edit; covered by a test over a pattern that is not the default; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Implemented in the `new` PR (not flipped to Done): `urzua new` numbers, names and titles by a declared `identity`, starts the status at the first declared value and honours a closed field set; `SPEC-12` 0.7 and `SPEC-3` 0.23; exercised by the `waiver` type's padded `identity` in `.urzua/config.yaml` (the one existing waiver renamed to `WAIVER-0001-...` with a `WAIVER-1` H1 to fit it). **Why:** the Done-when asks for a non-default pattern covered by a test and exercised in this repository. | **substantive** |
