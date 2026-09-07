# 73 — Decide when a spec needs a new spec instead of an in-place revision

> Status: Planned
> Stable-Id: 01M1YSGQ9F9WXDWB2VJEQ657K4
> Phase: 1
> Track: governance-process
> Implements: RFC-6

## What

Resolve RFC-6's own admitted open question: when a spec has already shipped (`Status: Accepted`,
already executed) and new capability needs documenting, is the right move an in-place revision
(the living-spec model RFC-6 currently describes) or a new spec (leaving the executed one frozen)?
RFC-6's own Open Questions section names this exactly ("what does 'spec drifted enough to need a
new ADR' actually look like operationally") and explicitly defers it to a follow-up RFC rather than
answering it.

## Why

Hit directly, live: SPEC-7 was written as a new spec rather than a revision to SPEC-6 specifically
because RFC-6 doesn't say which is correct once a spec has executed. That call was made once, for
one case, without a real decision behind it -- the next time this comes up, nothing says whether to
follow the same precedent or do something else. Worth deciding properly rather than accumulating
inconsistent precedent one spec at a time.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
