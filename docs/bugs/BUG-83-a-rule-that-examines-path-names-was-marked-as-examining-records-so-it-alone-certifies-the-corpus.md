---
Stable-Id: 01M2YPPRG2JB523YSJ8EKXDW8R
Status: Fixed
Found-in: "Round 9 of the 0.4.0 review, reproduced against a scratch repository"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_path_scoped_rule_alone_does_not_make_a_run_ok"
---
# 83 — A rule that examines path names was marked as examining records so it alone certifies the corpus

## What was wrong

`BUG-81` gave `RuleExecution` a `scope` so a rule counting configuration entries could not satisfy
"some rule examined a record". Six rules were marked `Config`.

`type.record-outside-declared-dir` was left `Records` and is neither. It inspects tracked *path
names* and never opens a file -- and the files it counts include the ones no type owns, which are
precisely the files no rule examines.

Reproduced: a corpus of one record whose entire content is `garbage not a header at all`, with only
`type.record-outside-declared-dir: warn` declared. Output: `records_examined: 1`, `scope: records`,
`ran` -- `status: ok`, **exit 0**, certifying a record whose header was never parsed.

`RuleScope` gains a third value for a population that is neither records nor configuration.

## Why nothing caught it

The audit that produced the six `Config` markings went rule by rule asking "does this read the
config?" and marked the ones that did. It never asked the complementary question of the rules left
alone -- whether each `Records`-marked rule actually reads a record.

`a_config_scoped_rule_alone_does_not_make_a_run_ok` pins one rule, `type.no-declared-spec`. A test
naming a single rule cannot cover a property every rule must have.

## References

- `BUG-81`, whose fix this completes.
- `BUG-62`, which added the rule.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
