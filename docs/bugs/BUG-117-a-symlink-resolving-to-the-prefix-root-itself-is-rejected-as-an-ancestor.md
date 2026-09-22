---
Stable-Id: 01M34QAQV2AY57C4HGXYT7X5G9
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "a_symlink_resolving_to_the_prefix_root_itself_is_deduped_not_rejected, check_integration.rs"
---
# 117 — a symlink resolving to the prefix root itself is rejected as an ancestor

## What was wrong

`read_claim_files`'s symlink-ancestor guard used `prefix_canonical.starts_with(&resolved)`, which is
reflexive: a symlink resolving to exactly the declared prefix root (not a genuine ancestor of it) also
satisfies `starts_with`, so it aborted the whole `check` run with "resolves to an ancestor of the
declared prefix," even though no unbounded walk would occur — `visited` is seeded with the prefix
root before the walk starts, so a self-referential link is silently deduped, never followed twice.

## Why nothing caught it

The existing symlink tests covered a link to a sibling directory, a nested directory, and a genuine
ancestor; nothing exercised a link resolving to the prefix root itself.

## What changed

The check now requires `resolved != prefix_canonical` in addition to the `starts_with` test, so a
self-referential link falls through to the ordinary `visited` dedup instead of erroring.

## References

- `BUG-69`, `BUG-85` — the ancestor-walk and outside-the-repository guards this check is adjacent to.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a full-release code review; a planted-violation test observed failing against the reflexive `starts_with` check before the fix. | **substantive** |
