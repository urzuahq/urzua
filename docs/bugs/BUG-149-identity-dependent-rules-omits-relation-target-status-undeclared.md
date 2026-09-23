---
Stable-Id: 01M37DNT002P07Y8HC0KQG96N3
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 27"
Regression-test: "none -- covered by init.rs's existing a_corpus_with_no_prefix_is_not_offered_rules_that_cannot_fire test, which now also exercises relation.target-status-undeclared"
---
# 149 — IDENTITY_DEPENDENT_RULES omits relation.target-status-undeclared

## What was wrong

`IDENTITY_DEPENDENT_RULES` lists the rules `init`'s adopt mode should not propose on a prefix-less
corpus, because they resolve references through a `RecordIndex` that is empty when no record has a
filename-derived identity. `relation_target_status_undeclared` resolves references through exactly the
same `RecordIndex` mechanism (via the shared `for_each_resolved_status_target` helper, `BUG-144`) as
`pointer_target_status`, which is in the list -- but `relation_target_status_undeclared` itself was
missing. `urzua init` on a prefix-less repo would propose `relation.target-status-undeclared: warn`,
and `urzua check` would forever report it `ran` while resolving zero references -- the exact "rule
loads, reports ran, examines nothing" failure class this list exists to prevent (`BUG-61`).

## Why nothing caught it

`relation_target_status_undeclared` was added after `IDENTITY_DEPENDENT_RULES` was last reviewed
(`RFC-45`/`ADR-63`), and nothing re-checked whether every rule sharing the reference-resolution
mechanism was still represented in the list.

## Fix

Added `RULE_RELATION_TARGET_STATUS_UNDECLARED` to `IDENTITY_DEPENDENT_RULES`.

## References

- `BUG-61` -- the original defect class this list exists to prevent.
- `BUG-144` -- `for_each_resolved_status_target`, the shared mechanism both rules depend on identity
  through.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-24 | Filed and fixed in one pass, found by a full-release code review. **Why:** a rule added after this list's last review shared the exact identity-dependency shape the list exists to track, and was missed. | **substantive** |
