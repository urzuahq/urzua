---
Stable-Id: 01M21GZ8KZEZHKHS9WWNW2YNS5
Status: Open
Found-in: researched live, in response to a question about whether other backward-compatibility shims exist in urzua-core for imagined future adopters rather than real ones (raised alongside RFC-23/ADR-44)
Regression-test: not yet written -- fix scope (remove the legacy-shape branches vs. keep and just document the divergence) not yet decided
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

## What's still undecided

Whether the right fix is removing the legacy-shape support entirely (this repo's own justification
for keeping it no longer holds) or keeping it -- unlike header-shape parsing (`ADR-33`, tracked via
`MILE-85`), which has an explicit, real reason to stay (a first-time evaluator's *own* pre-existing
corpus, a case every adopter can hit), legacy filename numbering is a much narrower, more
idiosyncratic historical convention specific to this project's own past -- a genuinely weaker case
for external relevance than header shapes are. Not decided here.

## References

- RFC-23/ADR-44 -- the conversation that prompted this research: are there other backward-
  compatibility shims in `urzua-core` for imagined adopters rather than real ones.
- ADR-36 -- the decision whose own stated expectation ("a mix ... indefinitely") this bug found to
  no longer hold.
- ADR-33/MILE-85 -- the comparable, but stronger-justified, case (header-shape parsing) this bug
  distinguishes itself from.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial bug record, `Status: Open`. Not yet fixed -- whether to remove the legacy-shape support or keep it (and if kept, why, given the corpus that justified it no longer exists) is not yet decided. | **structural** |
