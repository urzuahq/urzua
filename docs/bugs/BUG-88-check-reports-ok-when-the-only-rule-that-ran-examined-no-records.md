---
Stable-Id: 01M2YR3GYZER3NSN1GDAN5GZ12
Status: WontFix
Found-in: "Round 10 of the 0.4.0 review, reproduced against a scratch repository"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_rule_handed_nothing_discloses_that_it_certified_nothing -- asserting the disclosure, not a non-zero exit"
---
# 88 — check reports ok when the only rule that ran examined no records

## What was wrong

`BUG-77` made a run report `not-run` unless a rule had looked. `BUG-84` then relaxed the test from
"a record-scoped rule examined something" to "a record-scoped rule ran", because the strict form made
`audit` unsatisfiable on a clean corpus.

The relaxation is too wide in the other direction. Reproduced: a config declaring exactly one rule,
`relation.supersession-reciprocity: error`, over a single record whose entire content is
`no header at all, total garbage`:

```json
{"status": "ok", "files_examined": 1,
 "rules_executed": [{"relation.supersession-reciprocity", "records_examined": 0, "scope": "records", "ran"}]}
```

Exit 0, certifying a corpus nothing read. Before `BUG-84` this was `not-run`.

## Why it is not fixed here

Both available forms of the guard are wrong, in opposite directions, and this is the third round in
which that has been demonstrated:

| guard | fails |
|---|---|
| `examined > 0` | `BUG-84` -- `audit` unsatisfiable on a clean corpus, which is the config `init` writes |
| a record-scoped rule `ran` | this -- `ok` over a corpus no rule could read |

Neither is a defect in the guard's implementation. Both are the same missing distinction: a rule that
examined nothing because there was nothing to examine, versus one that examined nothing because it
could not address what was there.

That is `MILE-106`'s subject, and `MILE-106` now records the concrete shape -- each rule declares its
eligible population, so the signal is `eligible > 0 && examined == 0`. Tightening the proxy a third
time would produce a fourth defect in one direction or the other.

## Why nothing caught it

The guard's tests all supply a corpus where something is wrong and a rule that addresses it. A corpus
whose records are unreadable by the declared rules is a third case neither the strict nor the relaxed
form was tested against.

`ADR-55` requires a planted-violation test to assert the rule looked. No test asserts that a **green**
run carries a record-scoped rule with `records_examined > 0`, which is the assertion that would have
failed here.

## References

- `BUG-77` and `BUG-84`, the two previous positions of this guard.
- `MILE-106`, which owns the distinction and now records its shape.
- `ADR-55`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | `Status: Open` → `WontFix`, re-graded as a thin-config state rather than a defect. **Why:** nothing in the reported run was ever untrue. One policy was declared, it ran, and it held; the fixture is silent because it declares nothing that reads a header, which `ADR-53` makes the adopter's call. Attempting to close it through the gate is what produced `BUG-84` -- requiring a non-zero population made `audit` unsatisfiable on exactly the config `init` generates -- and the fourth review established with fixtures that no predicate over rule populations satisfies both at once. The real concern was that such a run reads as a clean corpus, and that is now answered by disclosure instead: the report carries `records_read_by_any_rule`, so the run exits 0 and states that no rule judged any record. Judging that number is `MILE-106`'s, as a declared opt-in rule. | **substantive** |
