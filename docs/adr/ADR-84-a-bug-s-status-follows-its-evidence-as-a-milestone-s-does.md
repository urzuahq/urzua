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
# 84 — A bug's status follows its evidence, as a milestone's does

## Context

`MILE-183` made a milestone's `Status` answer to evidence: `statuses.delivery` names the computed tiers
that mean the work was delivered, and `status.delivery-agreement` reports a milestone that is not done
while its evidence says it is, and a `Done` milestone with nothing behind it. A bug record has the same
shape of problem and no such rule. A bug is `Fixed` by a change that lands a test, and nothing compares
its `Status` with that test.

The measurement is in this repository's history. Of 167 bugs that reached `Fixed`, 110 were filed already
`Fixed` in the fixing PR, 48 were closed later by a code PR, and 9 by a records-only commit, of which 6
were genuine lag: `BUG-17`, `BUG-57`, `BUG-65`, `BUG-145`, `BUG-178` and `BUG-179`, found by one triage
pass on 2026-11-05 (b031f80). All six fixing PRs named the bug in their title or message, and five of the
six were never named in any release fragment, so a rule that reads changesets cannot see them (`ADR-81`
tried that route and was rejected on the same evidence). That is 6 of 167, 3.6 percent, found by hand once.

A declared signal exists: a bug that is `Open` while its `Regression-test` names a real test. Before the
triage pass, 3 of the 6 (`BUG-65`, `BUG-178`, `BUG-179`) carried one; 3 did not. At HEAD it fires once, on
`BUG-99`, which is a deliberately partial fix and a false positive. So on this repository the recall is 3
of 6 historical cases, with one deliberate false positive. This ADR does not claim more.

