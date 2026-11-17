---
Stable-Id: 01M7KB9D2QB8GND2X47RHRWY35
Status: Planned
Phase: '1'
Track: write-path
Blocked-on: null
---
# 322 — urzua new does not write a field it has no value for as null

## What

Have `urzua new` write an unset field the way the type's template writes it, or leave it out, so that a new record does not start with a finding the author did not cause.

## Why

`urzua new` serialises a field it was given no value for as `null` (`new_record.rs`), although the type's template carries a placeholder for it. `header.required-fields` passes, since the key is present, and `field.quality` then reports the field as blank. The first thing a new record shows is a warning about the tool's own output.

## Done means

- A record made by `urzua new` with no value for a field carries the template's text for that field or omits the field, shown by a test against each initialised type profile.
- `urzua new` followed by `urzua check` on the staged record reports no finding about a field `new` wrote, or `new`'s report lists the fields still to fill with a next action.
- The same holds for a required field with no value given and for an optional pointer field left unset: neither is written as `null`, and an unset optional pointer field trips no pointer rule (`header.pointer-field-clean` reports it today), shown by a test of each.

## Amendment (2026-11-17): a field `new` has no value for is written empty

Accepted by the maintainer on 2026-11-17. The maintainer's decision, verbatim: "leave it empty and tell the agent what to fill ... that means it doesn't get filled with data that shouldn't be in there. The agent should explicitly have to fill in data that needs to be filled."

The first Done-means item, which says the record "carries the template's text for that field or omits the field", is delivered in the form this decision sets: the field is written empty, listed in `to_fill` with the template's hint. `urzua new` neither copies the template's text into the header nor omits the field.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-16 by an agent setting up a new adopting repository on release 0.4.6, relayed at the maintainer's request; checked against the code the same day. | **structural** |
> | 2026-11-17 | Added a Done-means item for required fields and unset optional pointer fields. **Why:** a second report from the same adopting repository on 2026-11-17 found a required field written as `null` with its template hint discarded, and unset optional pointer fields removed by hand; decided by the orchestrator on 2026-11-17, pending maintainer review. | **substantive** |
> | 2026-11-17 | Added the amendment of 2026-11-17: a field `new` has no value for is written empty and listed in `to_fill` with the template's hint, which is how the first Done-means item is delivered. **Why:** maintainer decision of 2026-11-17, accepted by the maintainer. | **substantive** |
