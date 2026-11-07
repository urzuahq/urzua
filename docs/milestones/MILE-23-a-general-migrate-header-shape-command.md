---
Status: Planned
Stable-Id: 01M1Y5JR9JPNCX4YYJ8SM18V5H
Phase: '1'
Track: interop
Implements: ADR-33
Depends-on: COND-3
Blocked-on: COND-3 -- a real external adopter asking for it; its Signal says exactly what counts as met.
---
# 23 — A general migrate header-shape command

## What

A shipped, public urzua migrate header-shape subcommand converting blockquote/bold-list to yaml-frontmatter for any adopter.

## Why

Explicitly rejected as premature in the corrected ADR-33 -- no adopter is under real pressure to migrate while parsing support stays permanent.

## Done means

- `urzua migrate header-shape` converts a corpus's records to `yaml-frontmatter`, byte-preserving everything below the header.
- A second run is a no-op.
- A record it cannot convert is reported.
- A fixture corpus converts and then passes `check`.
- It is built only once an adopter asks for it, whom this record then names.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Status` Blocked -> Planned; `Blocked-on` unchanged. **Why:** `Blocked` is derived only (`RFC-48`, `ADR-83`) and this repository no longer writes it as a status. Nothing has started, so the status that matches the progress is `Planned`. What it waits on (a real external adopter asking for it) is a fact or a decision, not a record, so there is no `Depends-on` to state; `Blocked-on` keeps the wait until the `condition` record type exists (`RFC-48`). | **substantive** |
> | 2026-11-05 | `Depends-on: COND-3` added; `Blocked-on` now an explanatory note. `Status` unchanged. **Why:** `MILE-187`: the wait on an external adopter asking for a general header-shape migration was prose, invisible to `urzua roadmap` and the derived `blocked` value; it is now one `Open` condition, COND-3, whose `Signal` states what counts as met. A person moves the condition to `Met`, not this change. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
