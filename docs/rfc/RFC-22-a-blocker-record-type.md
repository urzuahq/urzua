---
Stable-Id: 01M21DVCZ1P6XRJX0SJAX9MBYE
Status: Draft
Date: 2026-09-08
Author: '@beauwilliams'
---
# 22 — A blocker record type

## Summary

Proposal: a dedicated `blocker` record type, so `Blocked-on` could become a clean, structured
pointer (like `Implements`/`Derives-from`/`Parent`) instead of the free-text-plus-optional-reference
hybrid SPEC-6 documents today.

## Motivation

Raised live, discussing why `Blocked-on` is deliberately excluded from `header.pointer-field-clean`
(BUG-7): if the *reason* a milestone is blocked always lived in its own record, `Blocked-on` itself
could hold nothing but a clean pointer to it, closing the exact asymmetry that rule's exclusion
depends on.

Separately, and worth weighing on its own: a structured blocking relationship is arguably a
genuinely powerful, generic engine capability, not just a schema-cleanliness nicety. `Blocked-on`
already has real mechanical teeth today -- `blocked-on.stale` (ADR-42/MILE-83) checks whether a
cited blocker has reached a terminal status and flags it as a staleness signal. A first-class
`blocker` record could be a richer foundation for that same idea: its own lifecycle, its own
resolution condition as a real field rather than implied by the target's `Status`, possibly multiple
records blocked on the same blocker without duplicating the reasoning in each one's own prose. That
argument doesn't depend on today's 10-value corpus at all -- it's about what the mechanism could do
generally, independent of how much this repo's own corpus currently uses it.

## Proposal

A `blocker` record type: `dir`, a required field or two (what's blocking, maybe a resolution
condition), `Blocked-on` on other types becomes a plain reference to it. Same mechanism as any other
configured type -- zero `urzua-core` changes, matching `milestone`/`bug`/`waiver`'s own precedent.

## Open questions

- **Does today's volume justify it?** Checked the real corpus: 10 real `Blocked-on` values exist
  across every milestone, and only one (`MILE-38`'s: `Blocked-on: MILE-38 (staleness detection for
  code comments citing an amended record) -- deliberately sequenced first so that...`) carries real
  narrative; the other nine are short, one-line judgment calls (`a decision to actually build it`,
  `real adopter demand for AgDR round-tripping`, `RFC-9's own Q2`). Worth weighing against this
  project's own pattern for `Parent`/`Blocked-on`/`Amends`, each added only once a shape recurred
  three or more times -- but that's a data point for the discussion, not a decision made here.
- **What would the required fields on `blocker` actually be?** Not designed yet -- "what's blocking"
  and "a resolution condition" are placeholders, not a real schema.
- **Would this apply retroactively to the 10 existing values, or only going forward?**

## Non-goals

- Does not propose any change to `BUG-7`'s fix or `Blocked-on`'s exclusion from
  `header.pointer-field-clean` unless/until this RFC is decided -- both stay exactly as shipped for
  now.

## References

- BUG-7 -- the pointer-field-cleanliness fix whose `Blocked-on` exclusion prompted this idea.
- SPEC-6 -- `Blocked-on`'s documented free-text-plus-optional-reference design, unchanged unless
  this RFC is accepted.
- RFC-21 -- a related case (a proposed generic field, checked against real corpus volume) worth
  reading alongside this one, not a verdict on this one.
