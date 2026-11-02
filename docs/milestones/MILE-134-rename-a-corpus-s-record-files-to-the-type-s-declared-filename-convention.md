---
Stable-Id: 01M6DEN9P605CKC9GV4VVTE0PV
Status: Planned
Phase: '2'
Track: write-path
---
# 134 — Rename a corpus's record files to the type's declared filename convention

## What

`urzua migrate filenames`: rename a corpus's record files to its type's declared filename convention (`TYPE-NNNN-slug.md`), and rewrite relative links and path mentions to the renamed files, reporting every rewrite. Dry run by default.

## Why

`RFC-46` was rejected in favour of one convention: a corpus in another shape is renamed rather than tolerated. The first such adoption is renamed by a one-off script inside its own switchover change, so the tool is later work; it becomes worth building once adopting a second corpus repeats the same rename.

## References

- `RFC-46` -- the rejected alternative.
- `ADR-36` -- the filename convention.
- `MILE-120` -- the rule that reports off-convention files.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A gap the Phase 0 roadmap found with no record. | **substantive** |
