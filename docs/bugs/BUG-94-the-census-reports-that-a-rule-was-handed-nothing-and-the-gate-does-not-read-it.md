---
Stable-Id: 01M30JSVKZ8DMSPCMVN0JHXG1X
Status: Fixed
Found-in: "Round 11 of the 0.4.0 review, against nine PRs that had never been reviewed in aggregate"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_rule_handed_nothing_discloses_that_it_certified_nothing"
---
# 94 — The census reports that a rule was handed nothing and the gate does not read it

## What was wrong

`Population` was added this release specifically so a rule could say what it had been handed.
`any_rule_looked` -- the one thing that decides whether a run may report `ok` -- did not read it:

```rust
executed.iter().any(|e| e.status == RuleStatus::Ran && e.scope == RuleScope::Records)
```

So a rule reporting `eligible: 0` still certified the corpus. Reproduced on a two-record corpus whose
config enables only `header.layout-consistency`, with no type declaring a `header_layout`:

```json
{ "status": "ok", "files_examined": 2, "findings": [],
   "rules_executed": [{ "rule": "header.layout-consistency",
     "population": { "unit": "record", "eligible": 0, "examined": 0 },
     "status": "ran" }] }
```

Exit 0. **One JSON object saying both "I was handed nothing" and "this corpus is fine."**

This is `ADR-55`'s pattern at a new level. The signal that a rule looked at nothing was built, shipped,
and consumed by nothing -- the third time this release a signal was produced without a reader, after
`records_examined` itself (`BUG-40`) and `doctor` (`BUG-91`).

## The fix, and what it is not

A converted rule counts toward "something was looked at" only when its population is record-derived
**and** non-empty. Unconverted rules keep the old signal until every rule carries a population.

Two constraints it must not break, both already recorded:

- **`BUG-81`/`BUG-83`**: a config or path rule with a non-empty population has read declarations or
  filenames, not records. The unit test stays.
- **`BUG-84`**: `audit` runs two unconverted rules, and on the configuration `init` writes both
  legitimately examine nothing. Requiring a non-zero count from them is that bug.

This is per *run*, not per rule: one rule with an empty population among others that had work is the
cold-start state and not a fault. `header.layout-consistency` is in it permanently on this corpus.

## What stays open

Once every rule is converted, the unconverted fallback disappears and `audit` on `init`'s config has
two record-derived populations that are legitimately empty -- which is `BUG-84` again. The plan
records that no unit-only predicate satisfies `BUG-84` and `BUG-88` at once; this fix does not resolve
that, it defers it to the point where the fallback is removed. `MILE-106` owns the final form.

## Why nothing caught it

Nine PRs merged between round 10 and this review, each reviewed individually and none in aggregate.
The gate was untouched by all nine, so no reviewer of any single PR had reason to look at it -- the
defect is in the *relationship* between a field one PR added and a predicate another PR did not
change.

## References

- `ADR-55`, `BUG-40`, `BUG-81`, `BUG-83`, `BUG-84`, `MILE-106`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | Regression test renamed and re-pointed at the disclosure. **Why:** the fix as filed made the gate read the population, and that gate is now deleted -- `BUG-88`'s re-grading established that `eligible: 0` is the cold start of enabling a rule before its scope exists, not a fault. The finding stands: a signal built this release and consumed by nothing is the defect. It is now consumed by `records_read_by_any_rule`, and the test asserts that a two-record corpus whose only rule was handed nothing reports `files_examined: 2, records_read_by_any_rule: 0`. | **substantive** |
