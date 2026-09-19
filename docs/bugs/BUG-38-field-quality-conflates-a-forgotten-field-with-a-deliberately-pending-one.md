---
Stable-Id: 01M2Q5574E1HVXH4TNDGXP8YT5
Status: Fixed
Found-in: 'MILE-80 -- reviewing its own recorded consequences before merge; the flattening was written down as intended and `Pending`''s definition contradicts that'
Regression-test: 'rust/crates/urzua-core/src/rules.rs::a_pending_field_belongs_to_field_pending_not_field_quality -- a Pending field belongs to field.pending, a Blank one stays with field.quality, and neither claims the other'
---
# 38 — `field.quality` conflates a forgotten field with a deliberately pending one

Introduced by `MILE-80`.

## What

`field.quality` (`rules.rs`) maps four field states onto two severities:

| state | meaning | severity |
|---|---|---|
| `Blank` | field absent or empty | Error |
| `Placeholder` | unedited template text (`TBD`, `—`) | Error |
| `Pending` | *"explicitly marks work not yet done"* | Warning |
| `Present` | a real value | none |

`Blank` and `Placeholder` mean **someone forgot**. `Pending` means **someone decided** -- it is a
declaration, not an omission. Those are two different questions sharing one rule id.

## Why it matters now

Before `MILE-80` the two severities were hardcoded, so the distinction survived by accident. `MILE-80`
made a rule's level a repository's declaration, and **one rule has one level**, so the grading is
flattened. There is now no setting that is correct:

- `field.quality: error` -- a deliberate `Pending` marker blocks CI
- `field.quality: warn` -- a genuinely blank required field stops blocking

This is `pointer.resolution`'s defect at smaller scale. That one emitted 172 of 213 findings and was
split in `MILE-80`; this one was noted as an accepted consequence in the same change and should not
have been. Flattening a grading is fine when the grading is one question's degrees. It is not fine
when the two arms answer different questions, and `Pending`'s own doc comment says it answers a
different one.

## Fix (shipped)

The same split. `field.quality` keeps `Blank`/`Placeholder` -- *is this required field filled in* --
and the `Pending` arm becomes its own rule id, opt-in like everything else, for a repository that
wants deliberately-unfinished work reported. A repository that does not declare it is not told about
its own `Pending` markers, which is the correct default: it already knows, having written them.

## How this was found

Reviewing `MILE-80`'s own consequences before merge, prompted by asking what follow-up `field.quality`
needed. The flattening was already written down there as intended; reading what `Pending` actually
means is what changed the verdict.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Filed. **Why:** `MILE-80` recorded this flattening as an accepted consequence. It is not acceptable -- `Pending` is a declaration and `Blank` is an omission, so no single declared level is correct for both. | **substantive** |
> | 2026-09-17 | `Status: Open` → `Fixed` in the same change that filed it. **Why:** this record's own `Regression-test` value is a `Pending` marker, so filing it made `urzua check docs/` exit non-zero on this repo -- the bug reproduced itself in the record describing it, which settled that it was not deferrable. `field.pending` split out; `field.quality` keeps `Blank`/`Placeholder`. | **substantive** |
> | 2026-09-19 | `Regression-test` now names the test that exists. **Why:** the field still read *"not yet written"* after the test was written and observed failing, so this record claimed the work was undone while the work was done. One of eight such records, found by the audit that filed `BUG-57`. | **structural** |
