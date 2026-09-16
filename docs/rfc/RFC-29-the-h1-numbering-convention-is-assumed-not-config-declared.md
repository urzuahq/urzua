---
Stable-Id: 01M2MF5TZAHVVN561WK77ZG7NS
Status: Draft
Date: 2026-09-16
Author: beauwilliams
---
# 29 — The H1 numbering convention is assumed, not config-declared

## Summary

`filename.title-consistency` requires a record's H1 to open with a number matching its filename. That
is `ADR-36`'s convention, decided for this repo. No config knob declares it, so an adopter whose
corpus titles records any other way gets a blocking `Error` on every record, with a blanket waiver as
the only escape. Propose making it declarable per record type, as `ADR-44` did for relationship
fields.

## Motivation

The per-type config surface is `dir`, `prefix`, `required_fields`, `known_fields`, `header_shape`,
`spec`, `pointer_fields`, `narrative_fields`. Nothing there concerns titles.

Two escape hatches exist and neither is sufficient. A filename without the type prefix is skipped
entirely by the `BUG-9`/`ADR-36` legacy guard — verified: a Nygard-style `0001-record-architecture-
decisions.md` is not examined at all. But a corpus that *does* adopt the type prefix while numbering
records somewhere other than the H1 — in a header field, or not at all — hits the rule on every
record. There is no way to say "my titles are not numbered."

This is the third instance of one class, and the first two are already settled or in flight:

| | Assumption hardcoded in Rust | Status |
|---|---|---|
| BUG-8 | pointer/narrative field names | Fixed — `ADR-44`/`MILE-90` made them declared |
| RFC-28 | `is_terminal_status`'s per-type status vocabulary | Draft |
| this | the H1 numbering convention | — |

`AGENTS.md` states the test this fails: anything that only works because of a hardcoded, type-specific
assumption in Rust source is a regression against the founding claim, not a shortcut.

## Proposal

A per-type declaration of where a record's number lives, defaulting to today's behaviour so this repo
is unaffected. Sketch, not settled:

```toml
[record_types.adr]
number_in = "h1"     # "h1" | "none"
```

`none` skips the rule for that type rather than waiving it — a waiver records an exception to a rule
that applies, and this is a rule that does not apply.

`BUG-32` is entangled: if the convention becomes declarable, the extractor's contract (what counts as
a number in a leading token) is part of what gets declared, and the two should be decided together.

## Open questions

- Is `h1 | none` enough, or does a corpus that numbers records in a header field need `number_in =
  "field:Number"`? No evidence either way yet — inventing the third case now would be speculative.
- Should `none` also disable `filename_number` parsing, or only the title comparison?
- `RFC-28` proposes a config-declared status vocabulary. Both are "the tool decides what the repo
  should declare." Is there one mechanism here rather than two, and does deciding them separately
  produce two near-identical config shapes?
- What does `urzua init` infer when adopting an existing corpus? Detecting the convention is
  attractive and contradicts `RFC-10`'s "declared, never sniffed."

## References

- ADR-36 -- the filename and numbering scheme this rule enforces.
- ADR-44, MILE-90, BUG-8 -- the same class, resolved by declaration.
- RFC-28 -- the sibling still in flight; likely decided together.
- BUG-23, BUG-30, BUG-32 -- defects in this rule found while working against foreign-corpus shapes.
- SPEC-3 -- the config schema any new key lands in.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Added measured evidence from `MILE-51`'s first run: against `npryce/adr-tools`, `filename.title-consistency` examines **zero** records, and `urzua new` writes a colliding `ADR-1-…` beside an existing `0001-…` (`BUG-37`). The motivation was previously reasoned from this repo's config surface alone; it now has a foreign corpus behind it, and the numbering half turns out to be a live defect rather than a latent gap. | **substantive** |
> | 2026-09-16 | Initial proposal, `Status: Draft`. **Why:** surfaced by an engine-principle review of the `BUG-23` fix -- the message defects were worth fixing on their own, but none of them closes the gap that the convention itself is not declarable, and leaving that unfiled would have let a message fix read as resolving it. | **structural** |
