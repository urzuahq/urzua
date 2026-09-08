---
Stable-Id: 01M21DVCZ1P6XRJX0SJAX9MBYE
Status: Rejected
Date: 2026-09-08
Author: '@beauwilliams'
---
# 22 — A blocker record type -- deferred, real volume too low to justify it yet

## Summary

Proposal: a dedicated `blocker` record type, so `Blocked-on` could become a clean, structured
pointer (like `Implements`/`Derives-from`/`Parent`) instead of the free-text-plus-optional-reference
hybrid SPEC-6 documents today. Checked against the real corpus before deciding anything, and
rejected for now: volume doesn't justify it.

## Motivation

Raised live, discussing why `Blocked-on` is deliberately excluded from `header.pointer-field-clean`
(BUG-7): if the *reason* a milestone is blocked always lived in its own record, `Blocked-on` itself
could hold nothing but a clean pointer to it, closing the exact asymmetry that rule's exclusion
depends on.

## Proposal (rejected)

A `blocker` record type: `dir`, a required field or two (what's blocking, maybe a resolution
condition), `Blocked-on` on other types becomes a plain reference to it. Same mechanism as any other
configured type -- zero `urzua-core` changes, matching `milestone`/`bug`/`waiver`'s own precedent.

## Why rejected

Checked the real corpus before building anything: **10 real `Blocked-on` values exist across every
milestone**, and only **one** (`MILE-38`'s: `Blocked-on: MILE-38 (staleness detection for code
comments citing an amended record) -- deliberately sequenced first so that...`) carries enough real
narrative to arguably deserve its own record. The other nine are short, one-line judgment calls --
`a decision to actually build it`, `real adopter demand for AgDR round-tripping`, `RFC-9's own Q2` --
with nothing to gain from a dedicated record type, directory, and header schema. Building one for a
9-to-1 ratio would be overhead with no real informational gain, the same shape of mistake RFC-21
turned out to be: a reasonable-sounding abstraction that doesn't survive contact with actual volume.

Matches this project's own recurring discipline: `Parent`, `Blocked-on` itself, and `Amends` were
each added as real fields only once a pattern showed up three or more times, never speculatively.
`Blocked-on`'s current design already handles today's 10 cases correctly.

## Open questions

- **What would actually justify revisiting this?** Not specified precisely here -- "the pattern
  gets meaningfully more common" is the working bar, not a fixed count. If most `Blocked-on` values
  start looking like `MILE-38`'s (real narrative, not a one-liner) rather than the exception, that's
  the signal to reopen this.

## Non-goals

- Does not decide `Blocked-on`'s current design is permanent -- only that today's volume doesn't
  justify changing it.
- Does not propose any change to `BUG-7`'s fix or `Blocked-on`'s exclusion from
  `header.pointer-field-clean` -- both stay exactly as shipped.

## References

- BUG-7 -- the pointer-field-cleanliness fix whose `Blocked-on` exclusion prompted this idea.
- SPEC-6 -- `Blocked-on`'s documented free-text-plus-optional-reference design, unchanged by this
  RFC's rejection.
- RFC-21 -- the same shape of lesson (a plausible abstraction rejected once checked against real
  volume), the direct precedent for how this RFC reached its own conclusion.
