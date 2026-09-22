---
Stable-Id: 01M34QAQEH8KSDNVKXY5R7MW1V
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "not applicable -- an efficiency fix with no observable behavior change; verified by a real-corpus check run reporting identical findings"
---
# 116 — identity.collision rebuilds the shared record index independently

## What was wrong

`check.rs` builds one shared normalized-record index (`build_normalized_index`) and passes it to five
rules, specifically to avoid the O(corpus) traversal each used to do independently
(`round-14-duplication`). `identity_collision` was not one of the five: it called
`build_index_reporting_collisions` — the same underlying function `build_normalized_index` wraps —
on its own, doing the identical traversal and `HashMap` construction a second time in the same
`check` run.

## Why nothing caught it

`identity_collision` needs the *collisions* list, not just the index, and `build_normalized_index`
discards collisions (`.0` of the pair `build_index_reporting_collisions` returns) — so simply handing
it the shared index wouldn't have been enough, and nothing revisited the call site once the shared-index
work landed for the other five rules.

## What changed

`check.rs` now calls `build_index_reporting_collisions` once, keeping both the index (for the other
five rules) and the collisions list (for `identity_collision`, which now takes it as a parameter
instead of rebuilding it internally). `identity_collision` and `build_index_reporting_collisions`'s
`IdentifierCollision` type are made `pub` to cross the crate boundary.

## References

- `round-14-duplication`'s own shared-index consolidation — the fix this bug's site was missed by.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a full-release code review; verified as a no-op via a real-corpus `check` run reporting identical findings before and after. | **substantive** |
