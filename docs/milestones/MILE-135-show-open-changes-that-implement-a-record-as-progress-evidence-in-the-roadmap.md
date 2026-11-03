---
Stable-Id: 01M6DG4C3EHVQ8HP0832YET5Q9
Status: Planned
Phase: '3'
Track: roadmap-tracking
---
# 135 — Show open changes that implement a record as progress evidence in the roadmap

## What

A tool, separate from the engine's record checks, that reads open changes on a forge (pull requests
that cite a record in `Implements` or a `Closes`-style trailer) and adds them to `urzua roadmap` output
as progress evidence: in progress, with a link, against the record it implements.

## Why

`RFC-48` computes the roadmap from records in the repository only, so a milestone being built looks the
same as one nobody has started until its record changes. Reading the forge is a different trust and
dependency boundary from reading files, so it is kept out of the first roadmap and filed here as later
work. Needs `RFC-48`'s roadmap first.

## References

- `RFC-48` -- the roadmap this extends; its non-goals exclude forge input.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** Author asked to keep forge-based progress evidence as a future roadmap idea. | **substantive** |
