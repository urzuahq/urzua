---
Status: Planned
Stable-Id: 01M2VN4CXDW71M0XYZHKDV9JZQ
Phase: '1'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 100 — Run green in a second real codebase with a different primary language

## What

`SPEC-1`'s Phase 1 exit criterion for this, in full:

> It runs green in **at least two** real target codebases, with different primary languages, via
> pre-push hook and CI, with zero host-repo runtime added to either.

One exists: this repository, Rust, CI only. The second has no milestone, no candidate named, and no
work item. It is a Phase 1 exit criterion, not part of the single Phase 0 exit (`SPEC-1`, Exit bar).

## Not the same as `MILE-51` or `MILE-96`

Those read a foreign **corpus**. This is a foreign **codebase**: urzua installed, configured, and
gating that repository's own CI, with a config its maintainers would recognise as describing their
conventions.

`MILE-51` proves the engine can *read* someone else's records. This proves it can be *adopted*, which
is a different claim and the one the criterion makes.

## Blocked on the document model

`MILE-51`'s re-run, after `BUG-36` and `BUG-37` landed, still returns nine findings of *"no
header-shaped region found"* against `npryce/adr-tools`, because a Nygard record keeps its metadata in
a bare `Date:` line and nothing can declare that. A second codebase cannot run green until the engine
can read a shape this project did not invent -- `MILE-98`.

## What it needs beyond that

- **A candidate**, chosen for a different primary language and a real corpus, not a fixture.
- **"Zero host-repo runtime"** taken literally: a binary and a config file, no dependency added to that
  repository's own build.
- **The two-config diff**, which `SPEC-3`'s second success criterion asks for: the two configs read
  side by side as a statement of how the corpora genuinely differ, rather than as two unrelated files.

## Done means

`urzua check` runs green in a second real codebase whose primary language differs from this repository's Rust, through both its pre-push hook and its CI, with zero host-repo runtime added to it (the `SPEC-1` Phase 1 exit criterion); the codebase is named in this record, the run is recorded with the commit, and it is a codebase adoption, not a foreign corpus (`MILE-51`, `MILE-96`).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `SPEC-1`'s first success criterion requires two target codebases and only one exists, with nothing on the plan tracking the second. Distinguished from `MILE-51`/`MILE-96` deliberately: those read a foreign corpus, this one adopts a foreign codebase, and only the second is what the criterion claims. | **substantive** |
> | 2026-11-04 | Reworded for the restated exit bar: the second codebase is a Phase 1 exit criterion, not part of the single Phase 0 exit. `Phase` is unchanged. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | `Phase` 0 -> 1. **Why:** Maintainer decision, 2026-11-04: the second codebase is a Phase 1 exit criterion, so it no longer gates Phase 0 or the switchover (`MILE-102`). | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-04 | `Blocked-on` cleared: `MILE-98` is Done (declared `identity`, `fields` and `sections`, including `###` headings read through `sections.depth`). `Status` unchanged. **Why:** `narrative-field.stale` flagged a blocker that reached a terminal status; the dependency no longer holds. | **substantive** |
