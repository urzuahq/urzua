---
Stable-Id: 01M2WCN953GF5BXSZB5DFM2AD9
Status: Open
Found-in: "Round 6 of the 0.4.0 release review"
Regression-test: "not yet written -- a record with an empty `#` line followed by a real H1 whose number disagrees with the filename must report the disagreement"
---
# 64 — An empty H1 line abandons the title scan instead of skipping it

## What was wrong

`first_h1` returns `None` when it meets an H1 line whose text is empty, which abandons the scan
rather than skipping the line.

A record whose body opens with a bare `#` and then carries a real title is reported by
`filename.title-consistency` as `no H1 title found to check against the filename's number`. If that
real title disagrees with the filename -- `ADR-7-x.md` containing `# ADR-9 - Wrong number` -- the
disagreement is never compared, and the rule reports the absence of a title instead of the mismatch it
exists to find.

The rule is declared `error` here, so the wrong message is also a blocking one.

## Why nothing caught it

`filename_title_consistency_treats_an_empty_h1_as_absent` pins the case where the empty `#` is the
only H1, and that behaviour is right. Because the fixture has nothing after it, the test cannot
distinguish `return None` from `continue` -- both pass. The abort-versus-skip distinction was never
exercised.

## References

- The existing test `filename_title_consistency_treats_an_empty_h1_as_absent`, which must keep
  passing.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
