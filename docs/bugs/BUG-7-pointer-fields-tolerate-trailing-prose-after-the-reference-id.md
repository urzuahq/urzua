---
Stable-Id: 01M2165QW5JTT3P8PJ74Y9M4N1
Status: Open
Found-in: discussed live while designing RFC-21's structured References field -- asked whether References should tolerate trailing prose the way Parent already does, which surfaced that Parent's tolerance was itself never a deliberate design decision
Regression-test: urzua-core::header::tests -- a planned test asserting extract_references (or its successor) rejects/does not silently accept a Parent/Implements/Derives-from value with trailing prose after the leading reference token; not yet written
---
# 7 — Pointer fields tolerate trailing prose after the reference ID

## What was wrong

`extract_references` (used by `pointer.resolution` and `urzua graph`) reads only the leading
`PREFIX-N` token from each comma-separated entry in a pointer field, silently ignoring everything
after it. This means a pointer field's own raw value can legally contain arbitrary explanatory
prose mixed into what's supposed to be structured data -- real, live examples exist in this exact
corpus: `SPEC-2`'s `Parent: SPEC-1 (v0 CLI). Cross-cutting rules — no-silent-no-op, the permanent...`
and `SPEC-4`'s `Parent: SPEC-1 (v0 CLI), which lists the bug classes this suite must reproduce.`
Neither is a data-entry mistake -- the tool accepts and resolves both without complaint, meaning it
was never actually decided that pointer fields should stay clean; it was just never prevented.

This is ambiguous for whoever writes the field next, human or agent: nothing signals whether the
correct convention is "the ID alone" or "the ID plus context," so both keep happening.

## Why nothing caught it

No rule or test ever asserted a pointer field's value contains *only* a reference, because
`extract_references`'s whole design already tolerates -- and was written to tolerate -- exactly this
shape (it exists specifically to pull a clean ID out of a field that might have trailing text, per
its own doc comment). The tolerance was a deliberate parsing choice for reading an *existing*
corpus's real conventions leniently, never audited for whether new pointer fields should keep
allowing it.

## References

- RFC-21 -- the structured `References` field proposal whose design discussion surfaced this,
  since rejected (both of its own motivating cases turned out to be fixable with existing fields
  like `Implements`, not a new one). Whatever fix this bug gets should tighten `extract_references`
  itself, or the existing `Parent`/`Implements`/`Derives-from` fields directly -- not wait on a new
  field that isn't being built.
- `SPEC-2`/`SPEC-4` -- the two live instances of the pattern this bug describes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial bug record, `Status: Open`. Not yet fixed -- fix scope (tighten the parser to reject trailing prose, and/or clean up the two existing instances) still to be decided. | **structural** |
