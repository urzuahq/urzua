---
Stable-Id: 01M2YMFMG02BH8JD5FAJQS4XQE
Status: Accepted
Date: 2026-09-20
Author: beauwilliams
Deciders: beauwilliams
---
# 55 — A rule must be able to fire and a test must assert that it looked

## Context

The most repeated defect in this project is a rule or check that **reports success without having
examined anything**. Seven review rounds over the 0.4.0 diff found roughly twenty instances:

| Instance | Found |
|---|---|
| `audit` bypassed the rules table entirely | round 1 |
| A rule gated off still reported `records_examined: N` | round 1 |
| `field.quality`'s template map was empty for every type, so the rule shipped inert and green | round 2 |
| A required-options guard accepted an empty vec as "declared" | round 3 |
| `is_terminal_status`'s `_ => &[]` made every unknown type's statuses non-terminal | round 4 onward |
| `init` wrote a config whose rules examine zero records | round 5 |
| A path scope made every cross-directory pointer dangle | round 6 |
| `claim.status-agreement` read one directory level and reported a clean run | round 6 |
| A path scope dropped findings about untracked files, so `check .` exited 0 where `check` exited 1 | round 7 |
| A `claim_paths` entry naming a file scanned nothing and passed | round 7 |
| `init` disarmed the identity rules for every type when one lacked a prefix | round 7 |

**Every one of them was visible in the tool's own output at the moment it shipped.**
`RuleExecution.records_examined` exists for precisely this purpose -- its own doc comment says it is
tracked so that "a rule that ran over zero derived input and a rule that ran cleanly over the whole
corpus" are distinguishable. The field has been in every report since it was introduced. Nothing
reads it. Seven rounds of adversarial review re-derived by hand a signal the tool was already
printing.

*(Amended 2026-09-21: `records_examined` was replaced by `RuleExecution.population`, which carries
`eligible` and `examined` with the unit they are counted in. The argument above is unchanged and the
diagnosis was understated -- the field was not only unread, it was untrue, holding three different
denominators under one name (`BUG-40`). See the Consequences below for the clause this changes.)*

`ADR-53` decided that every rule is a declared policy and every policy is opt-in. It does not say a
declared policy must be **capable of firing**, and that is the hole these twenty defects fall through.

The second half is the test suite. Two consecutive rounds shipped a planted-violation test that could
not fail:

- Round 6's first test for `BUG-60` passed against unfixed code, because its fixture omitted
  `header_shape`, so the rule examined zero records and the "no finding" assertion held vacuously.
- Round 7 found that `a_corpus_with_no_prefix_is_not_offered_rules_that_cannot_fire` used a single
  prefix-less type, where `any` and `all` are the same predicate -- it passed against the defect it
  was written to pin.

`AGENTS.md` already requires a planted-violation test observed failing before the fix. Both of these
satisfied that requirement and still proved nothing, because asserting a finding is *absent* passes
whether the code is correct or inert.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Keep relying on adversarial review | No work; it does find them | Found ~20 over seven rounds, each after shipping; rounds 2, 4, 6 and 7 each found a regression introduced by the previous round's fix |
| Document the pattern and rely on discipline | Cheap | The pattern is already named in three records and was re-derived from scratch every round anyway |
| Read the signal the tool already emits | Turns a manual re-derivation into a check; uses an instrument already built and paid for | `records_examined: 0` is sometimes legitimate, so the check needs a discriminator rather than a blanket rule |
| Require every rule to prove it examined something at the type level | Strongest guarantee | A large refactor of every rule signature, for a property that is observable from outside |

## Decision

In the context of an engine whose most repeated defect is a check that reports success without
looking, facing evidence that the signal distinguishing the two has been emitted and ignored for the
project's whole life, we decided that **a declared rule that ran and examined nothing is itself
reportable, and a test that pins a rule's verdict must assert the rule examined something**, to
convert a defect class found by review after shipping into one the tool reports about itself,
accepting that the first form needs a discriminator for the legitimately-empty case and that the
second is enforced by review rather than mechanically.

## Reversibility

Cheap both ways. The engine half is one opt-in rule (`MILE-106`), removable by deleting a config line
under `ADR-53`. The test half is a convention, and a convention that proves unhelpful is abandoned by
not following it. Neither changes a schema, an output contract, or a record shape.

The risk is not the decision being wrong, it is the rule being noisy enough to be turned off -- which
is the same failure as not having it, arrived at more slowly.

## Consequences

- A new opt-in rule reports a declared rule that ran over nothing (`MILE-106`).
- A planted-violation test asserts on the rule's **population** -- a specific `eligible`/`examined`
  pair -- not only on the presence or absence of a finding. Absence is not evidence when the rule may
  never have run.
- `AGENTS.md`'s existing requirement gains this second clause; satisfying the first alone has twice
  produced a test that could not fail.
- The legitimately-empty case must be discriminated rather than suppressed. A declared type a corpus
  does not use yet is a real state, and reporting it as a defect would make the rule noise.
  `type.dir-matches-nothing` already draws this distinction -- an absent directory is a
  misdeclaration, an empty one is a type not yet used -- and is the precedent to follow.
- Instances found in future rounds are appended to the table above, so the evidence accumulates in
  one place instead of being re-derived.

## References

- `ADR-53`, which made every rule a declared policy without requiring that a policy be able to fire.
- `MILE-106`, which builds the engine half.
- `BUG-60`, `BUG-67`, `BUG-70` -- the instances whose tests could not fail.
- `type.dir-matches-nothing`, the precedent for discriminating an empty state from a misdeclared one.
- `BUG-40`, which established that the signal this ADR relies on was itself not one number.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | Re-keyed the planted-test clause from `records_examined` to the rule's population. **Why:** this ADR's normative clause named a field that the same argument caused to be deleted -- the signal it told tests to assert on could not distinguish records from declared slots from configuration entries, so a test satisfying the clause could still be asserting against the wrong denominator. The requirement is unchanged; only the instrument it names is now one that carries its unit. | **substantive** |
