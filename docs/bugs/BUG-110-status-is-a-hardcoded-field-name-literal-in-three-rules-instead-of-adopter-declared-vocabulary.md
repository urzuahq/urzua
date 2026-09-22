---
Stable-Id: 01M34BEPVA9BX031V41XEENVZ5
Status: Open
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "not yet written -- needs a design decision first, see below"
---
# 110 — Status is a hardcoded field name literal in three rules instead of adopter-declared vocabulary

## What was wrong

`claim_status_agreement`, `pointer_target_status`, and `narrative_field_stale` all read a target
record's lifecycle field via the literal `"Status"`, baked into each rule function. `ADR-60` decided
*whether* that read should be gated on the target's type declaring the field (yes) but did not question
the literal itself — every other cross-cutting field name this engine reads is adopter-declared
(`pointer_fields`, `narrative_fields`, `required_fields`), and `BUG-59` already established the exact
precedent for this class of gap: a hardcoded per-record-type status table went permanently inert for
any corpus naming its types differently, and the fix was to make the vocabulary declared, not
compiled in.

An adopter whose corpus calls its lifecycle field `State` rather than `Status` gets all three of these
rules silently finding nothing to read, indistinguishable from a clean corpus — the same failure mode
`BUG-59` fixed for terminal-status *values*, now visible one level up, for the field *name* those values
live in.

## Why nothing caught it

`ADR-60` (this same release) fixed the adjacent, narrower question — whether to gate the existing
`"Status"` read on declaration — and that fix's own tests all use corpora that happen to name the field
`Status`, so nothing exercised a corpus that doesn't.

## What this needs before a fix

A design decision: a config-declared field name (e.g. `status_field: "State"`, defaulting to
`"Status"` so an existing config needs no change) threaded to all three call sites, replacing the
hardcoded literal — the same shape `MILE-90` already used to make `pointer_fields`/`narrative_fields`
declared instead of a hardcoded `"Blocked-on"`.

## References

- `BUG-59` — the precedent: a hardcoded per-type status *table* went inert for a differently-named
  corpus; the fix made the vocabulary declared.
- `MILE-90` — made pointer/narrative field names declared instead of a hardcoded `"Blocked-on"`, the
  same shape this bug proposes for the status field name.
- `ADR-60` — decided the adjacent, narrower question (gate the read on declaration) without deciding
  this one (make the field name itself declared).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed, not fixed. **Why:** found by a full-release code review; the fix needs a config-schema decision (a new declared key, its default, and migration for the three call sites) that shouldn't be picked under review pressure. | **substantive** |
