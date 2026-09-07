# 10 — Decide the filename convention: type prefix, zero-padding, and whether to keep slugs

> Status: Done
> Stable-Id: 01M1Y5JHCGH1AM59YY07F3YG9J
> Phase: 0
> Track: roadmap-tracking
> Implements: ADR-36

## What

Whether filenames should embed the type prefix (ADR-30-slug.md) instead of relying on directory context, whether numbers should stay zero-padded, and whether a human-readable slug belongs in the filename at all versus just the H1.

## Why

Raised directly; explicitly not decided yet -- BUG-2 already removed the actual defect (numbers already grow unbounded in both parsing and display), so this is a considered style question, not an urgent fix, and shouldn't be rushed into a corpus-wide rename.

## Blocked on

—

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Decided (ADR-36): type prefix in the filename going forward, zero-padding and slugs both kept, legacy filenames never renamed and never stop working. Also caught and fixed a second independent instance of BUG-2's defect class in filename_title_consistency's own helpers. | **substantive** |
