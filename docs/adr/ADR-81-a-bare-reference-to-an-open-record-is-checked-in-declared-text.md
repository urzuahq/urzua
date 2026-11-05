---
Stable-Id: 01M6N3D1YNWMGXWRY89XHY1XPJ
Status: Rejected
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: RFC-36
---
# 81 — A bare reference to an open record is checked in declared text

## Context

`claim.status-agreement` recognizes a claim to close a record only when a closing verb introduces the
reference. `BUG-66` is the incident: five bug records shipped `Status: Open` in the branch that fixed
them, from a changeset naming each as `(BUG-60)`. The rule read the file, examined 2 claim inputs and
reported no findings.

`RFC-36` measured the obvious fix and ruled it out: of 49 references in this repository's changeset
history, 44 are bare, and they include `ADR-11`, `SPEC-1` and `RFC-9`, citations of the decision a change
implements. Reading a bare reference as a claim would report a false disagreement for most of them. The
maintainer decided on 2026-11-05 to accept `RFC-36`, which keeps the verb requirement and adds a report
for the case where a bare reference was probably meant as a claim. `BUG-66` has been blocked on that
decision.

The decision has to hold two engine principles: the engine knows no repository's file names or
vocabulary (`BUG-59`, `BUG-65`), and every rule is a declared, opt-in policy (`ADR-53`).

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Treat a bare reference as a claim | Closes the silence | Reports a false disagreement for the 44 of 49 that are citations; destroys the rule's precision |
| B. Report only that the rule found no claims | Cheap, no new policy | Says nothing about a bare reference that should have been a claim; the author still discovers it late |
| C. Keep the verb requirement and add a rule reporting a bare reference to an open record in declared text (chosen) | Precision kept; the incident is caught; the paths and types are the repository's declaration | A new rule with an unmeasured false-positive rate on citations of open work |
| D. Make `claim.status-agreement` emit a second finding kind | One rule | A declared level applies per rule, and "you claimed something false" and "you may have meant to claim" want different levels |

## Decision

In the context of a rule that recognizes a claim only by a closing verb, facing a changeset that named
five open records bare and passed, we decided to **keep the closing-verb rule unchanged and add a
separate opt-in rule that reports a bare reference to an open record in text the repository declares**,
to achieve a prompt to rewrite the reference before the silence is discovered later, accepting that a
citation of open work will sometimes be flagged.

- **`claim.status-agreement` is unchanged.** A claim is still introduced only by a closing verb. What
  it does when it parses a claim is correct (`BUG-36`) and the verb vocabulary stays with `MILE-98`.
- **A new rule, provisionally `reference.bare`, is its own rule.** It is not a second finding from
  `claim.status-agreement`, because the two want different levels and a level is declared per rule.
- **It reports a reference when all three hold:** the reference is bare (no closing verb introduces it);
  its record's type is declared closable; and the record's current `Status` is not in the declared
  closed set. A citation of an Accepted ADR, or of a bug already Fixed, fails the third condition.
- **Everything the repository names is declared.** `reference_paths` (path prefixes whose text is
  scanned), `closable_types` (types for which still open is meaningful) and `closed_statuses`. The engine
  knows no type, file kind or status by name (`ADR-64`, `ADR-53`). Declaring none enables nothing, and a
  declared path that covers no file is reported, not clean.
- **This repository declares its changeset files** (`.changeset`) as the scanned text, and its bug and
  milestone types as closable.
- **The finding states the observation and the fix without deciding which applies:** it names the record,
  says it is open, and says that if the change closes it the reference should be rewritten with a closing
  verb, and that if it only cites it no change is needed.
- **It ships at `warn`.** Promotion to `error` is a config edit, made once the false-positive rate on the
  44 existing references is measured and the discriminator holds. `error` is where it should end up
  (`RFC-36`).

**Questions `RFC-36` leaves open, and where this ADR lands, for the maintainer before Accepted:**

1. **Is open-record-of-a-closable-type a good enough discriminator?** Not settled. It will flag "this
   extends the guard added in BUG-56" while `BUG-56` is open. The decision is to ship at `warn` and
   measure, not to claim the discriminator is right. The pass over the 44 references to count citations of
   open records is still to be done and gates promotion to `error`.
2. **Which paths?** Decided for this repository: changeset files. A commit message or PR body is not a
   file a path prefix reads, so those are not covered and are not decided here; they may want a different
   input mechanism.
3. **`reference_paths` or `claim_paths`?** Decided as separate options, as `RFC-36` lists them, so a
   repository can scan a wider set for bare references than it trusts for claims. Both name the same
   directory here. The maintainer may prefer one shared option.
4. **Its own rule, or a second finding?** Decided as its own rule (above).
5. **Must a bare reference resolve at all?** Not decided. The pre-`ADR-36` identifier shape that no longer
   resolves stays unreported by this rule.
6. **The rule's final id.** `reference.bare` is provisional (`RFC-36`); the maintainer may rename it
   before it ships.

## Reversibility

Easy. The rule is additive and opt-in, `claim.status-agreement` does not change, and removing it costs a
config key. What is harder to take back is the `warn` to `error` promotion once other repositories rely on
it, which is why it is not made here.

## Consequences

- `SPEC-2` gains the rule and `SPEC-3` its three options, each with a revision row.
- `BUG-66` is unblocked by this decision. Its fix is the rule plus a regression test that a changeset
  naming an open record bare produces a finding.
- A changeset naming an open record bare now gets a warning in this repository, and the author rewrites it
  or leaves it as a citation.
- The false-positive rate is an unmeasured cost until the pass over the 44 references is done.
- `RFC-36` is amended to record the choices above; its `Status` is the maintainer's to change.

## References

- `RFC-36` -- the proposal this accepts; amended with the choices above.
- `BUG-66` -- the incident; `BUG-36` and `BUG-56` -- the rule's earlier defects.
- `BUG-59`, `BUG-65`, `MILE-98` -- declared vocabulary.
- `ADR-53` -- every rule is a declared, opt-in policy; `ADR-64` -- structured config.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** maintainer decision of 2026-11-05 to accept `RFC-36`, which `BUG-66` is blocked on; building part of an RFC needs its own decision record, and the maintainer approves before it is Accepted. | **substantive** |
> | 2026-11-05 | Rejected by the maintainer. Measured over this repository: of the 19 bare references to an open bug or milestone in the fragment history, 0 were claims to close, and at HEAD the rule gives 6 false findings under `fail_on: warning`, which would turn CI red. The original incident (`BUG-66`) was caught in review in about 70 minutes and never released. The recurring failure the rule was meant for is a fixed bug left `Open` (`ADR-84`), and a claim should be a declared field, not a reading of prose (`ADR-85`). **Why:** maintainer decision of 2026-11-05 on that evidence; a rule that reads prose to infer intent has no precision to build on. | **substantive** |
