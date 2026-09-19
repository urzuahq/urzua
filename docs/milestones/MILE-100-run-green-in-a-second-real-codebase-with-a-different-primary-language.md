---
Status: Planned
Stable-Id: 01M2VN4CXDW71M0XYZHKDV9JZQ
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: MILE-98
---
# 100 — Run green in a second real codebase with a different primary language

## What

`SPEC-1`'s first success criterion, in full:

> It runs green in **at least two** real target codebases, with different primary languages, via
> pre-push hook and CI, with zero host-repo runtime added to either.

One exists: this repository, Rust, CI only. The second has no milestone, no candidate named, and no
work item -- which is why Phase 0 read as eleven milestones from done while two of its three exit
criteria had nothing tracking them.

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

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `SPEC-1`'s first success criterion requires two target codebases and only one exists, with nothing on the plan tracking the second. Distinguished from `MILE-51`/`MILE-96` deliberately: those read a foreign corpus, this one adopts a foreign codebase, and only the second is what the criterion claims. | **substantive** |
