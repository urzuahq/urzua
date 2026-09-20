---
Stable-Id: 01M2YR3GYZER3NSN1GDAN5GZ12
Status: Open
Found-in: "Round 10 of the 0.4.0 review, reproduced against a scratch repository"
Regression-test: "not yet written -- a corpus whose records none of the declared rules can read must not report ok"
Blocked-on: MILE-106
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
