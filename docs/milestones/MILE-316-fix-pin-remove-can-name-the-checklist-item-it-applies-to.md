---
Stable-Id: 01M7K8M59D7E0JR35VCBF4SD3X
Status: Planned
Phase: '1'
Track: write-path
Blocked-on: null
---
# 316 — fix --pin --remove can name the checklist item it applies to

## What

Let `urzua fix --pin --remove` and `--replace` take the item a locator belongs to, and refuse an ambiguous removal instead of applying it to every item.

## Why

On 2026-11-16 a builder needed to remove one checklist item's evidence for a file that two other items also cite. `fix --pin --remove` removed the locator from the other two items and silently left the intended one, whose only locator it was; a second call refused it. The builder restored the two items by hand and deleted the third entry by hand, which is the kind of edit the evidence guards exist to make unnecessary.

## Done means

- `fix --pin --remove` and `--replace` accept a selector for the item, and with it change only that item, shown by a test with one locator cited by three items.
- Without the selector, a locator cited by more than one item is refused with a `next_action` listing the items.
- Removing an item's only locator is allowed when the item is waived (suppressed, in the neutral naming), since such an item carries no evidence, shown by a test.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Met on 2026-11-16 by a builder using the tool on this repository's own records; the maintainer said that day that friction met using the tool is an improvement point. | **structural** |
