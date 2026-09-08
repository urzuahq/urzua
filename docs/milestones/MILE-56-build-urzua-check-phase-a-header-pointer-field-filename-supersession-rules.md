# 56 — Build urzua check Phase A: header, pointer, field, filename, supersession rules

> Status: Done
> Stable-Id: 01M1YNN8V8XTS6CSCEEBAEWR9A
> Phase: 0
> Track: section-checks
> Implements: ADR-7, ADR-8, ADR-9, ADR-14
> Blocked-on: —

## What

The initial `check` rule set: `header.required-fields`, `pointer.resolution`, `field.quality`, `filename.title-consistency`, `relation.supersession-reciprocity` -- the substrate every other command builds on.

## Why

SPEC-2's own Phase A: a validator whose first run is green has told you nothing about itself, so both seed rules were chosen because they fire on this corpus's real, pre-existing defects.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
