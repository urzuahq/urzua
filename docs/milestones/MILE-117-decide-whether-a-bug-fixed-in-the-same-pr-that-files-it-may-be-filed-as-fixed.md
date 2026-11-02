---
Stable-Id: 01M6CXZTS2PNQ6Q0A7BN67K70E
Status: Planned
Phase: '0'
Track: governance-process
Implements: SPEC-19
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

## References

- `SPEC-19` -- governs `AGENTS.md` and its change history.
- `BUG-136`, `BUG-139`, `BUG-142` -- records filed directly as `Fixed`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** Rounds 25 to 27 filed same-PR bugs as `Fixed` against the rule that only a human sets that status; filed as an open decision rather than silently picking a side. | **substantive** |
