---
Stable-Id: 01M6NNMFMSJTP9V865RNXTYAJK
Status: Open
---
# COND-0002 — An adopter asks for AgDR round-tripping

## Signal

Met when a real external adopter asks for `urzua export --format agdr` and `urzua import` to round-trip with the AgDR standard. The ask is written down in a record (a milestone, an ADR or an RFC that names the adopter and the ask), and `Met-by` names that record. An internal wish to be compatible does not count: `ADR-2` already decided the schema and the export, and left the round trip unbuilt for want of demand.

## Why this is a condition

`MILE-21` waited on this in prose (`Blocked-on: real adopter demand for AgDR round-tripping`).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed `Open`. **Why:** `MILE-187`, the first records of the `condition` type (`RFC-48`). A person, not the PR that files it, moves it to `Met` or `Dropped`. | **structural** |
