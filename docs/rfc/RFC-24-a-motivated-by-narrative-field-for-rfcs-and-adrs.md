---
Stable-Id: 01M23CESRZYX7K4D1XE5JGSD62
Status: Draft
Date: 2026-09-09
Author: '@beauwilliams'
---
# 24 — A Motivated-by narrative field for RFCs and ADRs

## Summary

Scope out, without building, a `Motivated-by` field as a candidate future `narrative_fields` entry
(RFC-23/ADR-44) for `rfc`/`adr`: free text naming what motivated a proposal, tolerating an embedded
reference to a record whose own staleness would be worth flagging.

## Motivation

Filed as a direct side effect of resolving four review gaps in RFC-23/ADR-44, not from any real
case in this corpus yet. That amendment settled that every `narrative_fields` entry, for any field
declared that way, is checked against its target's terminal status by the same mechanism
`blocked_on_stale` generalizes to -- `Motivated-by` was the example used to argue `narrative_fields`
should be named for the mechanism, not `blocking_fields`, since a motivation pointer isn't about
blocking at all. That argument doesn't require the field to exist; it only requires the *name*
`narrative_fields` to not misdescribe it if it ever did. This RFC exists so that "should `Motivated-
by` actually be built" gets asked and answered on its own evidence, rather than smuggled in as an
unexamined side effect of RFC-23's naming argument.

No incident or measurement motivates this today. That absence is itself the reason this stays a
separate, low-weight RFC instead of scope inside RFC-23/ADR-44 or MILE-90 -- per this project's own
"add a field once a pattern recurs" discipline (RFC-23's own Non-goals), a field with zero real
cases behind it doesn't get built, only named as a candidate.

## Proposal

Not proposed for building here. If a real case emerges (a record whose Context section already
names "motivated by X" in prose, the way `Blocked-on`'s real cases predated its own field), the
shape would follow `Blocked-on`'s precedent directly: `Motivated-by` declared in `rfc`/`adr`'s
`narrative_fields`, free text tolerating an embedded reference, resolved by `pointer_resolution`,
checked for target terminal-status by the same generalized staleness rule.

## Open questions

- **Does a real case exist yet?** Not researched here -- this RFC exists to name the candidate, not
  to audit the corpus for prose that already expresses this informally (the way `Blocked-on`'s
  audit found real, if sparse, live usage before RFC-23 generalized the mechanism).
- **Is `Motivated-by` the right name**, if a real case does turn up? Not settled -- filed under
  today's working name only.

## Non-goals

- **Does not build anything.** No config change, no code change, no corpus migration. Status stays
  `Draft` until a real case is found and someone decides to move on it.

## References

- RFC-23 -- the amendment that used `Motivated-by` as its naming argument for `narrative_fields`
  and deferred actually building it to this RFC.
- ADR-44 -- the decision RFC-23 realizes; its own Context section already frames `narrative_fields`
  as "staleness-aware, prose-tolerant pointers," the shape this RFC would reuse unchanged.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-09 | Initial RFC, `Status: Draft`. **Why:** RFC-23's amendment used `Motivated-by` as a naming argument without deciding whether to build it -- scoped out here so that decision happens on its own evidence. | **structural** |
