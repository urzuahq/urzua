---
Status: Done
Stable-Id: 01M1Y5JHCGH1AM59YY07F3YG9J
Phase: '0'
Track: roadmap-tracking
Implements: ADR-36
Blocked-on: —
Realized-by: spec:docs/adr/ADR-36-filenames-carry-the-type-prefix-going-forward.md
---
# 10 — Decide the filename convention: type prefix, zero-padding, and whether to keep slugs

## What

Whether filenames should embed the type prefix (ADR-30-slug.md) instead of relying on directory context, whether numbers should stay zero-padded, and whether a human-readable slug belongs in the filename at all versus just the H1.

## Why

Raised directly; explicitly not decided yet -- BUG-2 already removed the actual defect (numbers already grow unbounded in both parsing and display), so this is a considered style question, not an urgent fix, and shouldn't be rushed into a corpus-wide rename.

## Done means

`ADR-36` is Accepted and states the filename convention (the type prefix in the filename going forward, zero-padding and slugs both kept, legacy filenames never renamed and never stopping working); a legacy-shaped filename is skipped rather than reported by `filename_title_consistency` (`filename_title_consistency_skips_a_legacy_shaped_filename`), and the helpers that rule uses no longer assume a fixed number width (the second instance of the `BUG-2` defect class).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Decided (ADR-36): type prefix in the filename going forward, zero-padding and slugs both kept, legacy filenames never renamed and never stop working. Also caught and fixed a second independent instance of BUG-2's defect class in filename_title_consistency's own helpers. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
