---
Stable-Id: 01M30VE5PPB5TVTF85WW5BRH8H
Status: Open
Found-in: "Reviewing what the 0.4.0 population work did and did not fix; reproduced against a scratch repository built to the prefixless shape"
Regression-test: "not yet written -- `init` then `audit` on a corpus with no type prefix must not exit 2"
---
# 99 — urzua audit exits 2 on the config urzua init writes for a corpus with no type prefix

## What was wrong

On a corpus whose filenames carry no type prefix (`0001-alpha.md`, the shape `ADR-36` declined for
new records and every pre-adoption corpus may already use), the adopter's first two documented
commands are `urzua init` and `urzua audit`. The second exits 2 on the config the first just wrote.

Reproduced on a two-record scratch repository:

```text
$ urzua init      # writes .urzua/config.yaml, 18 rules
$ urzua audit
{
  "status": "not-run",
  "files_examined": 2,
  "records_read_by_any_rule": 0,
  "rules_executed": [
    { "rule": "pointer.resolution",                "status": "not-enabled" },
    { "rule": "relation.supersession-reciprocity", "status": "not-enabled" }
  ]
}
exit 2                       # `urzua check` on the same corpus exits 0
```

Exit 2 is `SPEC-1`'s "could not run" -- the same code a missing or unparseable config produces. The
adopter is told their setup is broken when nothing about it is.

## Why nothing caught it

**Every individual decision in the chain is correct**, which is why eleven review rounds passed over
it.

`init` omits five rules through `identity_dependent` (`init.rs:192`) because a corpus with no prefix
gives them no identity to read -- they would load, report `ran`, and examine nothing. Declining to
propose a rule that cannot fire is `BUG-61`'s fix and is right.

`audit` runs exactly two rules, `pointer.resolution` and `relation.supersession-reciprocity`. **Both
are in that list.** So on this corpus `audit` has no declared rule at all.

The gate then reports `not-run`, which is also right: no rule ran, and `BUG-77` established that a run
in which nothing ran has established nothing.

The defect is in the seam between the three. `audit` is the only command whose entire rule set -- two
rules -- can be legitimately declined in full, and nothing checks that composition. `check` has
eighteen rules and cannot reach this state, so every fixture exercising the gate exercised `check`.

The 0.4.0 verdict/disclosure split does not fix it either: the rules are `not-enabled` rather than
having run over an empty population, so `any_rule_looked` is false before and after.

## Candidate fixes

Not decided here; the choice belongs with whoever takes it.

1. **`init` warns** when the config it writes leaves `audit` with no declared rule. Cheapest, and
   consistent with `ADR-33` -- adopt mode proposes and reports, it does not decide. Fixes the
   surprise, not the exit code.
2. **`audit` distinguishes** "no rule declared for me" from "could not run". This is the `ADR-53`
   reading: an adopter who declared no audit policy has no audit policy to enforce. It reopens
   `BUG-77`'s question for this one command and needs that argued rather than assumed.
3. **Broaden what `audit` runs**, so its rule set is not wholly identity-dependent. The largest
   change and the one most likely to have its own consequences.

## References

- `BUG-61`, whose fix introduced `identity_dependent`.
- `BUG-70`, why the check is every-type rather than any-type.
- `BUG-77`, why a run with no rule reports `not-run`.
- `BUG-84`, the same collision reached from the other direction -- a guard that made `audit`
  unsatisfiable on `init`'s own config.
- `ADR-33`, `ADR-53`, `SPEC-11`.
- `MILE-51`, the adoption work whose corpus shape this is.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | Filed, with a reproduction. **Why:** named during the 0.4.0 population planning as a live `BUG-84` variant "under investigation, to be filed separately" and then not filed, so it existed only in a plan document no rule reads. Reproduced before filing rather than carried forward on the original assertion. | **substantive** |
