---
Stable-Id: 01M214TS10Y1D48CT4N9B9PXKA
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: —
---
# 86 — Detect a record cited in prose with no matching structured pointer

## What

A `check` finding (or `doctor` check) for the case where a record's free-text `## References`
section cites another record using clearly load-bearing language ("implements," "decided by,"
"folds in," "supersedes") but the same relationship has no matching structured header pointer
(`Implements`/`Derives-from`/`Parent`) that `pointer.resolution` actually resolves and `urzua graph`
actually surfaces.

Not a proposal to make `References` itself structured — most of what lives there is legitimately
loose (evidentiary/contextual citations with no declared relationship type at all, e.g. "the rule
whose live finding this spec closes"), and forcing all of it into a fixed vocabulary of pointer
fields would either require inventing many more field types or collapse distinct reasons for citing
something into one generic bucket. The gap is narrower: when a citation *is* actually a checkable
relationship, prose currently offers no signal that it should have been a header field instead.

**Matching must check both endpoints, not just the citing record.** The real case below (`RFC-18`
citing `MILE-6`) resolved on `MILE-6`'s own `Implements` field, not on anything added to `RFC-18` —
a detector that only inspected the record doing the citing would have kept flagging this as unfixed
even after it was. Any real implementation needs to check whether *either* record in a cited pair
already carries a structured pointer resolving to the other, in whatever field is valid for that
record's type, before treating the citation as unaddressed.

## Why

Found live, twice in the same session: `SPEC-16`/`SPEC-17` cited RFC-1 in their own `## References`
sections for a while before gaining a real `Derives-from`/`Implements` header pointer; then RFC-18's
`## References` said "MILE-6 — ... this proposal folds in" with no reciprocal structured pointer on
either record until asked about directly. Neither case was caught by anything — `pointer.resolution`
only scans structured fields, and this project has explicitly declined to parse body prose for
structural claims elsewhere (SPEC-1's permanent content-scope ceiling). Both times, a human had to
notice the mismatch by reading the prose closely.

The real difficulty, not glossed over: distinguishing "this citation should be a header pointer"
from "this citation is legitimately loose" is itself a content-quality judgment, not something a
keyword scan can decide reliably (a heuristic like "contains 'implements' or 'decided by'" would
both over-fire on prose that's genuinely just explaining context and under-fire on relationships
phrased less mechanically). Whether this is buildable at all — and if so, as a hard rule or a
`doctor`-style advisory nudge — is exactly what's undecided here.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial milestone. **Why:** found live, twice this session (SPEC-16/17 and RFC-18), that a load-bearing citation living only in `## References` prose has no mechanism to flag it as a candidate for a real structured pointer instead. | **structural** |
> | 2026-09-08 | Considered and rejected a new generic `References` header field as the fix (RFC-21) -- both motivating cases turned out to be fixable with *existing* fields (`Implements: ADR-10` on SPEC-16/17; `Implements: RFC-18` added to MILE-6 itself), confirmed live in `urzua graph`. Restores this milestone's original scope: detection, pointing at an existing field, not new schema. Also surfaced `BUG-7` (pointer fields tolerating trailing prose), a real but separate defect. | **substantive** |
