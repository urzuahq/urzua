---
Status: Planned
Stable-Id: 01M1Y5JQ7E7PXJZ2N7QSX0BVTS
Phase: '1'
Track: interop
Implements: ADR-2
Depends-on: COND-2
Blocked-on: COND-2 -- real adopter demand for AgDR round-tripping; its Signal says exactly what counts as met.
---
# 21 — AgDR export/import

## What

urzua export --format=agdr / urzua import round-tripping with the AgDR open standard.

## Why

Decided (own the schema, be AgDR-compatible on export), correctly left unbuilt -- zero external demand for round-tripping so far.

## Done means

`urzua export --format agdr` emits decisions as AgDR and `urzua import` reads AgDR back; exporting, importing and exporting again is lossless for the fields AgDR carries, and the unrepresentable fields are listed in the output; built only once an adopter asks for round-tripping, whom this record then names.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Status` Blocked -> Planned; `Blocked-on` unchanged. **Why:** `Blocked` is derived only (`RFC-48`, `ADR-83`) and this repository no longer writes it as a status. Nothing has started, so the status that matches the progress is `Planned`. What it waits on (real adopter demand for AgDR round-tripping) is a fact or a decision, not a record, so there is no `Depends-on` to state; `Blocked-on` keeps the wait until the `condition` record type exists (`RFC-48`). | **substantive** |
> | 2026-11-05 | `Depends-on: COND-2` added; `Blocked-on` now an explanatory note. `Status` unchanged. **Why:** `MILE-187`: the wait on an adopter asking for AgDR round-tripping was prose, invisible to `urzua roadmap` and the derived `blocked` value; it is now one `Open` condition, COND-2, whose `Signal` states what counts as met. A person moves the condition to `Met`, not this change. | **substantive** |
