---
Stable-Id: 01M83SMV27CX5Q87DVCV1D9ZY9
Status: Open
Found-in: "The independent review of the pull request that adds `examined`, where `urzua fix --apply` set `MILE-358` to `Done` with Done means item 1 (a decision recorded in an ADR) resting on `spec:` evidence naming `ADR-106` at `Status: Proposed`."
Realized-by:
---
# 217 — A spec locator to a Proposed ADR resolves a Done means item, so fix --apply sets Done on an undecided decision

## What was wrong

A Done means item resolves on any locator whose target exists. A `spec:` locator naming an ADR still at `Proposed` (or `Draft`) therefore resolves an item whose text says a decision is recorded, and `urzua fix --apply` computes and writes `Done`. `urzua check` then reports nothing: the milestone is `Done`, every item has evidence, and the one decision it rests on has not been made. A milestone's `Done` can assert a decision the maintainer has not taken, which is the maintainer's call under `authority`.

## Why nothing caught it

Evidence resolution checks that a locator's target exists and, for code and tests, that its pin matches; it does not read the status of a record a locator names. No rule compares a cited decision record's status with the item it is offered for, and no configuration key lets a repository declare that a record cited as evidence must have reached a deciding status. Deciding which statuses count, and whether this is per type in configuration or per item, is a design call, so the bug is filed open rather than fixed here.

## References

- `MILE-358`, set back to `InProgress` in the pull request that adds `examined`, with item 1's evidence removed until `ADR-106` is decided.
- `ADR-101`: nothing is `Done` without evidence.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
