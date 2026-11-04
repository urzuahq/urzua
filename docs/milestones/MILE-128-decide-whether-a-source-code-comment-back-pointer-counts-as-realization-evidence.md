---
Stable-Id: 01M6CXZYE7Q8TBZQ00SXP63SKQ
Status: Planned
Phase: '0'
Track: embodiment-model
---
# 128 — Decide whether a source-code comment back-pointer counts as realization evidence

## What

Decide whether a back-pointer comment in source code (a comment naming the governing record) counts as
realization evidence for the record, in the sense `Realized-by` locators are evidence today.
`RFC-5` section 2 ("The pointer lives on the record, not embedded in application code") chose
record-side pointers and says comment-in-code is not the strongest available signal; `RFC-28`
("Why the record side alone is not enough") restates that `RFC-5` section 2 rejected in-code
`Implements:` comments as a way to discover what governs a file, and says it does not reopen that.

## Why

A real adopter relies on such comments at scale, across several hundred records, so the evidence model
either has a place for them or the adopter keeps a parallel mechanism and the replacement bar
(`MILE-102`) is not met. The prior rejections were about discovery and strength of signal, not about
whether a comment may be accepted as a claim the record already makes; that distinction is the decision
to make. If decided in favour, it needs its own ADR that narrows `RFC-5` section 2 rather than editing
it.

## Decision (2026-11-04)

A code comment citing a decision (`Implements: ADR-N`) counts as realization evidence, and a comment citing a record that does not exist is reported.

## References

- `RFC-5` -- section 2: the pointer lives on the record.
- `RFC-28` -- restates the section 2 decision and does not reopen it.
- `MILE-102` -- the adopter replacement this serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** Two accepted-or-drafted records reject in-code comments as the evidence model while a first adopter uses them at scale; filed as a decision. | **substantive** |
> | 2026-11-04 | Decision recorded: a code comment citing a decision (`Implements: ADR-N`) counts as realization evidence, and a comment citing a record that does not exist is reported. `Status` stays `Planned`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
