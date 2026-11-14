---
Stable-Id: 01M7CAZQKGYF9A90V9MZ27KC42
Status: Fixed
Found-in: 'an external review of urzua, item 1.1 (2026-11-14)'
Realized-by: test:rust/crates/urzua-cli/tests/fix_missing_locator_integration.rs, code:rust/crates/urzua-core/src/fix.rs, code:rust/crates/urzua-core/src/rules/status.rs
---
# 196 — fix computes an Embodiment from a Realized-by locator to a file that does not exist and --apply writes it

## What was wrong

`urzua fix` computed a record's `Embodiment` and delivery `Status` from every `Realized-by` locator, whether or not the file existed. A record stating `Embodiment: Not started` that cited `test:tests/missing.rs` computed to `Verified`, and `fix --apply` wrote it (a milestone cited `code:src/missing.rs` was likewise set to `Done`). `embodiment.locator-exists` reported the missing file, but the claim read as stronger than `Not started` and the repair wrote it into the record with an attested revision row.

## Why nothing caught it

Every `fix` fixture cited files it never created, so the tests only ever exercised the case where the cited file did not matter; nothing asserted that a locator to a missing file is not evidence.

## References

Fixed in `fix`'s Embodiment repair and in the delivery judgement (`status.delivery-agreement`): only locators that pass the test `embodiment.locator-exists` applies (tracked, present, a scope that resolves) count, and a repair that lowers a claim names the locators it did not count. `SPEC-8` documents it. The regression tests are in `fix_missing_locator_integration.rs`, and the acceptance case `fix-does-not-repair-an-embodiment-from-a-locator-to-a-missing-file` tags this bug.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-14 | Filed and fixed in one pass. **Why:** an external review reproduced `fix --apply` writing `Verified` from a locator to a file that did not exist; the fix is in the same pull request. | **substantive** |
