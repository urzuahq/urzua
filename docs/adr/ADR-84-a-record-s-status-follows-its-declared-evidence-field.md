---
Stable-Id: 01M6N6PC6Y79X69H1AKA01RA9K
Status: Proposed
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: —
---
# 84 — A record's status follows its declared evidence field

## Context

`MILE-183` made a milestone's `Status` answer to evidence: `statuses.delivery` names the computed tiers
that mean the work was delivered, and `status.delivery-agreement` reports a record that is not done while
its evidence says it is, and a done record with nothing behind it. The mechanism is already per declared
type. What it reads, though, is the embodiment machinery (`Realized-by` and source back-pointers). A type
whose evidence is a different declared field, a test locator for instance, has no way to say so, and its
status is held to nothing.

urzua is a general-purpose engine. A record type called `bug` is only what this repository declares; another
team may declare none, or call the same thing something else. This decision is therefore phrased over
declared types and fields, and the engine names no type, status or field.

This repository is the measured example. Its declared defect type has the problem: of 167 such records that
reached their success status, 110 were filed already in it by the fixing PR, 48 were closed later by a code
PR, and 9 by a records-only commit, of which 6 were genuine lag (`BUG-17`, `BUG-57`, `BUG-65`, `BUG-145`,
`BUG-178`, `BUG-179`, found by one triage pass on 2026-11-05, b031f80). All six fixing PRs named the record
in their title or message, and five of the six appear in no release fragment, so a rule that reads
changesets cannot see them (`ADR-81`, rejected on the same evidence, tried that). That is 6 of 167, 3.6
percent, found by hand once.

A declared signal exists: a record in a not-done status whose evidence field names a test that exists.
Before the triage pass, 3 of the 6 (`BUG-65`, `BUG-178`, `BUG-179`) carried one; 3 did not. At HEAD it fires
once, on `BUG-99`, a deliberately partial fix and a false positive. On this repository the recall is 3 of 6
historical cases, with one deliberate false positive. This ADR claims no more.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Leave it to review and periodic triage | No engine change | The lag recurs (6 of 167) and was found by one manual pass |
| B. Read prose: a changeset or PR body that names the record (`ADR-81`) | No new field | Rejected on measurement: 0 real hits among 19 flagged |
| C. Let `statuses.delivery` read a declared evidence field of any type, verified to exist (chosen) | One mechanism, already built; declared fields only; opt-in per type | Needs a verifiable locator in that field; recall is partial |
| D. Add a `Realized-by` to the type so the existing evidence applies | No new field kind | Adds a second evidence field to a type that already has one, and changes where authors write the test |
| E. Add a "partially fixed" status | Expresses `BUG-99` | A status the engine would have to know; the waiver mechanism already expresses a deliberate exception |
| F. Auto-downgrade a done record with no evidence | Symmetric | Wrong where a done record legitimately has none (a documentation or performance change) |

## Decision

In the context of records whose status lags the change that delivers them, facing a measured lag of 6 in
167 that no rule reads, we decided to **let a type declare an evidence field and hold the record's `Status`
to it, as `statuses.delivery` already does for milestones**, to achieve a delivered record being reported
before the lag is found by hand, accepting that the signal is partial and has a known false positive.

- **The evidence is a declared locator field.** A type names a field as a *test-locator field*: its value is
  a locator in the grammar `Realized-by` already uses (`path` or `path#Symbol`, resolved by
  `locator_resolvers`, `ADR-74`). The existing `embodiment.locator-exists` verifies it, so a locator naming
  a file or symbol that does not exist is an error whatever the record's status. This repository declares
  `Regression-test` this way. No `Realized-by` is added to that type. A value that does not parse as a
  locator is not evidence: it is counted and disclosed, never guessed at and never reported as a defect.
- **The delivering PR sets the status.** The change that delivers a record sets its declared success status
  and fills its evidence field in the same change (the fixed-in-PR convention, `BUG-180`, `BUG-186`;
  `MILE-183` for milestones).
- **A not-done record with evidence is reported.** A record whose `Status` is a declared not-done status
  while its evidence field resolves is a disagreement. The finding names the evidence. `urzua fix --apply`
  may set the declared success status with a structural revision row; the suggested action is `edit-field`
  (`ADR-73`).
- **A done record with no evidence is reported and never downgraded.** `fix` does not touch it: absence of
  evidence is legitimate for some changes, and only a human can say which.
- **Whether a source back-pointer alone counts is a config option.** A type may declare that a citing source
  back-pointer (`ADR-67`) is delivery evidence without a test locator. The default is that only the declared
  locator field counts.
- **A deliberately still-open record with evidence is a waiver, not a status.** There is no "partial"
  status. `BUG-99` is expressed with the existing waiver record (`ADR-11`), scoped to that record and this
  rule, with a reason and an expiry. The stale-waiver rule keeps it honest: when the record is closed or the
  expiry passes, the waiver is reported.
- **It is opt-in config with no type knowledge in the engine.** The mapping is `statuses.delivery`, on any
  type that declares it. The engine hardcodes no type, status or field name. A type that declares none is
  not examined. Enabling the rule on a type whose evidence field is not declared as a test-locator field is
  a config load error, not an inert rule (`BUG-44`).

## Reversibility

Moderate. The rule is additive and opt-in; a repository that declares it on no type is unchanged, and
`MILE-183`'s keys keep their meaning. What is harder to take back is the test-locator field kind once
records are migrated to it, because rewriting a field's values across a corpus is a mass edit. The
`fix --apply` write is reversible by a normal revert.

## Consequences

- `SPEC-3` gains the test-locator field kind and the back-pointer option, and says `statuses.delivery`
  applies to any type; `SPEC-2` gains the finding wording. Each gets a revision row.
- This repository declares `statuses.delivery` on its defect type and migrates the `Regression-test` values
  that name a real test to locators; the rest stay as disclosed non-evidence. Part of that migration is
  judgement and should be reviewed, not scripted.
- A value such as `none -- performance-only` is neither evidence nor reported unless the repository
  declares it evidence-free; the engine does not know the words.
- The rule would have caught 3 of the 6 known lagging records. The other 3 had no test when the lag began;
  only the back-pointer option could have caught them. That is a limit of the signal.
- `BUG-99` is flagged on the first run, so the change that enables the rule also adds its waiver.

## Open questions

1. Is the locator grammar enough for a test written as a name plus a file (`..._observed_failing, rules.rs`),
   or does the migration need a form for several tests per record?
2. Should the back-pointer option be per type or per repository?

## References

- `MILE-183` -- the delivery model this extends; `ADR-81` -- the prose route, rejected on measurement.
- `BUG-180`, `BUG-186` -- the fixed-in-PR convention; `BUG-99` -- the known false positive.
- `ADR-64` -- structured in, structured out; `ADR-53` -- declared, opt-in; `ADR-73` -- suggested actions.
- `ADR-67` -- source back-pointers; `ADR-74` -- locator resolvers; `ADR-11` -- waivers.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** maintainer decision of 2026-11-05, after the review of `ADR-81` found that the recurring failure is a delivered record left in a not-done status (6 of 167) and not a bare reference in a changeset; the maintainer approves before it is Accepted. | **substantive** |
> | 2026-11-05 | Amended while Proposed: retitled and rephrased over declared types and fields, since a record type like `bug` is only what this repository declares; the evidence is a declared test-locator field (no `Realized-by` is added); a back-pointer alone counting is a config option, default off; a deliberately open record with evidence is a scoped, expiring waiver, not a status. **Why:** maintainer answers of 2026-11-05 to the open questions. | **substantive** |