The constraints are the engine's: it names no type, status or field (`BUG-59`, `BUG-65`); structured
config in, structured output out (`ADR-64`); every rule is declared and opt-in (`ADR-53`).

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Leave it to review and periodic triage | No engine change | The lag recurs (6 of 167) and was found by one manual pass |
| B. Read prose: a changeset or PR body that names the bug (`ADR-81`'s route) | No new field | Rejected on measurement: 0 real hits among 19 flagged, and 5 of the 6 lagging bugs appear in no changeset |
| C. Extend `statuses.delivery` to any type with declared evidence, so a bug's `Status` is held to the same kind of evidence a milestone's is (chosen) | One mechanism, already built and tested; declared fields only; opt-in per type | Needs a verifiable test locator on the bug (below); recall is partial |
| D. Auto-downgrade a `Fixed` bug with no evidence | Symmetric | Wrong on this repository: documentation fixes and performance fixes have no test (`none -- performance-only`); a downgrade would reopen a correct record |

## Decision

In the context of bug records that stay `Open` after the change that fixes them, facing a measured lag of
6 in 167 that no existing rule reads, we decided to **extend the `statuses.delivery` model to bugs**, to
achieve a bug whose fix has landed being reported before the lag is found by hand, accepting that the
signal is partial and has a known false positive.

- **The delivering PR closes the bug.** This is the fixed-in-PR convention (`BUG-180`, `BUG-186`) stated
  as the rule: the change that fixes a bug sets its `Status` to the declared success status and fills its
  evidence in the same change, as `MILE-183` decided for milestones on 2026-11-04.
- **An `Open` bug with evidence is reported.** A bug whose `Status` is a declared not-done status while
  its declared evidence says the fix landed is a disagreement. Evidence is either of two things the
  repository declares: the bug's regression-test field names a test that exists (below), or a source file
  cites the bug as the thing it fixes through the declared back-pointer (`ADR-67`). The finding names the
  evidence it found. `urzua fix --apply` may set the declared success status, as it does for a milestone,
  with a structural revision row; the suggested action is `edit-field` (`ADR-73`).
- **A `Fixed` bug with no evidence is reported and never downgraded.** The finding says the record claims
  a fix nothing backs. `fix` does not touch it, because the absence of a test is legitimate for a
  documentation or performance fix (option D) and only a human can say which.
- **It is opt-in config with no type knowledge in the engine.** The mapping is the same declaration
  `MILE-183` introduced, `statuses.delivery`, on any type that declares it: the tiers that mean delivered,
  the tiers that mean not delivered, the statuses that mean not done, and the success status a delivered
  record is repaired to. A type that declares none is not examined. The engine hardcodes no `bug`, no
  `Fixed`, no `Regression-test`. Enabling the rule on a type whose evidence field is not a verifiable
  locator is a config load error, not an inert rule (`BUG-44`).
- **A deliberate partial fix is expressible.** `BUG-99` stays `Open` with a test that proves only part of
  it. The repository records that with a waiver (`ADR-11`) or a declared status; the engine does not try to
  tell a partial fix from a complete one. This is the one known false positive and it is stated here, not
  hidden.

**How a test locator is verified to exist.** Today `Regression-test` is free text and its values have
several shapes: a bare test name, a name followed by a file (`..._observed_failing, rules.rs`), a path with
`::`, a count of tests, and `none -- ...`. A value cannot be checked against the repository unless it is
parsed, and the engine must not parse prose (`ADR-64`). The proposal is a declared **test-locator field
kind**: a type names a field whose value is a locator in the grammar `Realized-by` already uses
(`path` or `path#Symbol`, resolved by `locator_resolvers`, `ADR-74`), and the existing existence check
(`embodiment.locator-exists`) verifies it, so a locator naming a file or symbol that is absent is an error
whatever the bug's `Status`. A value that does not parse as a locator is not evidence: it is counted and
disclosed, not guessed at and not reported as a defect. The recall above is therefore an upper bound that
depends on how many existing `Regression-test` values can be rewritten as locators.

## Reversibility

Moderate. The rule is additive and opt-in; a repository that declares `statuses.delivery` on no bug type
is unchanged, and `MILE-183`'s keys are not renamed. What is harder to take back is the test-locator field
kind once records are migrated to it, because rewriting a field's values across a corpus is a mass edit.
The `fix --apply` write is reversible by a normal revert.

## Consequences

- `SPEC-3` gains the test-locator field kind and says that `statuses.delivery` applies to any type;
  `SPEC-2` gains the finding wording for a bug. Each gets a revision row.
- This repository declares `statuses.delivery` on `bug` and migrates its `Regression-test` values that
  name a real test to locators; the rest stay as disclosed non-evidence. Part of that migration is judgement
  and should be reviewed, not scripted.
- A bug whose value is `none -- performance-only` or `No automated test exists` is neither flagged nor
  reported as a `Fixed` bug without evidence unless the repository declares that value as evidence-free; the
  engine does not know the words.
- The rule would have caught 3 of the 6 known lagging bugs. The other 3 had no test when the lag began, and
  only the declared source back-pointer could have caught them. That is a limit of the signal, not a gap to
  paper over.
- `BUG-99` is flagged on the first run. It needs a waiver or a status in the same change that enables the rule.

## Open questions

1. Should the test locator be a new field kind on `Regression-test`, or should a bug's evidence be read
   from `Realized-by` with a `test:` locator, which already exists and is verified? The first keeps the
   bug template; the second adds no kind but changes where authors write the test.
2. Does a back-pointer in code (`ADR-67`) alone count as delivery evidence for a bug, or only a test?
   A cited fix without a test is exactly the case a `Fixed` bug with no evidence reports.
3. How is a deliberate partial fix such as `BUG-99` recorded without a waiver per bug?

## References

- `MILE-183` -- the delivery model this extends; `ADR-81` -- the prose route, rejected on measurement.
- `BUG-180`, `BUG-186` -- the fixed-in-PR convention; `BUG-99` -- the known false positive.
- `ADR-64` -- structured in, structured out; `ADR-53` -- declared, opt-in; `ADR-73` -- suggested actions.
- `ADR-67` -- source back-pointers; `ADR-74` -- locator resolvers and pins; `ADR-11` -- waivers.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** maintainer decision of 2026-11-05, after the review of `ADR-81` found that the recurring failure is a fixed bug left `Open` (6 of 167) and not a bare reference in a changeset; the maintainer approves before it is Accepted. | **substantive** |
