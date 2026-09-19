---
Stable-Id: 01M2WE01RYCF3FYX9PHV1J45FC
Status: Draft
Date: 2026-09-19
Author: beauwilliams
---
# 36 — Should a bare reference in a changeset be read as a claim

## Summary
`claim.status-agreement` reads a reference as a claim to close only when a closing verb introduces
it. Across this repository's whole changeset history, **44 of 49** references are bare, so the rule
has reported agreement it never found for almost every changeset ever written. This proposes that a
bare reference stay a citation -- reading it as a claim would misread the majority of them -- and that
the tool instead report a bare reference to an open record of a closable type, naming the phrasing
that would make it a claim.

## Motivation
`BUG-36` shipped a changeset announcing a record closed while the record said `Open`.
`claim.status-agreement` was built to stop that, and `BUG-56` later stopped it going inert when its
input was unreachable. It went inert anyway, with its input in hand.

Measured over every changeset added in this repository's history:

| | Count |
|---|---|
| Changesets naming at least one record | 19 |
| References phrased as a claim | 5 |
| Bare references (no closing verb) | 44 |

`BUG-66` is the incident: five bug records shipped `Status: Open` in the branch that fixed them, with
a changeset naming all five as `(BUG-60)`. The rule read that changeset, examined 2 records, and
reported no findings.

The same measurement rules out the obvious fix. The bare references include `ADR-11`, `SPEC-1`,
`RFC-9`, `ADR-0031` -- citations of the decision a change implements, not claims to close it.
Treating a bare reference as a claim would report a false disagreement for each one, and those are
the records cited most often.

## Proposal
A bare reference remains a citation. Add a rule -- provisionally `claim.unphrased` -- that reports
when a changeset names a record and **all** of the following hold:

1. the reference is bare (no closing verb introduces it);
2. the record's type is *closable* -- declared per type in config, so the engine names no type itself
   (`BUG-59`);
3. that record's current `Status` is not in the type's closed set.

The finding states the observation and the phrasing that would resolve it, without deciding which
applies:

> `round-6-fixes.md` names BUG-60, which is Open. If this change closes it, write `Fixes BUG-60` so
> the claim is checked; if it only cites it, no change is needed.

The three conditions are what keep it quiet. A citation of an accepted ADR fails condition 3. A
reference to a bug that is already `Fixed` fails it too. What remains is the case where a change
plausibly closes something and said so in a form nothing can check -- which is exactly the incident.

Declared opt-in at `warn` like every other rule (`ADR-53`).

## Open questions
- **Is "open record of a closable type" a good enough discriminator, or does it flag ordinary
  citations of open work?** A changeset that says "this extends the guard added in BUG-56" while
  `BUG-56` is still open would be flagged and should not be. The measurement above does not separate
  these cases; it counts references, not intent. This needs a pass over the 44 to see how many are
  citations of *open* records specifically.
- **Should the rule look at the commit message and PR body too, or only the changeset?** The claim
  that closes a record is as likely to be written there, and neither is currently read.
- **Does this belong as its own rule or as a second finding from `claim.status-agreement`?** One rule
  reporting both "you claimed something false" and "you may have meant to claim something" is
  cohesive, but the two have different severities and an adopter may want only one.
- **Should a bare reference in a changeset be required to resolve at all?** Two entries above name
  `ADR-0034` and `ADR-34` in the same file, and `BUG-0002`/`ADR-0036` in another -- the pre-`ADR-36`
  identifier shape, which no longer resolves against the corpus. Nothing reports those.

## Non-goals
Changing what `claim.status-agreement` does when it *does* parse a claim -- that behaviour is correct
and `BUG-36` is the evidence for it.

Deciding the closing-verb vocabulary. Which verbs introduce a claim is the same declared-vocabulary
question as `BUG-59` and `BUG-65`, and belongs with them under `MILE-98`, not here.

Rewriting the 44 historical changesets. They are consumed at release and are not a corpus to repair.

## References
- `BUG-66`, the incident, and `BUG-36`, the defect the rule was built for.
- `BUG-56`, which stopped this rule going inert when its input was unreachable.
- `BUG-59` and `BUG-65` on declared vocabulary, both blocked on `MILE-98`.
- `ADR-53`, every rule is a declared policy, opt-in.
