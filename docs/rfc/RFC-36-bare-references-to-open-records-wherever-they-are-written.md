---
Stable-Id: 01M2WE01RYCF3FYX9PHV1J45FC
Status: Draft
Date: 2026-09-19
Author: beauwilliams
---
# 36 — Bare references to open records, wherever they are written

## Summary
A reference to a record is a claim to close it only when a closing verb introduces it. Measured over
this repository's own history, **44 of 49** references are bare, so the rule that checks such claims
has reported agreement it never found for almost every file it read. This proposes keeping the verb
requirement and adding a general engine rule for a *bare reference to an open record* -- in whatever
text a repository declares it should look at. This repository would point it at its release
fragments; the engine names no such thing.

## Motivation
`BUG-36` shipped a file announcing a record closed while the record said `Open`.
`claim.status-agreement` was built to stop that, and `BUG-56` later stopped it going inert when its
input was unreachable. It went inert anyway, with its input in hand.

Measured over every release fragment added in this repository's history:

| | Count |
|---|---|
| Files naming at least one record | 19 |
| References phrased as a claim | 5 |
| Bare references (no closing verb) | 44 |

`BUG-66` is the incident: five bug records shipped `Status: Open` in the branch that fixed them, from
a file naming all five as `(BUG-60)`. The rule read that file, examined 2 records, and reported no
findings.

The same measurement rules out the obvious fix. The bare references include `ADR-11`, `SPEC-1`,
`RFC-9`, `ADR-0031` -- citations of the decision a change implements, not claims to close it.
Treating a bare reference as a claim would report a false disagreement for each one, and those are
the records cited most often.

## Proposal
Keep the closing verb as the only thing that makes a claim -- a bare reference stays a citation --
**and** report the case where a bare reference was probably meant as one, so the author is told to
rewrite it rather than left to discover the silence later.

The two halves are not alternatives. The verb requirement is what keeps `claim.status-agreement`
precise enough to be worth having; the report is what stops a change routing around it by accident.
Neither works alone: the verb alone failed in `BUG-66`, and reading bare references as claims would
misread 44 of 49.

**The rule is about references in text, not about any particular kind of file.** A repository declares
which paths carry prose that may reference records; the engine reads those paths and knows nothing
about what the repository calls them. This is the shape `claim.status-agreement` already uses --
`claim_paths` is declared, and the engine does not know the phrase "release fragment" -- and the same
shape is what `BUG-59` and `BUG-65` are about: the engine must not name a repository's own types,
files or vocabulary in its behaviour.

Add a rule -- provisionally `reference.bare` -- that reports when declared text names a record and
**all** of the following hold:

1. the reference is bare (no closing verb introduces it);
2. the record's type is declared *closable* in config, per type, so the engine names no type itself;
3. that record's current `Status` is not in the type's declared closed set.

Options, all declared and none compiled in:

| Option | Meaning |
|---|---|
| `reference_paths` | Path prefixes whose text is scanned for references. |
| `closable_types` | Types for which "still open" is a meaningful state. |
| `closed_statuses` | Per the existing option of the same name. |

The finding states the observation and the phrasing that would resolve it, without deciding which
applies:

> names BUG-60, which is Open. If this change closes it, write `Fixes BUG-60` so the claim is
> checked; if it only cites it, no change is needed.

The three conditions are what keep it quiet. A citation of an accepted ADR fails condition 3. A
reference to a bug already `Fixed` fails it too. What remains is the case where a change plausibly
closes something and said so in a form nothing can check -- which is exactly the incident.

Declared opt-in like every other rule (`ADR-53`), and the level is the adopter's call: `warn` reads as
a prompt to rewrite, `error` makes the rewrite mandatory before the change can merge. This repository
should take `error` -- the whole point is that a warning in a long report is what went unread in
`BUG-66`, and the correction is a four-character edit the author is already positioned to make.

## Open questions
- **Is "open record of a closable type" a good enough discriminator, or does it flag ordinary
  citations of open work?** Text that says "this extends the guard added in BUG-56" while `BUG-56` is
  still open would be flagged and should not be. The measurement above counts references, not intent;
  this needs a pass over the 44 to see how many cite *open* records specifically.
- **Which paths should this repository declare?** Release fragments are the incident, but a commit
  message and a PR body are as likely to carry the claim, and neither is a file the engine can read
  from a path prefix. That may be a different input mechanism rather than a different value.
- **Does `reference_paths` duplicate `claim_paths`?** They would name the same directory here. One
  option read by two rules is simpler; two options let a repository scan a wider set for bare
  references than it trusts for claims.
- **Its own rule, or a second finding from `claim.status-agreement`?** One rule reporting both "you
  claimed something false" and "you may have meant to claim something" is cohesive, but the two want
  different levels, and a declared level applies per rule.
- **Should a bare reference be required to resolve at all?** Two files above name `ADR-0034` and
  `ADR-34` together, and `BUG-0002`/`ADR-0036` in another -- the pre-`ADR-36` identifier shape, which
  no longer resolves against the corpus. Nothing reports those.

## Non-goals
Changing what `claim.status-agreement` does when it *does* parse a claim -- that behaviour is correct
and `BUG-36` is the evidence for it.

Deciding the closing-verb vocabulary. Which verbs introduce a claim is the same declared-vocabulary
question as `BUG-59` and `BUG-65`, and belongs with them under `MILE-98`.

Repairing the 44 historical references. They are consumed at release and are not a corpus to keep.

## References
- `BUG-66`, the incident, and `BUG-36`, the defect `claim.status-agreement` was built for.
- `BUG-56`, which stopped that rule going inert when its input was unreachable.
- `BUG-59` and `BUG-65` on declared vocabulary, both blocked on `MILE-98`. This RFC is the same
  principle applied to which files a rule reads rather than which words it knows.
- `ADR-53`, every rule is a declared policy, opt-in.
