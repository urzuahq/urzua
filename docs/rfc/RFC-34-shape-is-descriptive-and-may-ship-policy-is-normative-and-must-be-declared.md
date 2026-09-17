---
Stable-Id: 01M2Q888BH2YGD478KT9SDNMWG
Status: Draft
Date: 2026-09-17
Author: beauwilliams
Supersedes / Superseded-by: —
---
# 34 — Shape is descriptive and may ship; policy is normative and must be declared

## Summary

`ADR-53` decided that a repository's governance is its configuration, not the engine's binary. Taken
without qualification that reads as *nothing may ship*, which would mean every adopter hand-writes a
parser for their own format before the tool can read a single file.

Propose the distinction that makes both true at once: **shape is descriptive and may ship as a
built-in; policy is normative and must be declared.**

## Motivation

Three words have been doing one job, and the confusion is now costing decisions:

| | what it is | it answers | example |
|---|---|---|---|
| **corpus** | evidence -- somebody's actual records | *does this work?* | `npryce/adr-tools`, MADR, Python PEPs |
| **format** | how a file is shaped | *where is the data?* | fields in YAML frontmatter / bare `Date:` prefix lines / RFC-822 headers |
| **policy** | what must be true | *is this acceptable?* | `Status` must be one of a declared set |

*"MADR puts its fields in YAML frontmatter"* is **true whether or not anyone likes it**. *"A MADR
record must carry Decision Drivers"* is a choice someone made. The first is a parser. The second is
governance.

`ADR-53` is about the second. It says the engine must not hold opinions an adopter cannot decline --
and a fact about where a file keeps its fields is not an opinion. There is nothing to decline.

## Proposal

**A shape may ship as a named built-in.** `fields.from = "madr"` is shorthand for a declaration the
adopter could have written by hand, and can still override field by field. It is a parser under a
name, and names are how a parser becomes discoverable.

**A policy may never ship.** Not as a default, not as a category, not as an always-on rule. `MILE-80`
already established the mechanism -- a rule not named in `rules` does not run -- and `MILE-95`'s
`init --preset` generates a config rather than supplying one at check time, so neither is an
exception to this.

**The test for which a thing is: could a reasonable adopter disagree with it and still be using this
format?**

- *"Fields live in YAML frontmatter"* -- no. Disagreeing means it is a different format.
- *"Status must be Accepted before a spec may implement it"* -- yes, easily. Policy.

**A shape earns a name from a corpus, never from anticipation.** `AGENTS.md` prohibits speculative
capability; a built-in shape nobody has tested against real records is exactly that. Each named shape
cites the corpus it was derived from and the date it was checked.

### The failure mode this exists to prevent

A shape that smuggles policy. If `fields.from = "madr"` also quietly required `decision-makers`, it
would be `ADR-53`'s defect wearing a parser's clothes -- an undeclared opinion arriving with something
an adopter had no choice but to accept. **A named shape may declare where things are. It may never
declare that something must be there.**

The distinction is checkable, which is the point: a built-in shape that emits a finding is a bug, not
a feature. Shapes locate. Rules judge.

## Open questions

- **Does a shape compose?** MADR is "YAML frontmatter, `##` sections, `NNNN-slug.md`" -- three
  declarations that happen together. Whether `madr` is one name or sugar for three is unsettled, and
  the answer probably follows from whether any real corpus wants two of the three.
- **What is the minimum evidence for a name?** One corpus is thin. This RFC's own claim -- that a
  small vocabulary covers decision records generally -- is what the third-corpus test is meant to
  falsify, and the same standard should apply here.
- **Does `identity` belong to shape or policy?** `^(?P<number>\d+)-(?P<slug>.+)$` is descriptive. *"A
  record must be numbered"* is not. They are currently the same declaration.

## Non-goals

Does not decide the function vocabulary -- that is `RFC-33`, and `ADR-53` deliberately left it open
pending a third corpus. Does not propose any specific built-in shape. Names the principle by which one
would be admitted.

## References

- ADR-53 -- governance is configuration; this qualifies what that does and does not reach.
- RFC-33 -- the declared document model these shapes would name.
- MILE-51 -- the first foreign corpus, and why the engine could not read it at all.
- MILE-95 -- `init --preset`, a generator rather than a runtime default, consistent with this.
- AGENTS.md -- speculative capability, and why a shape needs a corpus behind it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Filed, `Status: Draft`. **Why:** `ADR-53` read without qualification forbids shipping anything, which would make every adopter write a parser before the tool reads a file. Raised while discussing a third reference corpus: supporting many formats with built-ins needs a rule for what may ship, and "descriptive may, normative may not" is the line that keeps `ADR-53` intact. | **substantive** |
