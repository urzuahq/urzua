---
Stable-Id: 01M6J25TBPS0M0ESR5MW9MBV71
Status: Proposed
Date: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Derives-from: RFC-5
---
# 66 — A source comment citing a decision counts as realization evidence

## Context

`RFC-5` section 2 chose the record-side pointer (`Realized-by`) as the universal addressing
mechanism and set aside the in-code `Implements: ADR-N` comment as the way to *discover* what
governs a file, on three grounds: a comment only proves a string was typed, it needs a ritual no
linter enforces, and it cannot reach decisions realized outside application code. `RFC-28` restates
that and does not reopen it.

`MILE-128` (decision recorded 2026-11-04) separates two questions the earlier rejection ran
together. Discovering what governs a file from its comments stays rejected. Accepting a comment as a
claim the record already makes is a different question, and the first adopter relies on such comments
across several hundred records, so without a place for them in the evidence model that adopter keeps
a parallel mechanism and the replacement bar (`MILE-102`) is not met.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Leave comments outside the evidence model | Nothing to build; `RFC-5` section 2 untouched | The adopter keeps a parallel linter; `MILE-102` stays unmet |
| B. Comments count as code-tier evidence, dangling ones reported (chosen) | Reuses the existing tiers; no new tier; fails visibly on a typo | A comment is still only a claim; the tier can overstate |
| C. Comments count as `Verified` evidence | Higher credit for a comment | A comment is weaker than a test; contradicts the tier order |

## Decision

In the context of an adopter whose source files carry `Implements: ADR-N` comments, facing the
choice between ignoring them and crediting them, we decided that **a whole-line source comment
`Implements: <record id>[, <record id>...]`, in a file under a declared `back_pointer_paths` entry, counts
as code-tier realization evidence for each record it cites**, and that **a comment citing a record
that does not exist is reported** by the opt-in rule `embodiment.back-pointer-resolution`. We chose
this to give an existing practice a place in the evidence model, accepting that a comment remains an
unverified claim.

This narrows `RFC-5` section 2 and does not edit it. The pieces of that section that stand:

- The record-side pointer stays the primary mechanism; `Realized-by` is unchanged.
- A comment is read only to credit the record it names. Nothing here infers what governs a file
  from its comments.
- A comment is code-tier at most: it lifts `Not started` or `Specified` to `Implemented`, never
  outranks a `test:` locator, and never overrides `Drift detected`.

Narrowest reading, as built: only a comment line that starts with a comment leader and then the
case-sensitive marker `Implements:` is read (a trailing comment after code is not); reading stops at
the first token that is not a declared-prefix record id; ids compare numerically. Evidence is
collected only while the rule is enabled, from the `back_pointer_paths` option, the same list that scopes
the dangling check; `urzua fix` reads the same evidence so the two never disagree.

## Reversibility

Cheap. The rule is opt-in and the option is new, so removing the rule returns every record to its
`Realized-by`-computed tier and changes nothing for a repository that never enabled it. Only an
adopter who enabled it and stated `Implemented` on the strength of comments alone would see
`embodiment.consistency` disagree again.

## Consequences

- A record can reach `Implemented` with no `Realized-by` code locator, on a comment alone. That is
  the intent, and it is the weaker claim: `embodiment.consistency` still judges only records that
  carry both `Embodiment` and `Realized-by`, so a record with neither is not credited.
- `RFC-5`'s objection that a comment proves only that a string was typed still holds; the dangling
  check catches a renamed or mistyped id, not a comment that has gone stale against the code.
- `Realized-by` locators and comments are two sources for one tier. They are not reconciled: a
  comment citing a record whose own locators name other files raises no finding.
- Test files cannot earn `Verified` through a comment. Crediting a comment in a test directory at the
  `test` tier is a possible follow-up, not decided here.
- The marker is fixed at `Implements:`. A configurable marker is a follow-up if a second adopter
  uses another spelling.

## References

- `RFC-5` section 2 — the pointer lives on the record; narrowed here.
- `RFC-28` — restates the section 2 decision; not reopened.
- `MILE-128` — the decision this implements; `MILE-102` — the adopter replacement it serves.
- `SPEC-2` — the rule table and the `back_pointer_paths` option.
