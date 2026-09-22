---
Stable-Id: 01M30VE5PPB5TVTF85WW5BRH8H
Status: Open
Found-in: "Reviewing what the 0.4.0 population work did and did not fix; reproduced against a scratch repository built to the prefixless shape"
Regression-test: "init_warns_when_its_proposed_config_leaves_audit_with_nothing_declared, check_integration.rs"
Blocked-on: RFC-38
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

## The cause is not the edge case

This was first written up as an empty-rule-set edge case, with three candidate patches: warn from
`init`, teach `audit` to tell "nothing declared for me" from "could not run", or broaden what `audit`
runs. All three patch a symptom.

The cause is that `audit` owns a rule set at all. It runs two rule functions `check` already runs,
over the same corpus, producing the same report shape and exit codes -- a read-only subset of another
command. `ADR-53` then made rules a declared, opt-in policy, at which point a hardcoded subset inside
the tool overrides the adopter's declaration. `BUG-42` is that contradiction surfacing once already;
`gated()` fixed the severity half and left the selection half.

`BUG-42`, `BUG-84` and this bug are three instances of the one root. `RFC-38` proposes retiring the
command rather than patching it a third time, at which point this closes without a separate fix.

If `RFC-38` is rejected, warning from `init` is the cheapest patch, and teaching `audit` to tell
"nothing declared for me" from "could not run" reopens `BUG-77` for one command, which would need
arguing rather than assuming.

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
> | 2026-09-22 | Mitigated, not closed: `init` now emits a `rule-applicability` notice when its proposed config leaves `audit`'s entire rule set undeclared, so the adopter learns this before running `audit` rather than from its exit code. **Why:** the user asked to fix this before release; this record's own diagnosis says the root cause is `audit` owning a rule set at all (`RFC-38`), which stays undecided, so `Status` stays `Open` and `Blocked-on: RFC-38` stays -- this is exactly the "warning from `init` is the cheapest patch" option the previous entry already named, applied now rather than left for `RFC-38`'s resolution. `audit`'s own exit-code behavior is deliberately unchanged (`BUG-77`'s regression test still passes, and a new test asserts it stays that way alongside the new notice). | **substantive** |
> | 2026-09-21 | Re-diagnosed and `Blocked-on: RFC-38` declared. **Why:** filed as an empty-rule-set edge case with three candidate patches. Examining why `audit` has a rule set at all showed the edge case is a symptom of a read-only subset command that outlived `ADR-53`, and that `BUG-42` and `BUG-84` share that root. A bug record that misdiagnoses its own cause is the artifact this project keeps finding in review; corrected before it shipped rather than after. | **substantive** |
> | 2026-09-21 | Filed, with a reproduction. **Why:** named during the 0.4.0 population planning as a live `BUG-84` variant "under investigation, to be filed separately" and then not filed, so it existed only in a plan document no rule reads. Reproduced before filing rather than carried forward on the original assertion. | **substantive** |
