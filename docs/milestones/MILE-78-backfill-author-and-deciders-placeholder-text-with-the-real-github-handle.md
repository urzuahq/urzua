# 78 — Backfill Author and Deciders placeholder text with the real GitHub handle

> Status: Done
> Stable-Id: 01M1Z1EP2A91E5255WDRGQPYGD
> Phase: 0
> Track: accountability-identity
> Implements: —

## What

Replace every anonymized `Author`/`Deciders` placeholder (the literal text this project used before
identity was resolved) across the corpus with the real GitHub handle, `@beauwilliams` -- 92
instances across ADRs, RFCs, and specs. Distinct from MILE-18 (real identity *resolution* as a
mechanism, e.g. via `fix --apply`'s existing identity-resolution path for Reviewer/Decider roles):
this is a corpus data fix with the real value known today, not a tooling build. `urzua new` already
resolves a real identity for newly created records (ADR-27) -- this backfills everything authored
before that, plus one live regression (ADR-38, written this session, where the placeholder was
typed by hand over the tool's own auto-filled value).

## Why

Found live: reviewing ADR-38, the anonymized placeholder appeared in `Author`/`Deciders` again
despite `urzua new` already resolving the real identity automatically -- a hand-edit had silently
overwritten it. Checking the rest of the corpus found the same placeholder in 92 places going back
to the first RFC. A real decision record's `Author`/`Deciders` accountability is the entire point of
requiring the field (RFC-1) -- an anonymized placeholder in every single one defeats that as
thoroughly as an unfilled blank would, just less visibly, since `field.quality` doesn't flag
placeholder-shaped prose that happens to already be non-empty.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Fixed and shipped: mechanically replaced all 92 placeholder instances (`Author`/`Deciders`) with `@beauwilliams` across every ADR, RFC, and spec. **Why:** the placeholder defeated the actual purpose of the `Author`/`Deciders` fields (RFC-1's accountability requirement) in every single record that had one; the real value has been known the whole time, so leaving it unfixed was pure debt, not an open question. | **substantive** |
