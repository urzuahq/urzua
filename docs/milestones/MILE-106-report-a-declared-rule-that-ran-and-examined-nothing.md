---
Stable-Id: 01M2YMFN53X5QB064ZV60ZSQJT
Status: Planned
Phase: '0'
Track: schema-governance
---
# 106 — Report a declared rule that ran and examined nothing

## What

An opt-in rule that reports a **declared** rule which ran and examined nothing, so the engine detects
in itself the defect class `ADR-55` names: a check that reports success without looking.

The input is already there. `check`'s report carries a `RuleExecution` per rule with
`records_examined` and `status`, and the rule reads that table rather than the corpus.

The discriminator is the work. `records_examined: 0` is legitimate when a corpus holds nothing the
rule addresses -- a declared type not yet used, a scope matching nothing -- and reporting those would
make the rule noise, which is the same as not having it. `type.dir-matches-nothing` already draws the
equivalent line, treating an absent directory as a misdeclaration and an empty one as a type not yet
used, and is the shape to follow: report when the corpus contained something the rule should have
examined and it examined none of it.

**The concrete shape is a second number.** A rule reports what it examined; it cannot say what it was
*eligible* to examine. With both, the signal is `eligible > 0 && examined == 0` rather than
`examined == 0`:

| rule | eligible population | zero examined means |
|---|---|---|
| `relation.supersession-reciprocity` | records carrying a supersession field | the corpus has none -- clean |
| `filename.title-consistency` | records whose filename yields an identifier | the corpus has records and none yield one -- `BUG-61` |

So each rule declares its population predicate, not only its count. `RuleExecution` already carries
`records_examined` and `scope` (`BUG-81`, `BUG-83`); `records_eligible` is the missing third.

`BUG-84` is the evidence for doing it here rather than approximating it elsewhere. Guarding on
`examined > 0` alone made `audit` unsatisfiable on exactly the configuration `init` generates, while
guarding on "a record-scoped rule ran" -- the form shipped -- accepts a rule that could not address
anything. Both are proxies for this distinction, and each is wrong in one direction.

## Why

Roughly twenty instances of this defect were found across seven review rounds of the 0.4.0 diff, and
**every one was visible in the tool's own output when it shipped**. `records_examined` was introduced
precisely to distinguish a rule that ran over nothing from one that ran cleanly, and nothing has ever
read it.

Rounds 2, 4, 6 and 7 each found a defect introduced by the previous round's fix. Review catches this
class, but only after shipping and at the cost of a full adversarial pass each time. A rule that
reads the table turns that into a check the tool runs on itself -- which is Phase 0's thesis, the
engine bootstrapped with the engine, applied to the engine's own blind spot.

Filed now rather than later because the rate is not falling: round 7 found three instances in code
round 6 wrote to fix two others.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
