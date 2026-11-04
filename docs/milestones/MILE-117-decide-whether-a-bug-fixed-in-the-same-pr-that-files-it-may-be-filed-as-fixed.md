---
Stable-Id: 01M6CXZTS2PNQ6Q0A7BN67K70E
Status: Planned
Phase: '1'
Track: governance-process
---
# 117 — Decide whether a bug fixed in the same PR that files it may be filed as Fixed

## What

Decide whether a bug record may be filed as `Status: Fixed` in the same PR that fixes it, and make the
written rule and the practice agree. Either `AGENTS.md` and `SPEC-19` state an explicit exception
(same-PR fix, merge by the maintainer is the decision), or same-PR bugs are filed `Open` and moved to
`Fixed` by a human afterwards.

## Why

`AGENTS.md` says only the user moves a record to `Fixed`. In practice, rounds 25 to 27 filed
same-PR bug records directly as `Status: Fixed` (`BUG-136`, `BUG-139`, `BUG-142` among them), and the
maintainer's merge of that PR served as the decision. The practice and the written rule disagree, and
one of them has to change; leaving both standing means the rule is followed only when nobody looks.
This milestone records the decision to be made. It does not edit `AGENTS.md` or `SPEC-19`; whichever
way it is decided, the edit and a `SPEC-19` revision-log entry belong in the PR that closes this.

## Done means

The maintainer decides whether a bug may be filed `Fixed` in the same pull request that fixes it, and the written rule and the practice agree: either `AGENTS.md` and `SPEC-19` state the same-PR exception (the maintainer's merge is the decision) or same-PR bugs are filed `Open` and moved to `Fixed` by a human afterwards; no document says the opposite of the practice.

## References

- `SPEC-19` -- governs `AGENTS.md` and its change history.
- `BUG-136`, `BUG-139`, `BUG-142` -- records filed directly as `Fixed`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** Rounds 25 to 27 filed same-PR bugs as `Fixed` against the rule that only a human sets that status; filed as an open decision rather than silently picking a side. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. Not needed to replace the adopter's linter. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
