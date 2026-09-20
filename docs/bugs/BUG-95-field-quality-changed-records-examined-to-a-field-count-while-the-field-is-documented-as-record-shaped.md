---
Stable-Id: 01M30JTQT7HREJD050M4C0JKHP
Status: Fixed
Found-in: "Round 11 of the 0.4.0 review, against nine PRs never reviewed in aggregate"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_field_rule_counts_declared_slots_not_records"
---
# 95 — field.quality changed records_examined to a field count while the field is documented as record-shaped

## What was wrong

`RuleExecution.population` is documented as the new, unit-bearing count, with `records_examined`
*"authoritative until every rule carries a population"* (`report.rs`). `field.quality` and
`field.pending` then changed `records_examined` from a record count to a field-slot count in the same
commit that gave them a population.

The result is an entry that contradicts itself:

```json
{ "rule": "field.quality", "records_examined": 6, "scope": "records",
  "population": { "unit": "field", "eligible": 6, "examined": 6 } }
```

Six, on a corpus of **two** records. `population` names the unit correctly while `records_examined`
and `scope` beside it name it wrong -- and `RuleScope` exists (`BUG-81`) precisely so a count cannot
claim a denominator it is not counting.

`scripts/generate-dashboard.py` embeds `rules_executed` verbatim into a published page, so the
contradiction is published rather than merely held.

`records_examined` is a record count again in both rules. `population` carries the field count.

## Why nothing caught it

The two fields were changed in one commit and reviewed as one change, so "the number is right" and
"the number is labelled right" were never separated. The population was correct throughout; only the
field documented as record-shaped moved under it.

Nothing asserts the invariant that a `records_examined` accompanied by `scope: records` cannot exceed
`files_examined`.

## References

- `BUG-40`, the untrue denominator this release is fixing; `BUG-81`, which introduced `scope` for
  exactly this reason.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
