---
Status: Planned
Stable-Id: 01M1Y5JR9JPNCX4YYJ8SM18V5H
Phase: '1'
Track: interop
Implements: ADR-33
Blocked-on: a real external adopter asking for it
---
# 23 — A general migrate header-shape command

## What

A shipped, public urzua migrate header-shape subcommand converting blockquote/bold-list to yaml-frontmatter for any adopter.

## Why

Explicitly rejected as premature in the corrected ADR-33 -- no adopter is under real pressure to migrate while parsing support stays permanent.

## Done means

`urzua migrate header-shape` converts a corpus's records to `yaml-frontmatter` byte-preserving everything below the header, a second run is a no-op, a record it cannot convert is reported, and a fixture corpus converts and then passes `check`; built only once an adopter asks for it, whom this record then names.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Status` Blocked -> Planned; `Blocked-on` unchanged. **Why:** `Blocked` is derived only (`RFC-48`, `ADR-83`) and this repository no longer writes it as a status. Nothing has started, so the status that matches the progress is `Planned`. What it waits on (a real external adopter asking for it) is a fact or a decision, not a record, so there is no `Depends-on` to state; `Blocked-on` keeps the wait until the `condition` record type exists (`RFC-48`). | **substantive** |
