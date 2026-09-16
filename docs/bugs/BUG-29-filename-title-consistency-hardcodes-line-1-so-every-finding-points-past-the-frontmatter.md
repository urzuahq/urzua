---
Stable-Id: 01M2MF5RYBQG72GMM6FE709HEP
Status: Fixed
Found-in: 'building a synthetic corpus to reproduce BUG-23 -- every finding came back `line: 1` while the H1 sat on line 7'
Regression-test: 'rust/crates/urzua-core/src/rules.rs :: filename_title_consistency_reports_the_real_h1_line_observed_failing'
Realized-by: code:rust/crates/urzua-core/src/rules.rs
---
# 29 — filename.title-consistency hardcodes line 1, so every finding points past the frontmatter

## What was wrong

Both `Finding` sites in `filename_title_consistency` wrote `line: Some(1)` as a literal. Every record
in this corpus opens with YAML frontmatter, so an H1 is never on line 1 — `ADR-36`'s sits on line 11.
Every finding this rule emitted pointed an editor at the `---` opening the header instead of the
title it was complaining about.

The information was available and discarded: `title_number` located the heading with
`.lines().find(..)` and kept only the digits, dropping the index.

Reproduced against a three-record synthetic corpus before the fix — all three findings reported
`line: 1` against H1s on line 7.

## Why nothing caught it

No test asserted the `line` field of a `filename.title-consistency` finding; the rule's existing
tests assert on message text only. And this corpus produces **zero** findings from this rule — every
record's filename and H1 agree — so the wrong value was never rendered anywhere a reader would see
it. A field that is only wrong on a path nothing exercises is invisible to both the test suite and
review.

## The fix

`first_h1` returns the line it matched, and both findings carry it. The no-H1 case carries `line:
None` rather than a fabricated position, since there is no title to point at.

## References

- `rust/crates/urzua-core/src/rules.rs` -- `first_h1` and `filename_title_consistency`.
- BUG-23 -- fixed in the same change; this defect was found while reproducing it.
- BUG-30 -- the other defect in the same function, same change.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Filed and fixed in the same change. **Why:** found by reproducing `BUG-23` against a synthetic corpus rather than reasoning about the code -- the wrong line number is invisible in this repo, where the rule never fires, and only appears once a corpus actually trips it. | **substantive** |
