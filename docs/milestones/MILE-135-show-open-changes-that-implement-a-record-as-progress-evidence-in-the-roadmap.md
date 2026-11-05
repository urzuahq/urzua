---
Stable-Id: 01M6DG4C3EHVQ8HP0832YET5Q9
Status: Planned
Phase: '3'
Track: roadmap-tracking
Depends-on: MILE-182
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

## Done means

A tool separate from the engine's record checks reads open changes on a forge (pull requests citing a record in `Implements` or a `Closes`-style trailer) and `urzua roadmap` output shows each against the record it implements as in progress, with a link; with no forge access the roadmap still renders from records alone and says the evidence was not read; a fixture with a stubbed forge shows a cited record in progress and an uncited one not.

## References

- `RFC-48` -- the roadmap this extends; its non-goals exclude forge input.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** Author asked to keep forge-based progress evidence as a future roadmap idea. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Depends-on: MILE-182` added. **Why:** Why states it needs the `RFC-48` roadmap first, which `MILE-182` builds. Proposed for maintainer review; `Status` unchanged. | **structural** |
