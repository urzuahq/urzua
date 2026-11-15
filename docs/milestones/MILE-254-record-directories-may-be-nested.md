---
Stable-Id: 01M7FW1M365CZ6MGA2EP81VDNC
Status: Planned
Phase: '2'
Track: engine-extensibility
Blocked-on: null
---
# 254 — Record directories may be nested

## What

Let a record type's directory be recursive or a glob, so records can live in folders by year or by team without listing each.

## Why

the audit of 2026-11-15's extensibility experiments found that a record must sit directly in a listed directory; nested layouts work only by listing every folder under `extra_dirs`.

## Done means

- A type declared recursive discovers records in nested folders, and `urzua new` writes into a chosen one.
- Two records with the same number in different folders of one type are a duplicate finding.
- A configuration that does not declare it behaves as before.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
