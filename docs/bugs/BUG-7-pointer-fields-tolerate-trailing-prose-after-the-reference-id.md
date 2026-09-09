---
Stable-Id: 01M2165QW5JTT3P8PJ74Y9M4N1
Status: Fixed
Found-in: discussed live while designing RFC-21's structured References field -- asked whether References should tolerate trailing prose the way Parent already does, which surfaced that Parent's tolerance was itself never a deliberate design decision; a further audit found Derives-from's "(Status)" annotation was the same anti-pattern, applied consistently across 33 files
Regression-test: urzua-core::rules::tests::a_derives_from_status_annotation_is_flagged_observed_failing, a_parent_field_with_freeform_trailing_prose_is_flagged, a_clean_bare_reference_is_not_flagged, multiple_clean_references_are_not_flagged, blocked_on_is_excluded_even_with_trailing_prose
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

## Fix

Not a parser change: `extract_references`/`pointer_resolution` are shared with `Blocked-on`, which
*legitimately* mixes free text with an embedded reference (SPEC-6 -- "a milestone can exist before a
decision does"); a real live case (`MILE-38`'s own `Blocked-on`) needs exactly that tolerance, so
tightening the shared extractor would have broken it. Instead: a new, additive rule,
`header.pointer-field-clean`, checks `Implements`/`Derives-from`/`Parent` specifically (never
`Blocked-on`) -- a `Warning` for any comma-separated entry that isn't *exactly* a clean reference
token (or the `—` no-value placeholder) once trimmed. `pointer_resolution` itself is untouched, so
no existing resolution behavior changed.

Proven live before any cleanup: the new rule fired 63 times across 50 files on the real corpus.
Scope turned out much wider than the original two instances -- `Derives-from`'s `(Status)`
annotation (e.g. `RFC-1 (Accepted)`) was a consistent, corpus-wide habit across 33 ADRs/RFCs/specs,
not two isolated mistakes, and is additionally redundant with `pointer.resolution`'s own live status
report plus a staleness risk (nothing re-verifies a hand-typed status stays accurate). `Parent`'s
`(v0 CLI)` annotation appeared on all 17 specs with a `Parent` field, 15 of them pure boilerplate
(stripped to a bare `Parent: SPEC-1`) and 2 (`SPEC-2`, `SPEC-4`) carrying real explanatory content,
folded into their own `## Purpose` sections instead of staying in the header. `SPEC-2` additionally
had orphaned blockquote-continuation lines sitting after its H1 -- leftover from before the
yaml-frontmatter migration, never actually part of the parsed header even in the original file --
cleaned up in the same pass.

All 63 findings resolved; `header.pointer-field-clean` is silent on the real corpus after the fix.
`urzua graph` confirmed zero dangling edges post-cleanup -- every real reference still resolves.

## References

- RFC-21 -- the structured `References` field proposal whose design discussion surfaced this,
  since rejected (both of its own motivating cases turned out to be fixable with existing fields
  like `Implements`, not a new one).
- ADR-32 -- git-blame drift detection, cited as the reason `pointer.resolution`'s live status
  report is the correct source of truth, not a hand-typed annotation.
- SPEC-6 -- `Blocked-on`'s documented free-text-plus-optional-reference shape, the reason this
  fix couldn't be a blanket change to `extract_references`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial bug record, `Status: Open`. Not yet fixed -- fix scope (tighten the parser to reject trailing prose, and/or clean up the two existing instances) still to be decided. | **structural** |
> | 2026-09-08 | `Status: Fixed`. Built `header.pointer-field-clean` (additive, `Implements`/`Derives-from`/`Parent` only, `Blocked-on` deliberately excluded); cleaned all 50 real corpus files (33 `Derives-from`, 17 `Parent`), 2 of which (`Parent`) had real content moved into `## Purpose`. Verified: the new rule fires 63 findings before the fix, zero after -- more than 50 because several files carry more than one dirty comma-separated entry in a single field (e.g. `Derives-from: RFC-8 (Accepted), ADR-15 (Accepted), ADR-19 (Accepted)` is 1 file but 3 findings); `urzua graph` shows zero dangling edges post-cleanup. | **substantive** |
