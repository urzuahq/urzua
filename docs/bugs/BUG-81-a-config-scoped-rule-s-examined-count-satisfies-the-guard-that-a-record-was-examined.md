---
Stable-Id: 01M2YNYXPM0WMY00DWZKV6E1M3
Status: Fixed
Found-in: "Review of the BUG-77 fix, reproduced against a scratch repository"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_config_scoped_rule_alone_discloses_that_it_read_no_record"
---
# 81 — A config-scoped rule's examined count satisfies the guard that a record was examined

## What was wrong

`BUG-77` made `check` report `not-run` unless some rule ran and examined something, reading
`records_examined` as `ADR-55` requires.

Six rules examine the *configuration* rather than the corpus -- `type.no-declared-spec`,
`type.dir-matches-nothing`, `header.deprecated-shape` and the three `config.pointer-*` rules -- and
they store their count of declarations in the same field. The guard could therefore be satisfied by
a rule that had never read a record.

Reproduced: `rules: {type.no-declared-spec: warn}` over one ADR missing its required `Status`.
Output: `type.no-declared-spec` `records_examined: 1, ran`, `status: ok`, **exit 0** -- the exact
result `BUG-77` was written to prevent, reached through a count that was not counting records.

`RuleExecution` now carries a `scope`, and the guard considers only record-scoped rules.

## Why nothing caught it

`records_examined` has always held two different quantities. That was harmless while nothing read the
field, and `ADR-55` made it load-bearing the day before this guard was written. The defect was
introduced by the first code to trust it.

`BUG-78` was the same discovery from the other direction -- a rule inflating the count with a record
it never inspected. Neither was visible until something depended on the number being true.

## References

- `BUG-77`, whose fix this defeats, and `ADR-55`, which makes the count load-bearing.
- `BUG-78`, the same signal being untrue for a different reason.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | Regression test rewritten, `Status: Fixed` unchanged. **Why:** this record makes two claims and only the second changes. The first -- `type.no-declared-spec` reported configuration declarations as `records_examined` -- was the real defect, and it is now fixed structurally rather than by a label: the count carries its unit (`record-type`), so it cannot be read as records by anything. The second -- that the untrue count satisfied the gate -- no longer applies, because the gate no longer asks a second question; declaring only this rule is a thin config, which `ADR-53` makes the adopter's call. The rewritten test asserts what the bug was actually about: the rule's population reports `unit: record-type`, and `records_read_by_any_rule` is 0. | **substantive** |
