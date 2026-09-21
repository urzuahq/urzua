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

## Why, restated: this is the feedback loop that makes the configuration usable

The original framing below argues from `ADR-55` -- the engine should detect in itself the defect class
it exists to catch. True, and it undersells the milestone.

`ADR-53` decided that *"a repository's governance lives in its configuration and the engine ships only
general-purpose primitives"*, motivated by a foreign corpus receiving nine errors it never asked for.
The `README` puts it as *"a record engine, not a record format: you declare what a record type looks
like, and Urzua validates against that declaration."*

An engine you configure must tell you when your configuration is wrong. When a declared rule produces
nothing there are three causes and three **opposite** responses:

| state | what is true | what the adopter does |
|---|---|---|
| the corpus has none of this yet | legitimate, and common on a fresh adoption | nothing |
| the records are there and unreadable | the corpus is malformed | fix the records |
| the declared scope matches nothing | **the configuration is wrong** | fix the config |

Today all three arrive as one number pair and the adopter cannot tell which applies.

The third row is `BUG-61`: `init` wrote a `prefix` that matched nothing, five rules examined zero
records, and nothing said so. **That failure mode is unique to being configurable** -- a linter with
compiled-in rules cannot have it -- which makes it the most important diagnostic this engine has and
the one it currently lacks.

Read this way, the family is not eight lint bugs. It is the engine having no vocabulary for the
outcomes of its own configuration, surfacing eight times and from both directions: `BUG-84` and
`BUG-99` are a correct config reported as failure, `BUG-61` and `BUG-88` a wrong config reported as
success.

## What the design already specified, and the implementation dropped

The `0.4.0` population work was planned with **four** candidate states:

> `Examined` (verdict reached) · `Absent` (the declared slot or section is not there) · `Unreadable`
> (present, header did not parse) · `OutOfPopulation` (never a candidate, counted nowhere).
> `eligible` counts the first three; `examined` the first. **Keep all four internally so the collapse
> happens once, at serialization.**

What shipped is `Outcome::Examined | NotExamined`, and its doc comment carries the conflation openly:
*"the declared slot is absent, **or** the header it needed did not parse."*

So the field this milestone needs in order to discriminate was specified and then collapsed in the
code rather than at the wire. This is `BUG-40` one level down -- that bug was one number holding three
denominators, fixed by attaching the unit; this is one `NotExamined` holding several causes, and it
needs the reason attached. Every guard attempted so far has been a heuristic reconstructing
information the design said to keep.

**So the first work here is restoring the four states**, not choosing a fifth predicate. The two wire
numbers are unchanged, so nothing in the report contract moves.

It shares a root cause with `RFC-39` one layer up: a type that cannot express a distinction forces
every consumer to guess it.

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
> | 2026-09-21 | Restated the Why as configuration feedback, and recorded that the discriminator was specified and dropped. **Why:** designing the rule stalled because every candidate predicate felt arbitrary, and the reason is that `Outcome` collapsed the plan's four states into two, so the milestone was trying to answer with two states a question the design said needed four. Reframing also corrects the scope: this is not only `ADR-55` self-detection, it is the diagnostic an engine owes an adopter who can misconfigure it -- the failure mode `BUG-61` found and the one a compiled-in linter cannot have. | **substantive** |
