---
Stable-Id: 01M7V84TDVKYBVWKZV81EBNG76
Status: Planned
Phase: '1'
Track: embodiment-model
Implements: ADR-74
Blocked-on:
---
# 352 — A symbol locator tells a method from a field of the same name, so line-range pins can be retired

## What

A `path#symbol` locator names a method and a field of the same name apart (for example `Type::method` against the field `method`), so a claim about either can be pinned to its own span. Today the resolver cannot tell them apart, and the only precise pin left is a line range.

## Why

`Population::unreadable` is both a field and a method of `Population`, and `path#symbol` cannot name one without the other. `MILE-111` (`report.rs#L743-L748`) and `MILE-89` (`config.rs#L336-L376`) therefore pin a line range, which goes stale whenever lines above it move; one of them went stale while an unrelated pull request (`#609`) was in review. A line-range pin is a whole-file pin in disguise for churn, which `MILE-284` set out to reduce. `ADR-74` decides the scoped-locator grammar; a way to name a member of an impl against a field may need that grammar extended by a decision of its own, and that decision is part of this work.

## Done means

- A locator names a method and a field of one name apart, with a documented spelling, and `urzua fix --pin` resolves each to its own span.
- `MILE-111` and `MILE-89` pin the symbol they cite instead of a `#Lx-Ly` range, and no record in `docs/` pins a line range for want of a symbol that names the span.
- The resolver is exercised in this repository's own `.urzua/config.yaml` (`locator_resolvers`) and by a corpus case, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-20 | Filed as planned work: nothing built, no evidence yet. | **structural** |
