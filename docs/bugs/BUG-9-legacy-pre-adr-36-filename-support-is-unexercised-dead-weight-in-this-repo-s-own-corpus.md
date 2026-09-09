---
Stable-Id: 01M21GZ8KZEZHKHS9WWNW2YNS5
Status: Fixed
Found-in: researched live, in response to a question about whether other backward-compatibility shims exist in urzua-core for imagined future adopters rather than real ones (raised alongside RFC-23/ADR-44)
Regression-test: urzua-core::rules::tests::a_legacy_pre_type_prefix_filename_no_longer_resolves, urzua-core::rules::tests::filename_title_consistency_skips_a_legacy_shaped_filename, urzua-core::new_record::tests::next_display_number_ignores_a_legacy_pre_type_prefix_filename
---
# 9 — Legacy pre-ADR-36 filename support is unexercised dead weight in this repo's own corpus

## What was wrong

`next_display_number`, `record_id`, and `filename_title_consistency` (`rules.rs`/`new_record.rs`)
all carry a second code path reading the legacy, pre-`ADR-36` filename shape (`NNNN-slug.md`,
number as the first segment) alongside the current `TYPE-NNNN-slug.md` shape. `ADR-36` itself
explicitly expected this to matter permanently: *"docs/adr/, docs/rfc/, etc. will hold a mix of
legacy and type-prefixed filenames indefinitely"* and *"this repo's own existing 95 records are
untouched by this decision."*

That expectation didn't hold. Checked live: **zero files in this repo's own corpus use the legacy
shape today** -- every one of the original 95 (and everything since) has been renamed to the
type-prefixed form. The legacy-parsing branches in all three functions are exercising real code
paths for zero real files, kept alive only by their own test fixtures (`a_type_prefixed_filename_
resolves_the_same_as_a_legacy_one` and similar), not by any actual record in this or any other
known corpus -- there are no other known adopters yet for whom a real legacy-shaped corpus might
exist either.

## Why nothing caught it

`ADR-36`'s own reversibility/consequences reasoning was written against a real, live prediction
("will hold a mix ... indefinitely") that nothing ever re-checked once the corpus's actual state
diverged from it. No rule audits whether a documented architectural prediction is still true against
the current corpus -- the same permanent content-scope ceiling this project names elsewhere, applied
here to an ADR's own factual claim rather than a spec's.

## Fix

Decided: remove the legacy-shape support entirely -- this repo's own justification for keeping it
(a real, mixed corpus) no longer holds, and unlike header-shape parsing (`ADR-33`, tracked via
`MILE-85`), which has an explicit, real reason to stay (a first-time evaluator's *own* pre-existing
corpus, a case every adopter can hit), legacy filename numbering was always a narrower,
idiosyncratic historical convention specific to this project's own past.

`record_id`, `filename_number`, and `next_display_number` (`rules.rs`/`new_record.rs`) now
recognize only the type-prefixed `TYPE-NNNN-slug.md` shape. A reference to a legacy-shaped filename
is dangling, the same as a reference to any other nonexistent record -- tracked via ADR-36's own
amendment, not restated here.

## References

- RFC-23/ADR-44 -- the conversation that prompted this research: are there other backward-
  compatibility shims in `urzua-core` for imagined adopters rather than real ones.
- ADR-36 -- the decision whose own stated expectation ("a mix ... indefinitely") this bug found to
  no longer hold; its own amendment is the actual fix this bug tracks.
- ADR-33/MILE-85 -- the comparable, but stronger-justified, case (header-shape parsing) this bug
  distinguishes itself from.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial bug record, `Status: Open`. Not yet fixed -- whether to remove the legacy-shape support or keep it (and if kept, why, given the corpus that justified it no longer exists) is not yet decided. | **structural** |
> | 2026-09-09 | `Status: Fixed`. Legacy filename-shape acceptance removed from `record_id`/`filename_number`/`next_display_number`, per ADR-36's own amendment. | **substantive** |
